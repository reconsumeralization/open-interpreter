use crate::client_common::Prompt;
use crate::harness::kimi_cli;
use codex_chat_wire_compat::ToolKinds;
use codex_chat_wire_compat::ToolOutputKind;
use codex_protocol::openai_models::ModelInfo;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;

const KIMI_CODE_SYSTEM_PROMPT: &str = include_str!("kimi_code_system_prompt.md");
const KIMI_CODE_TOOLS: &str = include_str!("kimi_code_tools.json");
const KIMI_CODE_AUTO_PERMISSION_REMINDER: &str = "<system-reminder>\nAuto permission mode is active. Tool approvals will be handled automatically while this mode remains enabled.\n  - Continue normally without pausing for approval prompts.\n  - Do NOT call AskUserQuestion while auto mode is active. Make a reasonable decision and continue without asking the user.\n</system-reminder>";
const KIMI_CODE_BUILTIN_SKILLS: &str = "DISREGARD any earlier skill listings. Current available skills:\n### Built-in\n- update-config: Inspect or edit kimi-code's own config — `config.toml` (model, provider, permission, hooks) and `tui.toml` (theme, editor, notifications, auto-update). Use when the user asks what a setting does or wants to change one.\n  Path: builtin://update-config";
static KIMI_CODE_SYSTEM_PROMPT_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn build_request(
    prompt: &Prompt,
    model_info: &ModelInfo,
    conversation_id: &str,
) -> Result<(Value, ToolKinds), serde_json::Error> {
    let mut messages = vec![json!({
        "role": "system",
        "content": cached_system_prompt(prompt, conversation_id),
    })];
    messages.extend(add_auto_permission_reminders(
        kimi_cli::build_messages_with_options(
            prompt.get_formatted_input(),
            kimi_cli::MessageBuildOptions::kimi_code(),
        )?
        .collect(),
    ));
    let tools = build_tools();
    let tool_kinds = tools
        .iter()
        .filter_map(|tool| {
            tool.get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .map(|name| (name.to_string(), ToolOutputKind::Function))
        })
        .collect();

    Ok((
        json!({
            "model": model_info.slug,
            "messages": messages,
            "max_completion_tokens": 32768,
            "prompt_cache_key": kimi_code_prompt_cache_key(conversation_id),
            "stream": true,
            "stream_options": {
                "include_usage": true,
            },
            "tools": tools,
            "thinking": {
                "type": "enabled",
            },
            "reasoning_effort": "high",
        }),
        tool_kinds,
    ))
}

fn kimi_code_prompt_cache_key(conversation_id: &str) -> String {
    format!("session_{conversation_id}")
}

fn cached_system_prompt(prompt: &Prompt, conversation_id: &str) -> String {
    let cwd = prompt.cwd.as_deref().unwrap_or_else(|| Path::new("."));
    let skills = session_skills_listing(prompt);
    let mut skills_hasher = DefaultHasher::new();
    skills.hash(&mut skills_hasher);
    let key = format!(
        "{conversation_id}:{}:{:016x}",
        cwd.display(),
        skills_hasher.finish()
    );
    let mut cache = KIMI_CODE_SYSTEM_PROMPT_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(key)
        .or_insert_with(|| build_system_prompt(prompt, &skills))
        .clone()
}

/// Renders Kimi Code's source-backed model-facing skill listing.
fn session_skills_listing(_prompt: &Prompt) -> String {
    KIMI_CODE_BUILTIN_SKILLS.to_string()
}

fn add_auto_permission_reminders(messages: Vec<Value>) -> Vec<Value> {
    let mut with_reminders = Vec::with_capacity(messages.len() + 1);
    for message in messages {
        let is_user = message
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role == "user");
        let already_reminder = message
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|content| content == KIMI_CODE_AUTO_PERMISSION_REMINDER);
        with_reminders.push(message);
        if is_user && !already_reminder {
            with_reminders.push(json!({
                "role": "user",
                "content": KIMI_CODE_AUTO_PERMISSION_REMINDER,
            }));
        }
    }
    with_reminders
}

fn build_system_prompt(prompt: &Prompt, skills: &str) -> String {
    let cwd = prompt.cwd.as_deref().unwrap_or_else(|| Path::new("."));
    let listing = kimi_work_dir_listing(cwd);
    KIMI_CODE_SYSTEM_PROMPT
        .replace(
            "{% if KIMI_OS == \"Windows\" %}\n\nIMPORTANT: You are on Windows. The Bash tool runs through Git Bash, so use Unix shell syntax inside Bash commands — `/dev/null` not `NUL`, and forward slashes in paths. For file operations, always prefer the built-in tools (Read, Write, Edit, Glob, Grep) over Bash commands — they work reliably across all platforms.\n{% endif %}",
            "",
        )
        .replace(
            "{% if KIMI_ADDITIONAL_DIRS_INFO %}\n\n## Additional Directories\n\nThe following directories have been added to the workspace. You can read, write, search, and glob files in these directories as part of your workspace scope.\n\n{{ KIMI_ADDITIONAL_DIRS_INFO }}\n{% endif %}",
            "",
        )
        .replace("{{ ROLE_ADDITIONAL }}", "")
        .replace("{{ KIMI_OS }}", kimi_os_label())
        .replace("{{ KIMI_SHELL }}", "bash (`/bin/bash`)")
        .replace("{{ KIMI_NOW }}", &kimi_now())
        .replace("{{ KIMI_WORK_DIR }}", cwd.display().to_string().as_str())
        .replace("{{ KIMI_WORK_DIR_LS }}", &listing)
        .replace("{{ KIMI_ADDITIONAL_DIRS_INFO }}", "")
        .replace("{{ KIMI_AGENTS_MD }}", "")
        .replace("{{ KIMI_SKILLS }}", skills)
}

fn kimi_os_label() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
}

fn kimi_now() -> String {
    if let Ok(fake_time) = std::env::var("HARNESS_LAB_FAKE_TIME") {
        if let Some((date, time)) = fake_time.split_once(' ') {
            return format!("{date}T{time}.000Z");
        }
        return fake_time;
    }
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn kimi_work_dir_listing(cwd: &Path) -> String {
    let entries = match sorted_dir_entries(cwd) {
        Ok(entries) => entries,
        Err(_) => return String::new(),
    };
    let mut lines = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let last = index + 1 == entries.len();
        let branch = if last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() {
            lines.push(format!("{branch}{name}/"));
            if !name.starts_with('.') {
                append_child_listing(&mut lines, &entry.path(), last);
            }
        } else {
            lines.push(format!("{branch}{name}"));
        }
    }
    lines.join("\n")
}

fn append_child_listing(lines: &mut Vec<String>, dir: &Path, parent_last: bool) {
    let Ok(children) = sorted_dir_entries(dir) else {
        return;
    };
    let prefix = if parent_last { "    " } else { "│   " };
    for (index, child) in children.iter().take(20).enumerate() {
        let child_last = index + 1 == children.len().min(20);
        let branch = if child_last {
            "└── "
        } else {
            "├── "
        };
        let mut name = child.file_name().to_string_lossy().to_string();
        if child.path().is_dir() {
            name.push('/');
        }
        lines.push(format!("{prefix}{branch}{name}"));
    }
    if children.len() > 20 {
        lines.push(format!(
            "{prefix}└── ... and {} more",
            children.len().saturating_sub(20)
        ));
    }
}

fn sorted_dir_entries(cwd: &Path) -> std::io::Result<Vec<std::fs::DirEntry>> {
    let entries = std::fs::read_dir(cwd)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        if entry.path().is_dir() {
            dirs.push(entry);
        } else {
            files.push(entry);
        }
    }
    sort_kimi_entries(&mut dirs);
    sort_kimi_entries(&mut files);
    dirs.extend(files);
    Ok(dirs)
}

fn sort_kimi_entries(entries: &mut [std::fs::DirEntry]) {
    entries.sort_by(|left, right| {
        let left_name = left.file_name().to_string_lossy().to_string();
        let right_name = right.file_name().to_string_lossy().to_string();
        left_name
            .to_lowercase()
            .cmp(&right_name.to_lowercase())
            .then_with(|| left_name.cmp(&right_name))
    });
}

fn build_tools() -> Vec<Value> {
    serde_json::from_str(KIMI_CODE_TOOLS).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::build_request;
    use crate::client_common::Prompt;
    use codex_protocol::models::ContentItem;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::models::ResponseItem;
    use codex_protocol::openai_models::ModelInfo;
    use serde_json::json;

    #[test]
    fn kimi_code_request_renders_kimi_code_builtin_skills() {
        let prompt = Prompt {
            input: vec![ResponseItem::Message {
                id: Some(codex_protocol::ResponseItemId::from_server(
                    "user".to_string(),
                )),
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Run the QA pass".to_string(),
                }],
                phase: None,

                internal_chat_message_metadata_passthrough: None,
            }],
            cwd: Some(std::env::temp_dir()),
            ..Prompt::default()
        };

        let (request, _) = build_request(
            &prompt,
            &test_model_info(),
            "kimi-code-session-skills-conversation",
        )
        .expect("build request");

        let system = request["messages"][0]["content"]
            .as_str()
            .expect("system content");
        assert!(system.contains("DISREGARD any earlier skill listings"));
        assert!(system.contains("- update-config: Inspect or edit kimi-code's own config"));
        assert!(system.contains("Path: builtin://update-config"));
        assert!(!system.contains("{{ KIMI_SKILLS }}"));
        assert!(!system.contains("<skills_instructions>"));
    }

    #[test]
    fn kimi_code_request_does_not_render_codex_session_skills() {
        let prompt = Prompt {
            input: vec![
                ResponseItem::Message {
                    id: Some(codex_protocol::ResponseItemId::from_server(
                        "developer".to_string(),
                    )),
                    role: "developer".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "<skills_instructions>\n## Skills\nA skill is a set of local instructions to follow that is stored in a `SKILL.md` file.\n### Available skills\n- qa-testing: Run the project's QA test plan against a live build (file: /home/user/skills/.system/qa-testing/SKILL.md)\n### How to use skills\n- Discovery: ...\n</skills_instructions>"
                            .to_string(),
                    }],
                    phase: None,

                    internal_chat_message_metadata_passthrough: None,},
                ResponseItem::Message {
                    id: Some(codex_protocol::ResponseItemId::from_server(
                        "user".to_string(),
                    )),
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "hello".to_string(),
                    }],
                    phase: None,

                    internal_chat_message_metadata_passthrough: None,},
            ],
            cwd: Some(std::env::temp_dir()),
            ..Prompt::default()
        };

        let (request, _) = build_request(
            &prompt,
            &test_model_info(),
            "kimi-code-no-skills-conversation",
        )
        .expect("build request");

        let system = request["messages"][0]["content"]
            .as_str()
            .expect("system content");
        assert!(!system.contains("{{ KIMI_SKILLS }}"));
        assert!(!system.contains("qa-testing"));
        assert!(!system.contains("### Extra"));
    }

    #[test]
    fn kimi_code_request_renders_tool_call_ids_with_underscores() {
        let prompt = Prompt {
            input: vec![
                ResponseItem::Message {
                    id: Some(codex_protocol::ResponseItemId::from_server(
                        "user".to_string(),
                    )),
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "Find Python files".to_string(),
                    }],
                    phase: None,

                    internal_chat_message_metadata_passthrough: None,
                },
                ResponseItem::FunctionCall {
                    id: None,
                    name: "Glob".to_string(),
                    namespace: None,
                    arguments: r#"{"pattern":"*.py"}"#.to_string(),
                    call_id: "Glob:0".to_string(),

                    internal_chat_message_metadata_passthrough: None,
                },
                ResponseItem::FunctionCallOutput {
                    id: None,
                    call_id: "Glob:0".to_string(),
                    output: FunctionCallOutputPayload::from_text("module.py".to_string()),

                    internal_chat_message_metadata_passthrough: None,
                },
            ],
            cwd: Some(std::env::temp_dir()),
            ..Prompt::default()
        };

        let (request, _) = build_request(
            &prompt,
            &test_model_info(),
            "kimi-code-tool-call-id-conversation",
        )
        .expect("build request");
        let messages = request["messages"].as_array().expect("messages array");
        let assistant_tool_call = messages
            .iter()
            .find_map(|message| message.get("tool_calls"))
            .expect("assistant tool call");
        let tool_message = messages
            .iter()
            .find(|message| message.get("role").and_then(serde_json::Value::as_str) == Some("tool"))
            .expect("tool message");

        assert_eq!(assistant_tool_call[0]["id"], json!("Glob_0"));
        assert_eq!(tool_message["tool_call_id"], json!("Glob_0"));
    }

    #[test]
    fn kimi_code_request_preserves_image_content() {
        let prompt = Prompt {
            input: vec![ResponseItem::Message {
                id: Some(codex_protocol::ResponseItemId::from_server(
                    "user".to_string(),
                )),
                role: "user".to_string(),
                content: vec![
                    ContentItem::InputText {
                        text: "Describe this image.".to_string(),
                    },
                    ContentItem::InputImage {
                        image_url: "data:image/png;base64,AAAB".to_string(),
                        detail: None,
                    },
                ],
                phase: None,

                internal_chat_message_metadata_passthrough: None,
            }],
            cwd: Some(std::env::temp_dir()),
            ..Prompt::default()
        };

        let (request, _) = build_request(
            &prompt,
            &test_model_info(),
            "kimi-code-image-content-conversation",
        )
        .expect("build request");

        assert_eq!(
            request["messages"][1]["content"],
            json!([
                {
                    "type": "text",
                    "text": "Describe this image."
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": "data:image/png;base64,AAAB",
                        "id": null
                    }
                }
            ])
        );
    }

    fn test_model_info() -> ModelInfo {
        serde_json::from_value(json!({
            "slug": "kimi-k2.5",
            "display_name": "Kimi K2.5",
            "description": null,
            "supported_reasoning_levels": [],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": 1,
            "availability_nux": null,
            "upgrade": null,
            "base_instructions": "base",
            "model_messages": null,
            "supports_reasoning_summaries": false,
            "default_reasoning_summary": "auto",
            "support_verbosity": false,
            "default_verbosity": null,
            "apply_patch_tool_type": "freeform",
            "truncation_policy": {
                "mode": "bytes",
                "limit": 10000
            },
            "supports_parallel_tool_calls": false,
            "supports_image_detail_original": false,
            "context_window": null,
            "auto_compact_token_limit": null,
            "effective_context_window_percent": 95,
            "experimental_supported_tools": [],
            "input_modalities": ["text", "image"],
            "supports_search_tool": false
        }))
        .expect("deserialize test model")
    }
}

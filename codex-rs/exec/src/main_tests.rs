use super::*;
use pretty_assertions::assert_eq;

#[test]
fn top_cli_parses_resume_prompt_after_config_flag() {
    const PROMPT: &str = "echo resume-with-global-flags-after-subcommand";
    let cli = TopCli::parse_from([
        "codex-exec",
        "resume",
        "--strict-config",
        "--last",
        "--json",
        "--model",
        "gpt-5.2-codex",
        "--config",
        "reasoning_level=xhigh",
        "--dangerously-bypass-approvals-and-sandbox",
        "--skip-git-repo-check",
        PROMPT,
    ]);
    let mut inner = cli.inner;
    inner
        .config_overrides
        .prepend_root_overrides(cli.config_overrides);

    let Some(codex_exec::Command::Resume(args)) = inner.command.as_ref() else {
        panic!("expected resume command");
    };
    let effective_prompt = args.prompt.clone().or_else(|| {
        if args.last {
            args.session_id.clone()
        } else {
            None
        }
    });
    assert_eq!(effective_prompt.as_deref(), Some(PROMPT));
    assert_eq!(inner.config_overrides.raw_overrides.len(), 1);
    assert_eq!(
        inner.config_overrides.raw_overrides[0],
        "reasoning_level=xhigh"
    );
    assert!(inner.strict_config);
}

#[test]
fn resolve_interpreter_home_prefers_interpreter_home() {
    let resolved = resolve_interpreter_home_from_env(
        /*codex_home*/ None,
        Some(OsStr::new("/tmp/interpreter-home")),
        Some(OsStr::new("/tmp/open-interpreter-home")),
        Some(PathBuf::from("/tmp/home")),
    )
    .expect("home should resolve");
    assert_eq!(resolved, PathBuf::from("/tmp/interpreter-home"));
}

#[test]
fn resolve_interpreter_home_preserves_explicit_codex_home() {
    let resolved = resolve_interpreter_home_from_env(
        Some(OsStr::new("/tmp/codex-home")),
        Some(OsStr::new("/tmp/interpreter-home")),
        Some(OsStr::new("/tmp/open-interpreter-home")),
        Some(PathBuf::from("/tmp/home")),
    )
    .expect("home should resolve");
    assert_eq!(resolved, PathBuf::from("/tmp/codex-home"));
}

#[test]
fn resolve_interpreter_home_falls_back_to_open_interpreter_home() {
    let resolved = resolve_interpreter_home_from_env(
        /*codex_home*/ None,
        /*interpreter_home*/ None,
        Some(OsStr::new("/tmp/open-interpreter-home")),
        Some(PathBuf::from("/tmp/home")),
    )
    .expect("home should resolve");
    assert_eq!(resolved, PathBuf::from("/tmp/open-interpreter-home"));
}

#[test]
fn resolve_interpreter_home_defaults_to_dot_openinterpreter() {
    let resolved = resolve_interpreter_home_from_env(
        /*codex_home*/ None,
        /*interpreter_home*/ None,
        /*open_interpreter_home*/ None,
        Some(PathBuf::from("/tmp/home")),
    )
    .expect("home should resolve");
    assert_eq!(resolved, PathBuf::from("/tmp/home/.openinterpreter"));
}

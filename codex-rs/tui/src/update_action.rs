#[cfg(any(not(debug_assertions), test))]
use codex_install_context::InstallContext;
#[cfg(any(not(debug_assertions), test))]
use codex_install_context::InstallMethod;
#[cfg(any(not(debug_assertions), test))]
use codex_install_context::StandalonePlatform;

const OPEN_INTERPRETER_BRAND_ENV_VAR: &str = "OPEN_INTERPRETER_BRAND";
const CODEX_RELEASE_NOTES_URL: &str = "https://github.com/openai/codex/releases/latest";
const OPEN_INTERPRETER_RELEASE_NOTES_URL: &str =
    "https://github.com/KillianLucas/oix/releases/latest";
#[cfg_attr(debug_assertions, allow(dead_code))]
const CODEX_LATEST_RELEASE_URL: &str = "https://api.github.com/repos/openai/codex/releases/latest";
#[cfg_attr(debug_assertions, allow(dead_code))]
const OPEN_INTERPRETER_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/KillianLucas/oix/releases/latest";
const CODEX_STANDALONE_UNIX_UPDATE_COMMAND: &str =
    "curl -fsSL https://chatgpt.com/codex/install.sh | CODEX_NON_INTERACTIVE=1 sh";
const OPEN_INTERPRETER_STANDALONE_UNIX_UPDATE_COMMAND: &str = "\
curl -fsSL https://github.com/KillianLucas/oix/releases/latest/download/install.sh | \
CODEX_NON_INTERACTIVE=1 \
CODEX_GITHUB_REPO=KillianLucas/oix \
CODEX_INSTALL_PRODUCT_NAME='Open Interpreter' \
CODEX_PACKAGE_ASSET_STEM=open-interpreter-package \
CODEX_COMMAND_NAME=interpreter \
CODEX_RELEASE_TAG_PREFIX=rust-v \
sh";
const CODEX_STANDALONE_WINDOWS_UPDATE_COMMAND: &str =
    "$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex";
const OPEN_INTERPRETER_STANDALONE_WINDOWS_UPDATE_COMMAND: &str = "\
$env:CODEX_NON_INTERACTIVE=1; \
$env:CODEX_GITHUB_REPO='KillianLucas/oix'; \
$env:CODEX_INSTALL_PRODUCT_NAME='Open Interpreter'; \
$env:CODEX_PACKAGE_ASSET_STEM='open-interpreter-package'; \
$env:CODEX_COMMAND_NAME='interpreter'; \
$env:CODEX_RELEASE_TAG_PREFIX='rust-v'; \
if ([string]::IsNullOrWhiteSpace($env:CODEX_HOME)) { $env:CODEX_HOME = Join-Path $HOME '.openinterpreter' }; \
if ([string]::IsNullOrWhiteSpace($env:CODEX_INSTALL_DIR)) { $env:CODEX_INSTALL_DIR = Join-Path $env:LOCALAPPDATA 'Programs\\Open Interpreter\\bin' }; \
irm https://github.com/KillianLucas/oix/releases/latest/download/install.ps1 | iex";
const CODEX_STANDALONE_UNIX_UPDATE_ARGS: &[&str] = &["-c", CODEX_STANDALONE_UNIX_UPDATE_COMMAND];
const OPEN_INTERPRETER_STANDALONE_UNIX_UPDATE_ARGS: &[&str] =
    &["-c", OPEN_INTERPRETER_STANDALONE_UNIX_UPDATE_COMMAND];
const CODEX_STANDALONE_WINDOWS_UPDATE_ARGS: &[&str] = &[
    "-ExecutionPolicy",
    "Bypass",
    "-c",
    CODEX_STANDALONE_WINDOWS_UPDATE_COMMAND,
];
const OPEN_INTERPRETER_STANDALONE_WINDOWS_UPDATE_ARGS: &[&str] = &[
    "-ExecutionPolicy",
    "Bypass",
    "-c",
    OPEN_INTERPRETER_STANDALONE_WINDOWS_UPDATE_COMMAND,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductUpdateSource {
    Codex,
    OpenInterpreter,
}

impl ProductUpdateSource {
    pub(crate) fn current() -> Self {
        if std::env::var_os(OPEN_INTERPRETER_BRAND_ENV_VAR).is_some() {
            Self::OpenInterpreter
        } else {
            Self::Codex
        }
    }

    pub(crate) fn release_notes_url(self) -> &'static str {
        match self {
            ProductUpdateSource::Codex => CODEX_RELEASE_NOTES_URL,
            ProductUpdateSource::OpenInterpreter => OPEN_INTERPRETER_RELEASE_NOTES_URL,
        }
    }

    #[cfg_attr(debug_assertions, allow(dead_code))]
    pub(crate) fn latest_release_url(self) -> &'static str {
        match self {
            ProductUpdateSource::Codex => CODEX_LATEST_RELEASE_URL,
            ProductUpdateSource::OpenInterpreter => OPEN_INTERPRETER_LATEST_RELEASE_URL,
        }
    }

    fn standalone_unix_update_args(self) -> &'static [&'static str] {
        match self {
            ProductUpdateSource::Codex => CODEX_STANDALONE_UNIX_UPDATE_ARGS,
            ProductUpdateSource::OpenInterpreter => OPEN_INTERPRETER_STANDALONE_UNIX_UPDATE_ARGS,
        }
    }

    fn standalone_windows_update_args(self) -> &'static [&'static str] {
        match self {
            ProductUpdateSource::Codex => CODEX_STANDALONE_WINDOWS_UPDATE_ARGS,
            ProductUpdateSource::OpenInterpreter => OPEN_INTERPRETER_STANDALONE_WINDOWS_UPDATE_ARGS,
        }
    }
}

/// Update action the CLI should perform after the TUI exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    /// Update via `npm install -g @openai/codex@latest`.
    NpmGlobalLatest,
    /// Update via `bun install -g @openai/codex@latest`.
    BunGlobalLatest,
    /// Update via `brew upgrade codex`.
    BrewUpgrade,
    /// Update via `curl -fsSL https://chatgpt.com/codex/install.sh | CODEX_NON_INTERACTIVE=1 sh`.
    StandaloneUnix,
    /// Update via `$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex`.
    StandaloneWindows,
}

impl UpdateAction {
    #[cfg(any(not(debug_assertions), test))]
    pub(crate) fn from_install_context(context: &InstallContext) -> Option<Self> {
        match &context.method {
            InstallMethod::Npm => Some(UpdateAction::NpmGlobalLatest),
            InstallMethod::Bun => Some(UpdateAction::BunGlobalLatest),
            InstallMethod::Brew => Some(UpdateAction::BrewUpgrade),
            InstallMethod::Standalone { platform, .. } => Some(match platform {
                StandalonePlatform::Unix => UpdateAction::StandaloneUnix,
                StandalonePlatform::Windows => UpdateAction::StandaloneWindows,
            }),
            InstallMethod::Other => None,
        }
    }

    /// Returns the list of command-line arguments for invoking the update.
    pub fn command_args(self) -> (&'static str, &'static [&'static str]) {
        self.command_args_for_source(ProductUpdateSource::current())
    }

    pub(crate) fn command_args_for_source(
        self,
        source: ProductUpdateSource,
    ) -> (&'static str, &'static [&'static str]) {
        match self {
            UpdateAction::NpmGlobalLatest => ("npm", &["install", "-g", "@openai/codex"]),
            UpdateAction::BunGlobalLatest => ("bun", &["install", "-g", "@openai/codex"]),
            UpdateAction::BrewUpgrade => ("brew", &["upgrade", "--cask", "codex"]),
            UpdateAction::StandaloneUnix => ("sh", source.standalone_unix_update_args()),
            UpdateAction::StandaloneWindows => {
                ("powershell", source.standalone_windows_update_args())
            }
        }
    }

    /// Returns string representation of the command-line arguments for invoking the update.
    pub fn command_str(self) -> String {
        let (command, args) = self.command_args();
        shlex::try_join(std::iter::once(command).chain(args.iter().copied()))
            .unwrap_or_else(|_| format!("{command} {}", args.join(" ")))
    }

    pub(crate) fn command_str_for_source(self, source: ProductUpdateSource) -> String {
        let (command, args) = self.command_args_for_source(source);
        shlex::try_join(std::iter::once(command).chain(args.iter().copied()))
            .unwrap_or_else(|_| format!("{command} {}", args.join(" ")))
    }
}

#[cfg(not(debug_assertions))]
pub fn get_update_action() -> Option<UpdateAction> {
    UpdateAction::from_install_context(InstallContext::current())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use pretty_assertions::assert_eq;

    #[test]
    fn maps_install_context_to_update_action() {
        let native_release_dir =
            AbsolutePathBuf::from_absolute_path(std::env::temp_dir().join("native-release"))
                .expect("temp dir path should be absolute");

        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Other,
                package_layout: None,
            }),
            None
        );
        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Npm,
                package_layout: None,
            }),
            Some(UpdateAction::NpmGlobalLatest)
        );
        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Bun,
                package_layout: None,
            }),
            Some(UpdateAction::BunGlobalLatest)
        );
        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Brew,
                package_layout: None,
            }),
            Some(UpdateAction::BrewUpgrade)
        );
        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Standalone {
                    platform: StandalonePlatform::Unix,
                    release_dir: native_release_dir.clone(),
                    resources_dir: Some(native_release_dir.join("codex-resources")),
                },
                package_layout: None,
            }),
            Some(UpdateAction::StandaloneUnix)
        );
        assert_eq!(
            UpdateAction::from_install_context(&InstallContext {
                method: InstallMethod::Standalone {
                    platform: StandalonePlatform::Windows,
                    release_dir: native_release_dir.clone(),
                    resources_dir: Some(native_release_dir.join("codex-resources")),
                },
                package_layout: None,
            }),
            Some(UpdateAction::StandaloneWindows)
        );
    }

    #[test]
    fn standalone_update_commands_rerun_latest_installer() {
        assert_eq!(
            UpdateAction::StandaloneUnix.command_args(),
            (
                "sh",
                &[
                    "-c",
                    "curl -fsSL https://chatgpt.com/codex/install.sh | CODEX_NON_INTERACTIVE=1 sh"
                ][..],
            )
        );
        assert_eq!(
            UpdateAction::StandaloneWindows.command_args(),
            (
                "powershell",
                &[
                    "-ExecutionPolicy",
                    "Bypass",
                    "-c",
                    "$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex"
                ][..],
            )
        );
    }

    #[test]
    fn open_interpreter_standalone_update_commands_rerun_latest_installer() {
        assert_eq!(
            UpdateAction::StandaloneUnix
                .command_args_for_source(ProductUpdateSource::OpenInterpreter),
            (
                "sh",
                &["-c", OPEN_INTERPRETER_STANDALONE_UNIX_UPDATE_COMMAND][..],
            )
        );
        assert_eq!(
            UpdateAction::StandaloneWindows
                .command_args_for_source(ProductUpdateSource::OpenInterpreter),
            (
                "powershell",
                &[
                    "-ExecutionPolicy",
                    "Bypass",
                    "-c",
                    OPEN_INTERPRETER_STANDALONE_WINDOWS_UPDATE_COMMAND,
                ][..],
            )
        );
    }

    #[test]
    fn update_source_urls_follow_product() {
        assert_eq!(
            ProductUpdateSource::Codex.release_notes_url(),
            "https://github.com/openai/codex/releases/latest"
        );
        assert_eq!(
            ProductUpdateSource::OpenInterpreter.release_notes_url(),
            "https://github.com/KillianLucas/oix/releases/latest"
        );
        assert_eq!(
            ProductUpdateSource::Codex.latest_release_url(),
            "https://api.github.com/repos/openai/codex/releases/latest"
        );
        assert_eq!(
            ProductUpdateSource::OpenInterpreter.latest_release_url(),
            "https://api.github.com/repos/KillianLucas/oix/releases/latest"
        );
    }
}

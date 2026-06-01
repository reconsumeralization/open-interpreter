const OPEN_INTERPRETER_BRAND_ENV_VAR: &str = "OPEN_INTERPRETER_BRAND";
const CODEX_INSTALLER_URL: &str = "https://chatgpt.com/codex/install.sh";
const OPEN_INTERPRETER_INSTALLER_URL: &str =
    "https://github.com/KillianLucas/oix/releases/latest/download/install.sh";
const CODEX_INSTALL_COMMAND: &str = "curl -fsSL https://chatgpt.com/codex/install.sh | sh";
const OPEN_INTERPRETER_INSTALL_COMMAND: &str = "\
curl -fsSL https://github.com/KillianLucas/oix/releases/latest/download/install.sh | \
CODEX_NON_INTERACTIVE=1 \
CODEX_GITHUB_REPO=KillianLucas/oix \
CODEX_INSTALL_PRODUCT_NAME='Open Interpreter' \
CODEX_PACKAGE_ASSET_STEM=open-interpreter-package \
CODEX_COMMAND_NAME=interpreter \
CODEX_RELEASE_TAG_PREFIX=rust-v \
sh";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallProduct {
    Codex,
    OpenInterpreter,
}

impl InstallProduct {
    pub(crate) fn current() -> Self {
        if std::env::var_os(OPEN_INTERPRETER_BRAND_ENV_VAR).is_some() {
            Self::OpenInterpreter
        } else {
            Self::Codex
        }
    }

    pub(crate) fn installer_url(self) -> &'static str {
        match self {
            InstallProduct::Codex => CODEX_INSTALLER_URL,
            InstallProduct::OpenInterpreter => OPEN_INTERPRETER_INSTALLER_URL,
        }
    }

    pub(crate) fn install_command(self) -> &'static str {
        match self {
            InstallProduct::Codex => CODEX_INSTALL_COMMAND,
            InstallProduct::OpenInterpreter => OPEN_INTERPRETER_INSTALL_COMMAND,
        }
    }

    pub(crate) fn installer_env(self) -> &'static [(&'static str, &'static str)] {
        match self {
            InstallProduct::Codex => &[],
            InstallProduct::OpenInterpreter => &[
                ("CODEX_NON_INTERACTIVE", "1"),
                ("CODEX_GITHUB_REPO", "KillianLucas/oix"),
                ("CODEX_INSTALL_PRODUCT_NAME", "Open Interpreter"),
                ("CODEX_PACKAGE_ASSET_STEM", "open-interpreter-package"),
                ("CODEX_COMMAND_NAME", "interpreter"),
                ("CODEX_RELEASE_TAG_PREFIX", "rust-v"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn codex_install_source_uses_upstream_defaults() {
        assert_eq!(
            InstallProduct::Codex.installer_url(),
            "https://chatgpt.com/codex/install.sh"
        );
        assert_eq!(
            InstallProduct::Codex.install_command(),
            "curl -fsSL https://chatgpt.com/codex/install.sh | sh"
        );
        assert_eq!(InstallProduct::Codex.installer_env(), &[]);
    }

    #[test]
    fn open_interpreter_install_source_uses_oix_release_channel() {
        assert_eq!(
            InstallProduct::OpenInterpreter.installer_url(),
            "https://github.com/KillianLucas/oix/releases/latest/download/install.sh"
        );
        assert!(
            InstallProduct::OpenInterpreter
                .install_command()
                .contains("CODEX_PACKAGE_ASSET_STEM=open-interpreter-package")
        );
        assert_eq!(
            InstallProduct::OpenInterpreter.installer_env(),
            &[
                ("CODEX_NON_INTERACTIVE", "1"),
                ("CODEX_GITHUB_REPO", "KillianLucas/oix"),
                ("CODEX_INSTALL_PRODUCT_NAME", "Open Interpreter"),
                ("CODEX_PACKAGE_ASSET_STEM", "open-interpreter-package"),
                ("CODEX_COMMAND_NAME", "interpreter"),
                ("CODEX_RELEASE_TAG_PREFIX", "rust-v"),
            ]
        );
    }
}

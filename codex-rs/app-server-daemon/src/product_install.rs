pub(crate) type InstallProduct = codex_product_info::Product;

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

//! Entry-point for the `codex-exec` binary.
//!
//! When this CLI is invoked normally, it parses the standard `codex-exec` CLI
//! options and launches the non-interactive Codex agent. However, if it is
//! invoked with arg0 as `codex-linux-sandbox`, we instead treat the invocation
//! as a request to run the logic for the standalone `codex-linux-sandbox`
//! executable (i.e., parse any -s args and then run a *sandboxed* command under
//! Landlock + seccomp.
//!
//! This allows us to ship a completely separate set of functionality as part
//! of the `codex-exec` binary.
use clap::Parser;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_exec::Cli;
use codex_exec::run_main;
use codex_product_info::OPEN_INTERPRETER_BRAND_ENV_VAR;
use codex_utils_cli::CliConfigOverrides;
use std::ffi::OsStr;
use std::path::PathBuf;

const CODEX_HOME_ENV_VAR: &str = "CODEX_HOME";
const INTERPRETER_HOME_ENV_VAR: &str = "INTERPRETER_HOME";
const OPEN_INTERPRETER_HOME_ENV_VAR: &str = "OPEN_INTERPRETER_HOME";
const DEFAULT_OPEN_INTERPRETER_HOME_DIR: &str = ".openinterpreter";

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    ensure_interpreter_exec_home_env()?;
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let top_cli = TopCli::parse();
        // Merge root-level overrides into inner CLI struct so downstream logic remains unchanged.
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .prepend_root_overrides(top_cli.config_overrides);

        run_main(inner, arg0_paths).await?;
        Ok(())
    })
}

fn ensure_interpreter_exec_home_env() -> anyhow::Result<PathBuf> {
    let codex_home = std::env::var_os(CODEX_HOME_ENV_VAR);
    let explicit_codex_home = non_empty_path(codex_home.as_deref()).is_some();
    let resolved = resolve_interpreter_home_from_env(
        codex_home.as_deref(),
        std::env::var_os(INTERPRETER_HOME_ENV_VAR).as_deref(),
        std::env::var_os(OPEN_INTERPRETER_HOME_ENV_VAR).as_deref(),
        fallback_home_directory(),
    )?;
    std::fs::create_dir_all(&resolved)?;
    let canonical = resolved.canonicalize()?;

    // SAFETY: main() calls this before arg0_dispatch_or_else creates background
    // threads, so mutating the process environment here is safe.
    unsafe {
        if !explicit_codex_home {
            std::env::set_var(CODEX_HOME_ENV_VAR, &canonical);
        }
        std::env::set_var(INTERPRETER_HOME_ENV_VAR, &canonical);
        std::env::set_var(OPEN_INTERPRETER_HOME_ENV_VAR, &canonical);
        std::env::set_var(OPEN_INTERPRETER_BRAND_ENV_VAR, "1");
    }

    Ok(canonical)
}

fn resolve_interpreter_home_from_env(
    codex_home: Option<&OsStr>,
    interpreter_home: Option<&OsStr>,
    open_interpreter_home: Option<&OsStr>,
    fallback_home_dir: Option<PathBuf>,
) -> anyhow::Result<PathBuf> {
    if let Some(path) = non_empty_path(codex_home) {
        return Ok(path);
    }

    if let Some(path) = non_empty_path(interpreter_home) {
        return Ok(path);
    }

    if let Some(path) = non_empty_path(open_interpreter_home) {
        return Ok(path);
    }

    let Some(home_dir) = fallback_home_dir else {
        anyhow::bail!("failed to resolve Open Interpreter home directory");
    };

    Ok(home_dir.join(DEFAULT_OPEN_INTERPRETER_HOME_DIR))
}

fn non_empty_path(value: Option<&OsStr>) -> Option<PathBuf> {
    value.filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn fallback_home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

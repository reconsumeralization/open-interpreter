# Open Interpreter

Open Interpreter is a provider-agnostic coding agent for your terminal.

It is built as a native Rust fork of the OpenAI Codex CLI/runtime, with Open
Interpreter product defaults, branding, local state, and provider/harness
selection.

---

## Installation

```bash
# macOS and Linux
curl -fsSL https://openinterpreter.com/install | sh
```

```powershell
# Windows
irm https://openinterpreter.com/install.ps1 | iex
```

After installation, start Open Interpreter in any project:

```bash
interpreter
```

`i` is a built-in shorthand for `interpreter`, so this works too:

```bash
i
```

You will be prompted to sign in or configure a provider, then Open Interpreter
will work in the current directory.

## Product Decisions

These are intentional Open Interpreter behaviors that should remain true as the
fork tracks upstream Codex:

- `interpreter` starts a thin terminal UI that connects to the native
  app-server runtime. It starts or reuses the daemon as needed.
- The live runtime is native Rust. Harness modes are selected inside the
  runtime instead of shelling out to external agent CLIs.
- `/model` is provider-first: choose a provider, choose or type a model, choose
  reasoning or thinking controls when the selected model supports them, then
  choose the harness behavior.
- `/harness` is a separate control for changing harness behavior without
  changing providers or models.
- Provider and model membership comes from generated provider catalog artifacts,
  including wire API metadata, instead of hand-written provider lists in Rust.
- The terminal UI uses Open Interpreter branding: an unboxed Open Interpreter
  header, Open Interpreter onboarding/status/update copy, neutral grey footer
  metadata, transparent user/composer surfaces, solid separator rules, and
  `Interpreting` for active work.
- Open Interpreter defaults should stay light: the normal interactive path uses
  the app-server-backed TUI and should not pull optional code-mode runtime
  weight into every installed binary.
- Standalone installs from the public macOS/Linux and Windows installers should
  remain managed installs that can discover future Open Interpreter releases.

## What You Can Do

- Ask questions about the codebase in the current directory.
- Make edits, run commands, and inspect files from the terminal.
- Switch providers, models, reasoning levels, and harness behavior from the TUI.
- Use `interpreter exec` for non-interactive scripting.
- Keep config and session state local under `~/.openinterpreter`.

## CLI Compatibility

`interpreter` preserves the Codex CLI command surface under the Open
Interpreter name. Subcommands, flags, and non-interactive flows should behave
like their Codex CLI equivalents.

The intentional difference is bare startup: running `interpreter` without a
subcommand starts Open Interpreter's app-server-backed interactive TUI.

For programmatic integrations, use OpenAI's Codex SDK and point it at
`interpreter` or `interpreter-app-server`. See [the SDK docs](./docs/sdk.md).

## Building Locally

For local release builds from this checkout, use the repository build script:

```bash
./scripts/build-interpreter-release.sh
```

Do not build only `interpreter` by itself for local release testing. The
user-facing binary is a launcher/router and depends on sibling release
binaries at runtime: `interpreter-tui`, `interpreter-root-tui`,
`interpreter-app-server`, and `interpreter-exec`.

See [BUILDING.md](./BUILDING.md) for the full local build contract.

## License

Apache-2.0

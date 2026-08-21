# AST Research — UPD-002 (in-place self-update) — 2026-08-21

Scope: `rust/fspec-core/src` (shared engine home), `rust/fspec/src` (CLI),
`rust/fspec-tui/src` (TUI), plus dependency-shape invariants.

## 1. Shared engine home: `codelet-fspec-core::update`

- `codelet-fspec-core` is the pure-logic crate consumed by the `fspec`
  binary. Every command file exposes
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
  (verified across `commands/*.rs`). The update engine is NOT a spec-file
  command (no project_root) — it is a standalone `update` module with its
  own `thiserror` type per the attachment §3.1.
- `codelet-fspec-core/Cargo.toml` already depends on `serde`, `serde_json`,
  `thiserror`, `tokio`, `sha2`. It does NOT yet depend on `reqwest`, `tar`,
  `flate2`, `zip`, `self-replace`, `hex` — these must be added.

## 2. Dependency-arrow constraints (the deciding factor)

- `rust/fspec-tui/tests/source_shape_cargo.rs::
  codelet_fspec_tui_production_dependencies_include_only_rpc_seam_and_ratatui_deps`
  asserts the `fspec-tui` `[dependencies]` table:
  - MUST list codelet-rpc / -types / -embedded / -server, ratatui, crossterm,
    tokio, async-trait, futures, tarpc, tokio-tungstenite, url, anyhow, tracing
  - MUST NOT list `codelet-napi`
  - MUST NOT list `codelet-core`
  - (does NOT forbid `codelet-fspec-core`)
- `rust/fspec-tui/tests/no_napi_dependency.rs` forbids a transitive
  `codelet-napi` and any `codelet_napi` import in `fspec-tui/src`.
- `codelet-fspec-core` does NOT depend on `codelet-napi` or `codelet-core`
  (it depends on `codelet-git`, `merman-core`, `gherkin`, …), so adding
  `codelet-fspec-core` to `fspec-tui`'s deps introduces no forbidden arrow
  and no cycle (`codelet-fspec` → both `codelet-fspec-core` and
  `codelet-fspec-tui`; `codelet-fspec-tui` → `codelet-fspec-core`).
- **Decision**: the shared engine lives in `codelet-fspec-core::update`.
  The CLI (`fspec/src/update_cmd.rs`) and the TUI
  (`fspec-tui/src/app/dispatch_slash_update.rs`) both call it — satisfying
  rule [0] (one engine, no duplication). `codelet-fspec-tui` gains a
  `codelet-fspec-core.workspace = true` dependency.

## 3. TUI slash-command shape (mirror CONT-002 `/continue`)

- `rust/fspec-tui/src/app/slash_parser.rs:85` —
  `pub fn parse_slash_command(text: &str) -> SlashCommandParse`; routes
  `/continue`/`/schedule`/`/loop`/`/goal` families to dedicated parsers
  (e.g. L140-142). Add an `UpdateSubcommand(UpdateSubcommand)` variant + a
  `/update` route.
- `rust/fspec-tui/src/app/continue_parser.rs:42` —
  `pub fn parse_continue_command(input: &str) -> ContinueSubcommand` (pure,
  unit-testable). New `update_parser.rs` mirrors this.
- `rust/fspec-tui/src/app/dispatch_slash_continue.rs` —
  `impl App { pub(crate) fn handle_continue_subcommand(&mut self, sub) }`;
  spawns async work via `tokio::spawn` + `Action::EmitSessionNotice`
  (fire-and-forget; keeps the UI thread free). New
  `dispatch_slash_update.rs` mirrors this — it must NOT call
  `std::process::exit` / re-exec (rule [5], [6]).
- `rust/fspec-tui/src/views/agent/slash_commands.rs:30` —
  `enum SlashCommandAction` (palette); `SLASH_COMMANDS` const registry at
  L89. Add an `Update` variant + a registry row so the palette + help
  dialog surface `/update`.
- `rust/fspec-tui/src/components/help_content.rs` derives its slash rows
  from `SLASH_COMMANDS`, so adding the registry row auto-updates the help.

## 4. CLI subcommand shape

- `rust/fspec/src/main.rs` — `struct Cli` (L228) + `enum Mode` (L241),
  clap derive. Each subcommand is a `Mode` variant dispatched to a
  `<module>::run(...)` (e.g. `Some(Mode::Status{..}) => status::run(..)` at
  L2384). New `Mode::Update { check: bool }` + `mod update_cmd;`.
- `#[command(version)]` on `Cli` (L218) already prints the workspace
  version via `fspec --version`.
- `rust/fspec/tests/common/mod.rs` provides `fspec_bin()` (via
  `CARGO_BIN_EXE_fspec`) + `project_root()` for CLI integration tests.

## 5. Test harnesses

- `codelet-fspec-core/tests/*.rs` — engine integration tests (mock GitHub
  API via a tiny `axum` server on 127.0.0.1:0; `axum` is a workspace dep).
- `codelet-fspec/tests/upd002_update_cli.rs` — `fspec update --check` exit
  codes via `assert_cmd` + `FSPEC_UPDATE_BASE_URL` env override.
- `codelet-fspec-tui/tests/upd002_update_command_test.rs` — parser +
  routing (mirror `cont002_continue_command_test.rs`).
- `rust/test-helpers/` — `strip_rust_comments`, `collect_rs_files` for the
  source-shape tests (rule [5] no-prompt, [6] no-restart, [8] shared
  engine).

## 6. Engine API (manual reqwest+sha2 path, attachment §5.3)

- `UpdateConfig` (base_url, repo_owner, repo_name, bin_name,
  current_version, install_path) with `for_production()` reading
  `FSPEC_UPDATE_BASE_URL`.
- `check_latest(&self) -> Result<ReleaseInfo, UpdateError>` — GET
  `{base_url}/repos/{owner}/{repo}/releases/latest`, pick the asset named
  `fspec-<current_target()>.<ext>`, compare tag to current_version.
- `perform_update(&self) -> Result<UpdateOutcome, UpdateError>` — if
  up-to-date, no-op; else download→verify SHA-256→extract→atomic replace.
- `current_target()` — cfg!-based triple (attachment §3.1).
- Errors: `UpdateError::{Network, NoAssetForTarget, ChecksumMismatch,
  ReplaceFailed}` (thiserror). No `unwrap()`/`panic!()` in production code.

## 7. Decision (2026-08-21, user-confirmed)

- Manual reqwest+sha2 download path (NOT the `self_update` crate) —
  testable against a local mock GitHub API via `base_url` override, immune
  to crate version drift. `self-replace` is used only for the Windows
  locked-.exe rename.

# AST Research — UPD-001 / UPD-002 (2026-08-21)

Scope: `rust/fspec/src` (CLI binary), `rust/fspec-tui/src` (TUI app),
`rust/fspec-core/src` (shared engine crate).

## Findings

### 1. CLI subcommand shape (rust/fspec/src/main.rs)
- `struct Cli` (L228) + `enum Mode` (L241) — clap derive, every subcommand
  is a `Mode` variant; dispatch is a match arm calling `<module>::run(...)`
  (e.g. `Some(Mode::Status { connect }) => status::run(connect).await` at
  L2384).
- Subcommand modules live flat in `rust/fspec/src/` (e.g. `status.rs`,
  `list_work_units.rs`). New `fspec update` → `mod update_cmd;` +
  `Mode::Update` variant + match arm.
- `#[command(version)]` on `Cli` (L218) — `fspec --version` already prints
  the workspace version.

### 2. Shared engine shape (rust/fspec-core/src/commands/)
- Every command file exposes exactly:
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
  (verified across unlink_coverage.rs:78, compact_work_unit.rs:57,
  update_work_unit.rs:89, etc.).
- The update engine is NOT a spec-file command (no project_root); it is a
  standalone module `codelet-fspec-core::update` with its own
  `UpdateError` (thiserror) per the UPD-002 attachment §3.1.

### 3. TUI slash-command shape (rust/fspec-tui/src/app/)
- `slash_parser.rs:85` — `pub fn parse_slash_command(text: &str) -> SlashCommandParse`;
  routes `/continue`/`/schedule`/`/loop`/`/goal` families to dedicated
  parsers (e.g. L140-142 for `/continue`).
- `continue_parser.rs:42` — `pub fn parse_continue_command(input: &str) -> ContinueSubcommand`
  (pure function, unit-testable; CONT-002 precedent).
- `dispatch_slash_continue.rs` — `impl App { pub(crate) fn handle_continue_subcommand(&mut self, sub: ContinueSubcommand) }`;
  spawns async work via `tokio::spawn` + `Action::EmitSessionNotice` for
  completion/error lines (fire-and-forget pattern, keeps UI thread free).
- New `/update` mirrors this: `update_parser.rs` + `SlashCommandParse::UpdateSubcommand`
  variant + `dispatch_slash_update.rs`.

### 4. Version surface
- `rust/Cargo.toml [workspace.package] version` is the single source of
  truth (was `0.1.0`, bumped to `0.10.0` for UPD-001).
- `rust/rust-toolchain.toml` pins channel `1.95.0`.
- `[profile.release-slim]` (rust/Cargo.toml L344) exists — the CI profile.

### 5. Test harnesses
- `rust/fspec/tests/common/mod.rs` — `fspec_bin()` via `CARGO_BIN_EXE_fspec`;
  177 existing CLI integration test files follow the
  `tests/cli_<command>.rs` pattern with `@step` comments.
- `rust/fspec-tui/tests/cont002_continue_command_test.rs` — TUI slash
  parser/routing test precedent for `/update`.
- `rust/test-helpers/` — shared temp-dir/fixture helpers.

## Decision (2026-08-21, user-confirmed)
- UPD-002 update engine: **manual reqwest+sha2 download path**
  (attachment §5.3) instead of the `self_update` crate — testable against a
  local mock GitHub API via a `base_url` override, immune to crate
  version drift.
- UPD-001: implement + local verification only; the `v0.10.0` tag push is
  left to the user.

# BUG-285 AST research — `fspec --sync-version`

Queries run with `fspec astgrep` from the repo root.

## Where the stale version comes from

```
fspec astgrep --pattern 'const FSPEC_VERSION: $T = $V;' --lang rust --path rust/fspec-core/src/commands/
rust/fspec-core/src/commands/init.rs:28:1:const FSPEC_VERSION: &str = "0.9.3";
rust/fspec-core/src/commands/report_bug_to_github.rs:41:1:const FSPEC_VERSION: &str = "0.9.3";
```

`init.rs` embeds the pinned TS-parity `0.9.3` into the generated slash command
(`generate_slash_command_content`, `fspec --sync-version {FSPEC_VERSION}`), while
the binary reports the workspace version (`fspec --version` → `CARGO_PKG_VERSION`).
Fix: `FSPEC_VERSION = env!("CARGO_PKG_VERSION")` so the embedded and running
versions are the same source.

```
fspec astgrep --pattern 'const BOOTSTRAP_DOC: $T = include_str!($F);' --lang rust --path rust/fspec-core/src/commands/
rust/fspec-core/src/commands/bootstrap.rs:39:1:const BOOTSTRAP_DOC: &str = include_str!("bootstrap_doc.txt");
```

`bootstrap_doc.txt` is a byte-exact capture with `fspec --sync-version 0.9.3` on
line 5. `bootstrap::run` already substitutes `__AST_RESEARCH_BLOCK__`-style
placeholders, so line 5 becomes `__FSPEC_VERSION__`, substituted with
`init::FSPEC_VERSION`.

## Reusable install logic (no duplication)

```
fspec astgrep --pattern 'fn install_agent_files($$$ARGS) -> $R { $$$BODY }' --lang rust --path rust/fspec-core/src/commands/init.rs
rust/fspec-core/src/commands/init.rs:469:1:fn install_agent_files(project_root: &Path, agent: &Agent) -> Result<Vec<String>, FspecCoreError> {

fspec astgrep --pattern 'pub fn detect_agents($$$ARGS) -> $R { $$$BODY }' --lang rust --path rust/fspec-core/src/commands/init.rs
rust/fspec-core/src/commands/init.rs:356:1:pub fn detect_agents(project_root: &Path) -> Vec<String> {
```

`install_agent_files` writes `spec/<DOC_TEMPLATE>` (spec/AGENTS.md for claude,
spec/CURSOR.md for cursor) and the slash command (`.claude/commands/fspec.md`,
`.cursor/commands/fspec.md`). sync-version calls it through a small
`pub(crate)` wrapper keyed by agent id; `detect_agents` is the filesystem
fallback when spec/fspec-config.json is missing/corrupt.

## Tool-config checks on a version match

```
fspec astgrep --pattern 'pub(super) fn $NAME(project_root: &Path) -> String { $$$BODY }' --lang rust --path rust/fspec-core/src/commands/update_work_unit_status/reminders.rs
reminders.rs:744:1:pub(super) fn check_test_command(project_root: &Path) -> String {
reminders.rs:803:1:pub(super) fn check_quality_commands(project_root: &Path) -> String {

fspec astgrep --pattern 'pub(crate) fn format_agent_output($$$ARGS) -> $R { $$$BODY }' --lang rust --path rust/fspec-core/src/commands/review.rs
rust/fspec-core/src/commands/review.rs:188:1:pub(crate) fn format_agent_output(agent: &ResolvedAgent, message: &str) -> String {
```

The configure-tools checks (`checkTestCommand` / `checkQualityCommands` ports)
already exist; visibility is widened to `pub(crate)` so sync-version reuses them.
`format_agent_output` wraps the restart message in `<system-reminder>` for
agents that support it.

## CLI front door

```
fspec astgrep --pattern 'struct Cli { $$$FIELDS }' --lang rust --path rust/fspec/src/cli_registration/
rust/fspec/src/cli_registration/cli.rs:15:1:struct Cli {

fspec astgrep --pattern 'pub const ADDED_MODULES: $T = $V;' --lang rust --path rust/fspec/tests/refac020/
rust/fspec/tests/refac020/source.rs:63:1:pub const ADDED_MODULES: &[&str] = &["daemon_directory", "daemons_cmd"];
```

`/fspec` runs `fspec --sync-version X` with no subcommand, so the flag is a
top-level (hidden, internal) clap field on `Cli`. The REFAC-020 frozen-contract
tests compare `Cli`, `main` and the module header against a frozen blob, so the
new field, the `sync_version` bridge module and the single early-return
statement in `main` are registered as approved additions in
`rust/fspec/tests/refac020/source.rs` (same mechanism RPC-447 used for `daemons`).

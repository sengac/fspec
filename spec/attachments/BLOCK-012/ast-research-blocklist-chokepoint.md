# AST Research — BLOCK-012 blocklist loading chokepoint

Research for BLOCK-012 (auto-install default system blocklist template).
Method: DeepSearch over `rust/tools/src/`, `rust/napi/src/blocklist.rs`,
`rust/fspec/src/common.rs`, `rust/sessions/src/codex_allowlist.rs`.

## (1) Function signatures & line numbers — `rust/tools/src/blocklist/middleware.rs`

| Function | Line | Signature |
|---|---|---|
| `system_config_path` | 45 | `pub fn system_config_path() -> Option<PathBuf>` — `~/.fspec/blocklist.json` via `dirs::home_dir()` |
| `project_config_path` | 50 | `pub fn project_config_path(project_root: &Path) -> PathBuf` — `<root>/.fspec/blocklist.json` |
| `load_blocklist_config` | 56 | `pub fn load_blocklist_config(project_root: Option<&Path>) -> BlocklistConfig` — loads system + project, merges (project wins) |
| `init_blocklist` | 92 | `pub fn init_blocklist(project_root: Option<&Path>)` — stores root in `BLOCKLIST_PROJECT_ROOT` (RwLock), builds `BLOCKLIST_MATCHER` |
| `check_bash_command` | 129 | `pub fn check_bash_command(command: &str, session_id: uuid::Uuid) -> Result<(), BlockedError>` |
| `check_file_path` | 197 | `pub fn check_file_path(file_path: &str, session_id: uuid::Uuid) -> Result<(), BlockedError>` |
| `reload_blocklist` | 282 | `pub fn reload_blocklist(project_root: Option<&Path>) -> BlocklistMatcher` |

## (2) Callers of `load_blocklist_config` (hot-reload chokepoint)

All four funnel through it inside `middleware.rs`:

- `init_blocklist` — line 102
- `check_bash_command` — line 133 (reloads on **every** check; matcher rebuilt per call, line 139)
- `check_file_path` — line 201 (same per-call reload pattern)
- `reload_blocklist` — line 283

Note: `check_command_raw` (line 263) does **not** call it — it reads the
cached `BLOCKLIST_MATCHER` under a read lock.

## (3) Tool files in `rust/tools/src` calling the check functions

**`check_bash_command`:**
- `bash.rs:134` and `bash.rs:242` (import line 22)
- `unified_exec/tool.rs:145` (import line 12)

**`check_file_path`:**
- `write.rs:84` (import line 9)
- `read.rs:289` (import line 12)
- `edit.rs:87` (import line 10)
- `apply_patch/mod.rs:39` (import line 13)

Re-exported publicly at `rust/tools/src/lib.rs:104` and `blocklist/mod.rs:28`.

## (4) Direct `load_blocklist_config` callers outside codelet-tools

- **`rust/napi/src/blocklist.rs`** — YES. Imports at line 9, calls at
  **line 140** inside `blocklist_load(project_root: Option<String>)`
  (the `#[napi]` binding, lines 138–142). Also uses `init_blocklist` (128),
  `project_config_path` (150, 182), `system_config_path` (173),
  `check_command_raw` (164).
- **`rust/fspec/src`** — NO direct calls. `common.rs:105` calls
  `codelet_tools::blocklist::init_blocklist(Some(workspace))` (RPC-407,
  inside `build_service`). `blocklist_init_tests.rs` uses
  `init_blocklist`/`check_bash_command` — but never `load_blocklist_config`.
- Also relevant: `rust/sessions/src/handle_impl.rs:1765` documents that
  `blocklist_list` deliberately does **not** call `load_blocklist_config`
  (loads system + project separately via `system_config_path()` /
  `project_config_path()` to preserve per-rule provenance).

## data/ directory & codex_allowlist.rs precedent

- **`codelet-tools` has NO `data/` directory** — `rust/tools/` contains only
  `Cargo.toml`, `src/`, `tests/`. `blocklist/` contains only `config.rs`,
  `matcher.rs`, `middleware.rs`, `mod.rs`.
- **`rust/sessions` DOES have `data/`** with `codex-models.json`.
  `codex_allowlist.rs` embeds it at compile time via `include_str!`:
  ```rust
  // line 26
  const BUNDLED_ALLOWLIST_JSON: &str = include_str!("../data/codex-models.json");
  ```
  `load_codex_allowlist()` (line 50) tries the user override
  `~/.fspec/codex-models.json` first (via `fspec_user_dir()`), falling back
  to parsing the bundled constant; parse/empty failures degrade to an
  unfiltered list.

## Conclusion

`load_blocklist_config` (middleware.rs:56) is the single chokepoint every
runtime check funnels through (bash, unified_exec, read, write, edit,
apply_patch, napi `blocklist_load`, `init_blocklist`, `reload_blocklist`).
Placing the check-then-write auto-install inside it covers all entry points
with one change. The `include_str!` + `data/*.json` pattern from
`codex_allowlist.rs` is the established precedent for embedding the
template.

# AST Research: RLCD-001 service supervisor

Date: 2026-09-24 (RLCD-001 specifying phase)
Method: AstGrep + targeted reads across `rust/` (codelet-tools, codelet-sessions, codelet-fspec-core)

## Findings

### 1. Config read/write pattern to mirror (codelet-sessions)

- `rust/sessions/src/profile_sections.rs:270` — `pub(crate) fn fspec_user_dir() -> Option<PathBuf>`:
  resolution order `FSPEC_USER_DIR` env → `dirs::home_dir()/.fspec` → `$HOME/.fspec`.
  The RLCD module must mirror this resolution (tools cannot import sessions —
  crate arrow tools → sessions is forbidden; tools already depends on `dirs`).
- `rust/sessions/src/profile_sections.rs:471` — `pub(crate) fn read_config_value(&Path) -> Option<serde_json::Value>`:
  missing file → None (debug log), malformed → None (warn log). Never an error.
- `rust/sessions/src/profile_sections.rs:499` — `pub(crate) fn write_config_value(&Path, &Value) -> io::Result<()>`:
  pretty-prints + trailing newline; workspace `preserve_order` serde_json keeps
  key order on round-trip.
- Both are `pub(crate)` in sessions → codelet-tools gets its own small mirror
  (architecture note contract), NOT a re-export.

### 2. Key-preserving writer precedent (the model to copy)

`rust/sessions/src/last_used_model_persistence.rs:69` —
`save_persisted_model_string_to(user_dir, model)`:
read root → `json!({})` on missing/malformed → non-object root replaced by `{}`
→ `entry("tui").or_insert_with(json!({}))` → insert leaf key →
`create_dir_all(parent)` → write. `Result<(), String>`, callers best-effort.
RLCD-001's `save_rlcd_url_at` mirrors this shape with the top-level `"rlcd"` key
instead of nested under `"tui"`.

### 3. codelet-tools dependency surface (already available, no new deps needed)

`rust/tools/Cargo.toml`: `reqwest` (json + rustls), `tokio`, `thiserror`,
`tracing`, `dirs`, `once_cell`, `serde`/`serde_json` (preserve_order workspace),
dev-deps `tempfile`, `serial_test`, `tokio` (macros + rt-multi-thread),
`codelet-test-helpers`.
`[target.'cfg(unix)'.dependencies]` already includes `libc` (needed for
`process_group(0)` detach — `CommandExt::process_group` on Unix).

### 4. No existing PATH-resolution helper in codelet-tools

Grepped for `fn which` / `which_binary` in `rust/tools/src/` — no results.
MCP (`mcp.rs:379`) only emits a hint string when a program is missing.
RLCD-001 must implement a small `resolve_binary(&str) -> Option<PathBuf>`
(PATH iteration via `std::env::split_paths`) inside the module.

### 5. Test-file gates that will apply to this work unit (codelet-fspec-core)

`rust/fspec-core/src/commands/update_work_unit_status.rs`:
- `specifying → testing` gate chain: example-mapping complete (rules + examples
  + architecture notes + **attachment containing `ast-research`**) →
  unanswered questions blocked → scenarios exist (feature tagged `@RLCD-001`) →
  temporal ordering (feature file mtime AFTER first `specifying` entry) →
  estimate warning.
- `testing → implementing` temporal gate uses `find_test_files`, which ONLY
  scans `src/**/__tests__/**/*.test.ts` — Rust integration tests in
  `rust/tools/tests/*.rs` are NOT subject to that file-level mtime check.
- `implementing`/`validating` gate (`step_docstrings.rs`): coverage file must
  exist, ≥1 test mapping, **exactly ONE distinct test file per feature**
  (1:1 mapping — the shared `rlcd_mock.rs` helper must therefore NOT be
  linked as a second test file; only `rlcd001_service_supervisor.rs` gets
  linked), and EVERY Gherkin step needs a matching `// @step <Keyword> <text>`
  comment in the linked test file (hybrid-similarity matcher, adaptive
  threshold 0.70–0.85).
- `implementing → validating` requires implMappings on every non-deprecated
  scenario.

### 6. Workspace lint constraints on the implementation

`rust/Cargo.toml [workspace.lints]`: `unsafe_code=deny`; clippy
`expect_used/unwrap_used/panic=deny`; `manual_*` family deny;
`unnecessary_lazy_evaluations`, `redundant_clone`, `needless_borrow` deny.
Tests carry a file-level `#![allow(clippy::unwrap_used, clippy::expect_used,
clippy::panic)]` (same convention as existing `rust/tools/tests/*.rs`).

### 7. Concurrency/global-state precedent

`codelet-tools` already uses `once_cell` statics for registry-style globals
(session_registry, model_capabilities) and `dashmap` for concurrent maps.
The shared `RLCD_STATUS` (global supervisor) follows the once_cell lazy
static pattern; the 10s poll task uses `tokio::runtime::Handle::current()`
(no new runtimes — workspace rule).

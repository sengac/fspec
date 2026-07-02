# RPC-407 — Rust Binary Never Initializes Project Blocklist Root

**Type:** Bug (security/config)
**Crates:** `codelet-fspec` (startup), `codelet-tools` (blocklist middleware — read-only reference)

## 1. Problem statement

`codelet/tools/src/blocklist/middleware.rs` keeps a process-global `BLOCKLIST_PROJECT_ROOT: RwLock<Option<PathBuf>>` (line 17). It is set **only** by `init_blocklist(project_root)` (line 92).

`check_bash_command` (line 129) and `check_file_path` (line 197) hot-reload config on every check via:

```rust
let project_root = get_project_root();               // reads BLOCKLIST_PROJECT_ROOT
let config = load_blocklist_config(project_root.as_deref());
```

`load_blocklist_config` (line 56) merges `~/.fspec/blocklist.json` (system) with `<project_root>/.fspec/blocklist.json` (project, takes precedence).

**Call sites of `init_blocklist`:** only `codelet/napi/src/blocklist.rs:128` (the `blocklistInit` NAPI export, called by the legacy TypeScript shell at startup) plus tests. The standalone Rust binary (`codelet/fspec/src/main.rs` → `daemon.rs` / `combined.rs` / `common.rs::build_service`) **never calls it**. Consequence: in the Rust TUI, project-level `.fspec/blocklist.json` rules are silently ignored — only the home-directory system config applies. A project that adds its own `block`/`prompt` rules gets no protection.

## 2. Fix

Call `codelet_tools::blocklist::init_blocklist(Some(&project_root))` during Rust-binary service startup, using the same project root the service already resolves for sessions (investigate `codelet/fspec/src/common.rs::build_service` and the daemon/combined entry paths — there is an existing notion of project/cwd used when creating `SessionManager` sessions; use that, falling back to `std::env::current_dir()`).

Requirements:
- Both entry modes covered: `daemon` and `combined` (TUI). Prefer one shared seam (e.g. inside `build_service`) so it cannot be forgotten by a future entry point.
- Idempotent / re-init safe: `init_blocklist` already supports reinitialization (doc comment, line 89-91).
- No behavior change for the napi path.
- Note: because `check_*` reload config per call, setting the root **once at startup** is sufficient for hot-reload of rule edits; the root itself only changes if the process's project changes (out of scope).

## 3. Test plan (minimum)

Integration test (in `codelet/fspec/tests/` or wherever `build_service` is testable; real temp dirs, no mocks — redirect `HOME` if the system config would interfere):
1. Create temp project root with `.fspec/blocklist.json` containing a `block` rule for a unique pattern (e.g. `"pattern": "sentinel-rpc407"`, action `block`).
2. Build the service via the same startup path the binary uses (or call the new init seam directly).
3. Assert `check_bash_command("sentinel-rpc407", some_uuid)` returns `Err(BlockedError)` — proving the project config loaded.
4. Negative control: without init (or with a different root), the same check passes.
5. Serial-test guard: `BLOCKLIST_PROJECT_ROOT` is process-global — use `serial_test::serial` like the existing middleware tests, and restore prior state.

Also add a source-shape assertion that the startup path contains the `init_blocklist` call (mirrors the crate's existing shape-test convention) so it can't be silently dropped.

## 4. Non-goals
- Per-session project roots (worktree-isolated sessions using a different root) — the global matches napi behavior today.
- Blocklist editing UI, rule format changes, session-allowance changes.

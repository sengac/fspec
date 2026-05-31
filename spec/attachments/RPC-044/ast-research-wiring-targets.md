# RPC-044 — AST Research: wiring targets

Performed via the `AstGrep` tool against the Rust workspace in `codelet/`.

## 1. The function to edit

```text
codelet/fspec/src/common.rs:68
  pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> { … }
```

Current body (from `Read codelet/fspec/src/common.rs`):

```rust
pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    let data_dir = home_fspec_dir()?;
    codelet_common::set_data_directory(data_dir)
        .map_err(|e| anyhow!("codelet_common::set_data_directory: {e}"))?;

    let watcher = Arc::new(
        WorkUnitsWatcher::new(workspace)
            .with_context(|| format!("WorkUnitsWatcher::new({})", workspace.display()))?,
    );
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(workspace.to_path_buf()),
    ))
}
```

After RPC-044 it constructs a `SessionManager` and threads it through
`SharedFspecService::with_session_manager(...)` instead of the bare
`SharedFspecService::new(...)`.

## 2. The constructor we wire into

```text
codelet/rpc/src/lib.rs:422
  pub fn with_session_manager(
      watcher: Arc<WorkUnitsWatcher>,
      session_manager: Arc<dyn SessionManagerHandle>,
  ) -> Self { … }
```

The constructor stores `session_manager: Some(handle)`; the existing
delegation in `chunks_rx` / `logs_rx` / `status_changes_rx` (lines
526-580) routes broadcasts through the handle. **No additional fan-out
task is needed.**

## 3. The producer crate

```text
codelet/sessions/src/session_manager.rs:180
  impl SessionManager { … }
codelet/sessions/src/session_manager.rs:182
  pub fn new() -> Self { … }
```

The `SessionManager::new()` constructor is parameter-less. It also
implements `codelet_core::SessionManagerHandle` (RPC-042) so it is
directly castable to `Arc<dyn SessionManagerHandle>`.

## 4. Cargo manifest deltas

`codelet/fspec/Cargo.toml` currently lacks `codelet-sessions`. It must
gain an entry; it must NOT gain `codelet-napi`.

The workspace Cargo manifest already exposes
`codelet-sessions = { path = "sessions" }` via
`[workspace.dependencies]` (asserted by
`codelet/sessions/tests/skeleton_invariants.rs::scenario_cargo_workspace_recognises_the_new_codelet_sessions_crate`).

## 5. Dependency-rule regression-test precedent

The existing pattern lives in
`codelet/sessions/tests/skeleton_invariants.rs::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi`
(lines 116-234). It uses:

1. `cargo metadata --format-version 1` (workspace-wide).
2. `serde_json` parse of the JSON.
3. Find the root package by `name`.
4. BFS over `resolve.nodes[*].dependencies` to compute the transitive
   set, mapping IDs back to package names.
5. Assert `codelet-napi` is NOT in the transitive set.

The three new test files (`codelet/fspec/tests/no_napi_dependency.rs`,
`codelet/fspec-tui/tests/no_napi_dependency.rs`,
`codelet/sessions/tests/no_napi_dependency.rs`) reuse this exact
pattern with the appropriate root-package-name substitution.

For the source-import check, the precedent is
`codelet/rpc-embedded/tests/rpc_006_source_shape.rs::scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi`
(lines 124-158). It walks `src/*.rs` files via the helper
`collect_rs_files` (from `tests/source_helpers/mod.rs`) and asserts no
`use codelet_napi` / `codelet_napi::` substring after comment stripping.

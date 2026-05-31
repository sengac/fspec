# RPC-044 — Wire `codelet-sessions::SessionManager` into `codelet-fspec::common::build_service`; add dependency-rule regression tests

**Parent:** RPC-030 · **Phase:** 5.1-5.3 · **Estimate:** 3 pts · **Depends on:** RPC-043

## Goal

Edit `codelet/fspec/src/common.rs::build_service` so it constructs a real `Arc<dyn SessionManagerHandle>` from `codelet_sessions::SessionManager` and passes it into `SharedFspecService`. After this card, the `fspec` binary in all three modes (combined, daemon, client) runs real agent sessions through the NAPI-free `codelet-sessions` crate.

Add dependency-rule regression tests asserting `fspec → napi`, `fspec-tui → napi`, `sessions → napi` arrows do not exist.

## Source — `codelet/fspec/src/common.rs::build_service`

Current shape (before this card):

```rust
pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace.to_path_buf())?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(workspace.to_path_buf()),
    ))
}
```

Target shape:

```rust
use codelet_core::SessionManagerHandle;
use codelet_sessions::SessionManager;

pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace.to_path_buf())?);
    let session_manager: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new());
    // (or SessionManager::new_with_data_dir(...) if it needs the workspace explicitly)

    Ok(Arc::new(
        SharedFspecService::with_session_manager(watcher, session_manager)
            .with_cwd(workspace.to_path_buf()),
    ))
}
```

## `codelet/fspec/Cargo.toml` changes

Add:
```toml
codelet-sessions = { path = "../sessions" }
codelet-core = { path = "../core" }       # if not already present
```

**Confirm absent (CRITICAL — the forbidden arrow):**
```toml
# codelet-napi = ... ← MUST NOT EXIST
```

## Subscription wiring

`SharedFspecService::chunks_rx()` already returns a `broadcast::Receiver<(SessionId, StreamChunk)>`. In `with_session_manager`, the constructor should bridge `session_manager.chunks_rx()` → `SharedFspecService::chunks_tx` so the chunks flow from the real session manager through the shared service to all WS clients and the embedded backend.

Check `codelet/rpc/src/lib.rs::SharedFspecService::with_session_manager` constructor — confirm it spawns a fan-out task subscribing to `session_manager.chunks_rx()` and republishing on the service's own `chunks_tx`. If not, add it.

Same for `logs_rx` and the new `status_changes_rx` (added in RPC-037/041).

## Dependency-rule regression tests

### Existing — must remain green

`codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — asserts no `rpc → napi` references. No change needed.

### New — `codelet/fspec/tests/no_napi_dependency.rs`

```rust
//! Regression test: the `fspec` binary must NOT transitively depend on
//! `codelet-napi`. This is enforced architecturally — the binary uses
//! `codelet-sessions` instead.

#[test]
fn no_codelet_napi_in_dependency_graph() {
    let output = std::process::Command::new("cargo")
        .args(["tree", "-p", "codelet-fspec", "-e", "normal"])
        .output()
        .expect("cargo tree failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("codelet-napi"),
        "codelet-napi appeared in codelet-fspec's dependency tree:\n{stdout}"
    );
}

#[test]
fn no_napi_imports_in_source() {
    use walkdir::WalkDir;
    for entry in WalkDir::new("../fspec/src").into_iter().flatten() {
        if entry.path().extension().map_or(false, |e| e == "rs") {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains("codelet_napi"),
                "Found codelet_napi import in {:?}",
                entry.path()
            );
        }
    }
}
```

### New — `codelet/fspec-tui/tests/no_napi_dependency.rs`

Same shape, scoped to `codelet-fspec-tui`.

### New — `codelet/sessions/tests/no_napi_dependency.rs`

Same shape, scoped to `codelet-sessions`.

## Smoke tests

Run all three `fspec` modes against a stub provider:

1. `fspec` (combined) → start, attach AgentView, type a prompt, observe chunks.
2. `fspec daemon` + `fspec client` → connect via WS, type a prompt, observe chunks.
3. Verify no panics, no `GLOBAL_CHUNK_CALLBACK` references in any log.

## Acceptance criteria

1. `codelet/fspec/src/common.rs::build_service` constructs `Arc<dyn SessionManagerHandle>` from `codelet_sessions::SessionManager` and passes it to `SharedFspecService::with_session_manager`.
2. `codelet/fspec/Cargo.toml` adds `codelet-sessions` dep. Does NOT add `codelet-napi`.
3. Three new dependency-rule regression tests pass:
   - `codelet/fspec/tests/no_napi_dependency.rs`
   - `codelet/fspec-tui/tests/no_napi_dependency.rs`
   - `codelet/sessions/tests/no_napi_dependency.rs`
4. Existing `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` stays green.
5. `cargo build --workspace` passes.
6. `cargo run --bin fspec` boots into combined mode without panic.
7. Sending a prompt via the Rust AgentView produces text chunks visible in the scrollback (validates that `chunks_rx` from real `SessionManager` reaches the AgentView through both transports).

## Risks

- The `SessionManager::new()` constructor needs a workspace path for persistence. Verify the signature accepts a `PathBuf` or document how it discovers the data dir (via `codelet_core::persistence::set_data_directory`).
- `SharedFspecService::with_session_manager` may need an internal fan-out task to bridge `session_manager.chunks_tx` → `service.chunks_tx`. Confirm in `codelet/rpc/src/lib.rs`.
- The `fspec daemon` + `fspec client` split mode: only the daemon should construct the `SessionManager`. The client connects via WS to the daemon's service.

## Out of scope

- AgentView wiring → RPC-045 onwards.
- Verification suite → RPC-065 onwards.

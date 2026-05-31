# RPC-038 — Create `codelet-sessions` crate skeleton

**Parent:** RPC-030 · **Phase:** 4.1 · **Estimate:** 2 pts · **Depends on:** RPC-037

## Goal

Create a new workspace member `codelet/sessions/` (crate name `codelet-sessions`) that will host the extracted `SessionManager` + `BackgroundSession` (RPC-039, RPC-040). No code moves in this card — it only creates the empty crate.

## File layout

```
codelet/sessions/
├── Cargo.toml
└── src/
    └── lib.rs        # empty: pub mod session_manager; pub mod background_session;
```

## `codelet/sessions/Cargo.toml`

```toml
[package]
name = "codelet-sessions"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
codelet-common = { path = "../common" }
codelet-tools = { path = "../tools" }
codelet-providers = { path = "../providers" }
codelet-cli = { path = "../cli" }
codelet-git = { path = "../git" }
codelet-core = { path = "../core" }
codelet-rpc-types = { path = "../rpc-types" }

tokio = { workspace = true, features = ["sync", "rt-multi-thread", "macros"] }
uuid = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
indexmap = { workspace = true }
parking_lot = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

**Notes:**
- No `napi` or `napi-derive` dependency. Ever.
- No `napi` feature flag. Ever.
- `codelet-rpc-types` is depended on WITHOUT the `napi` feature.
- All dependencies are workspace-versioned.

## `codelet/sessions/src/lib.rs`

```rust
//! `codelet-sessions` — the NAPI-free session-manager crate.
//!
//! Hosts `SessionManager` and `BackgroundSession`, the agent loop, and the
//! tokio broadcast wiring that replaces NAPI's `GLOBAL_CHUNK_CALLBACK`.
//!
//! Implements `codelet_core::SessionManagerHandle` so both the Rust `fspec`
//! binary and the existing `codelet-napi` adapter consume the same surface.

pub mod background_session; // populated by RPC-039
pub mod session_manager;    // populated by RPC-040
```

## Root `Cargo.toml`

Add to `members` array:

```toml
[workspace]
members = [
    # ...
    "codelet/sessions",
    # ...
]
```

## Compatibility note

The crate must compile **before** RPC-039/040 land. Use empty module files with a single `pub use` or doc-comment to satisfy `cargo build`:

```rust
// codelet/sessions/src/background_session.rs
//! Placeholder. Populated by RPC-039.
```

## Acceptance criteria

1. `codelet/sessions/Cargo.toml` exists with the dependencies listed above.
2. `codelet/sessions/src/lib.rs` exists with module declarations.
3. Root `Cargo.toml` lists `codelet/sessions` as a workspace member.
4. `cargo build -p codelet-sessions` passes (with empty modules).
5. `cargo metadata` shows zero `codelet-napi` in the transitive dependencies of `codelet-sessions`.
6. Add a smoke test in `codelet/sessions/tests/smoke.rs`:
   ```rust
   #[test] fn crate_compiles() {}
   ```

## Risks

- Cyclic deps: `codelet-cli` may pull in `codelet-providers` which may pull in `codelet-tools` etc. Verify the existing workspace graph doesn't already have a cycle that this crate inherits.
- Build time: adding a new crate to the workspace recompiles everything. Acceptable cost.

## Out of scope

- Moving any code → RPC-039, RPC-040.
- Implementing `SessionManagerHandle` → RPC-042.

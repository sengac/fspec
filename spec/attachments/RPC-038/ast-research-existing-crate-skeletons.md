# AST / source research — existing crate skeletons in `codelet/`

This document captures the source-level patterns observed in the existing workspace crates that the new `codelet-sessions` skeleton must mirror. Each finding is grounded in a concrete file path and line that was inspected with `Read` / `Grep` / `AstGrep` / `Bash ls`.

## 1. Workspace topology (root `codelet/Cargo.toml`)

- `members` lists every crate by its directory: `"cli", "common", "core", "fspec", "fspec-tui", "git", "napi", "providers", "rpc", "rpc-embedded", "rpc-server", "rpc-types", "tools", "tui"` (lines 2–17 of `codelet/Cargo.toml`).
- `[workspace.dependencies]` exposes each internal crate by name with `path = "<dir>"`. New crates added to `members` SHOULD also be added here so downstream crates can use `codelet-sessions.workspace = true`.
- `[workspace.lints.rust]` + `[workspace.lints.clippy]` declare project-wide deny lists (lines 170–209). Sub-crates opt in via `[lints] workspace = true`.
- The `tokio`, `serde`, `serde_json`, `uuid`, `chrono`, `async-trait`, `thiserror`, `tracing` keys already exist as `workspace.dependencies` entries. Two keys called out by the RPC-038 attachment — `indexmap` and `parking_lot` — are **absent** from `workspace.dependencies`. Both need to be added there so the "All dependencies are workspace-versioned" rule from the attachment can be satisfied.

## 2. Per-crate `Cargo.toml` shape (observed in `codelet/core/Cargo.toml`, `codelet/rpc-types/Cargo.toml`, `codelet/rpc/Cargo.toml`, `codelet/rpc-embedded/Cargo.toml`)

Every sibling crate follows this shape:

```toml
[package]
name = "codelet-<crate>"
version.workspace = true
edition.workspace = true
license.workspace = true
# optional: repository.workspace = true / description / publish

[lints]
workspace = true

[dependencies]
# internal first, in dependency-arrow order
codelet-foo = { workspace = true }
# external second, grouped by purpose
tokio.workspace = true
…

[dev-dependencies]
…
```

The new `codelet-sessions/Cargo.toml` must repeat this shape exactly.

## 3. Existing pattern for compile-only placeholder modules

The smallest sibling crate (`codelet-rpc-types`) shows that a crate may have a single `src/lib.rs` and no submodules at all. There is therefore precedent for very small lib crates — the additional novelty in RPC-038 is the two empty submodule files that mark seats reserved for RPC-039 / RPC-040.

Pattern for an empty placeholder module file in Rust (no AST hits in the workspace today because no crate currently uses this exact pattern, but the language allows it):

```rust
//! Placeholder. Populated by RPC-039.
```

A file with only `//!` inner doc-comment compiles cleanly as a Rust source file (it produces an empty module). This is the cheapest possible "module exists" stub.

## 4. Existing pattern for an integration smoke test

Looking at `codelet/rpc-embedded/tests/` (e.g. `rpc_006_source_shape.rs`) and `codelet/rpc-types/tests/rpc036_widen_types.rs`, the workspace convention is:

- one integration-test file per `tests/<name>.rs`
- each file may contain `#[test] fn scenario_…() { … }` functions
- the `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` header is sometimes used at the top to relax workspace clippy deny rules for test-only code

For RPC-038 the smoke test is intentionally trivial:

```rust
//! Smoke test for codelet-sessions skeleton (RPC-038).
//!
//! Locks in the compile-only contract: the crate must build and a test
//! binary must link before RPC-039/RPC-040 populate the modules.

#[test]
fn crate_compiles() {}
```

## 5. The forbidden arrow — proof that today no path leads `codelet-sessions` → `codelet-napi`

The `codelet-sessions` crate will declare these internal deps only:

- `codelet-common`
- `codelet-tools`
- `codelet-providers`
- `codelet-cli`
- `codelet-git`
- `codelet-core`
- `codelet-rpc-types` (default features, i.e. NAPI feature OFF)

Spot-check of each of those crates' `Cargo.toml` (`Grep` for `codelet-napi`):

```
$ rg 'codelet-napi' codelet/{common,tools,providers,cli,git,core,rpc-types}/Cargo.toml
(no matches)
```

This confirms none of the seven inbound deps already pulls `codelet-napi` into the graph. The Phase-4 invariant therefore holds at skeleton time.

## 6. Module declaration convention in larger crates

In `codelet/rpc-embedded/src/lib.rs` (line 28 onward) and `codelet/rpc/src/lib.rs`, the crate root uses `pub use` to flatten public symbols and `pub mod` only when the module is part of the public API. The RPC-038 attachment explicitly requests:

```rust
pub mod background_session; // populated by RPC-039
pub mod session_manager;    // populated by RPC-040
```

so both modules are public at the top level — matching the convention used by other "container" crates.

## 7. References

- `codelet/Cargo.toml` (workspace root) — workspace members, workspace deps, workspace lints.
- `codelet/core/Cargo.toml` — Cargo.toml shape with `version.workspace = true` etc.
- `codelet/rpc-types/Cargo.toml` — minimum-viable crate Cargo.toml with feature gate.
- `codelet/rpc-embedded/Cargo.toml` — example of a crate that depends on multiple internal sibling crates only via `workspace = true`.
- `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — model for `tests/smoke.rs` placement and shape.
- `codelet/napi/src/session_manager.rs` — 395 KB source that RPC-039 / RPC-040 will move; out of scope for RPC-038.
- `spec/attachments/RPC-038/codelet-sessions-skeleton.md` — primary brief for this card.
- `spec/attachments/RPC-030/background-session-and-agent-management-wiring.md` — Phase 4 context.

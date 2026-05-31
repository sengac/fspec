# RPC-067 — AST research: existing no-NAPI dependency-rule test pattern

## Goal

Find every existing test in the workspace that enforces the
`X → codelet-napi` forbidden arrow, so the new shared helper crate
(`codelet/test-helpers/`) faithfully captures the existing duplication
and the migration step is mechanical (no semantic drift).

## AST queries run

### 1. All `workspace_root()` helpers across tests

```
pattern: fn workspace_root() -> PathBuf { $$$BODY }
```

Matched **24 test files** across the workspace. Confirms `workspace_root`
is the de facto idiom for resolving the `codelet/` directory from a test
binary's `CARGO_MANIFEST_DIR`. Implementation is byte-identical across
files — always `manifest_dir.parent().expect(...).to_path_buf()`.

→ Reusable as-is in `codelet-test-helpers::dependency_rules`.

### 2. `strip_rust_comments` duplication

```
pattern: fn strip_rust_comments($$$ARGS) -> String { $$$BODY }
```

Matched **7 test files** (mcp_injection_source_shape, source_shape_rpc049,
ws_backend_smoke, source_shape_rpc050, plus the three RPC-044
`no_napi_dependency.rs` files).

Implementation is byte-identical across all 7 occurrences:

```rust
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' { i += 1; }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}
```

→ Reusable as-is. Lift into `codelet_test_helpers::source_scan::strip_rust_comments`.

### 3. `no_codelet_napi_in_transitive_dependency_graph` test fn

```
pattern: fn no_codelet_napi_in_transitive_dependency_graph() { $$$BODY }
```

Matched **3 test files**: `codelet/{fspec, fspec-tui, sessions}/tests/no_napi_dependency.rs`.

All three implementations follow identical structure:

1. `cargo metadata --format-version 1 --manifest-path <workspace>/Cargo.toml`
2. Parse JSON, locate root package by name (`codelet-fspec` / `codelet-fspec-tui` / `codelet-sessions`)
3. BFS walk `resolve.nodes[].dependencies[]` collecting visited IDs
4. Map IDs back to package names via `packages[].id`
5. Assert `!transitive_names.contains("codelet-napi")`

Only diff between files: the `crate name` literal passed to `find()`.

→ Parameterise: `assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str)`.

### 4. Existing `no_napi_dependency.rs` files

```
glob: **/no_napi_dependency.rs
```

Matched **3 files** (added in RPC-044). All ~190 LOC. ~180 LOC are
duplicated helper code (`workspace_root`, `collect_rs_files`,
`strip_rust_comments`) and the cargo-metadata walk. Only ~10 LOC are
the actual crate-specific assertions.

→ After migration each file should be ≤ 25 LOC total.

### 5. The "broader" rpc-embedded source-shape test

```
file: codelet/rpc-embedded/tests/rpc_006_source_shape.rs
```

Contains **6 scenarios** (default_fixture cfg-gating, host-runtime
handle, codelet_core dep allowed but no codelet_napi, loopback-only bind,
WorkUnitInfo deduplication, no-bincode push path). Only ONE of these
six (`scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi`)
overlaps with the no-napi rule. The other 5 enforce wider invariants.

**Decision:** leave `rpc_006_source_shape.rs` untouched. Migrating only
the one matching scenario would (a) bifurcate ownership (some asserts
in the helper, some inline), (b) lose context for future readers, and
(c) the RPC-067 attachment explicitly lists this file as "already in
place" without prescribing a rewrite.

## Helper API decision

```rust
// codelet/test-helpers/src/dependency_rules.rs

/// Walks `cargo metadata` transitive deps of `from_crate`. Panics if
/// `forbidden_pkg` appears in the resolved set.
pub fn assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str);

/// Walks `codelet/<crate_dir_name>/src/**/*.rs`, strips Rust comments,
/// asserts no file contains `forbidden_module::` or `use <forbidden_module>`.
pub fn assert_no_import_in_sources(crate_dir_name: &str, forbidden_module: &str);
```

Both functions panic on failure with messages that name `from_crate` /
`crate_dir_name`, `forbidden_pkg` / `forbidden_module`, and the offending
files / transitive set — so a sabotage test fails loudly and points at
the violation.

## Risk: helper-crate dependency hygiene

`codelet-test-helpers` must itself stay outside the forbidden-arrow
graph. It depends only on:

- `serde_json` (workspace) — for parsing `cargo metadata` output
- `std::process::Command` — for running cargo
- `std::fs`, `std::path` — for source-tree walking

→ No `codelet-*` dependencies. No third-party `napi` dep. The
`assert_no_transitive_dependency("codelet-test-helpers", "codelet-napi")`
sabotage check is trivially satisfied.

## Files this work creates / modifies

### New
- `codelet/test-helpers/Cargo.toml`
- `codelet/test-helpers/src/lib.rs`
- `codelet/test-helpers/src/dependency_rules.rs`
- `codelet/core/tests/no_napi_dependency.rs`
- `codelet/rpc-types/tests/no_napi_dependency.rs`

### Modified
- `codelet/Cargo.toml` — add `"test-helpers"` to members; add `codelet-test-helpers = { path = "test-helpers" }` to workspace.dependencies
- `codelet/fspec/Cargo.toml` — add `codelet-test-helpers.workspace = true` under `[dev-dependencies]`
- `codelet/fspec-tui/Cargo.toml` — same
- `codelet/sessions/Cargo.toml` — same
- `codelet/core/Cargo.toml` — same
- `codelet/rpc-types/Cargo.toml` — same
- `codelet/fspec/tests/no_napi_dependency.rs` — replace ~190 LOC with ~20 LOC helper calls
- `codelet/fspec-tui/tests/no_napi_dependency.rs` — same
- `codelet/sessions/tests/no_napi_dependency.rs` — same

### Unchanged
- `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — too broad to migrate

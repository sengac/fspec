# RPC-067 — Dependency-rule regression tests for `fspec`, `fspec-tui`, `sessions`

**Parent:** RPC-030 · **Phase:** 8.3 · **Estimate:** 2 pts · **Depends on:** RPC-066

## Goal

Enforce architectural dependency rules with three new regression tests. These tests fail the build if any forbidden arrow is reintroduced.

## The rules

```
fspec ──► fspec-tui ──► rpc / rpc-embedded / rpc-server ──► core ──► providers / tools / git / cli / common
                                                              ▲
                                                              │
                                                   codelet-napi (sink — nothing imports out of it)
```

Forbidden arrows:
- `rpc → napi` (enforced by existing `codelet/rpc-embedded/tests/rpc_006_source_shape.rs`)
- `fspec → napi`
- `fspec-tui → napi`
- `sessions → napi`
- `core → napi`
- `rpc-types → napi`

## Already in place

`codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — checks `rpc → napi`.

`codelet/fspec/tests/no_napi_dependency.rs` — added in RPC-044, checks `fspec → napi`.
`codelet/fspec-tui/tests/no_napi_dependency.rs` — added in RPC-044, checks `fspec-tui → napi`.
`codelet/sessions/tests/no_napi_dependency.rs` — added in RPC-044, checks `sessions → napi`.

## New in this card

Generalise the test pattern into a shared library `codelet/test-helpers/src/dependency_rules.rs`:

```rust
pub fn assert_no_dependency(from_crate: &str, to_crate: &str) {
    let output = std::process::Command::new("cargo")
        .args(["tree", "-p", from_crate, "-e", "normal"])
        .output()
        .expect("cargo tree failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(to_crate),
        "{to_crate} appeared in {from_crate} dependency tree:\n{stdout}"
    );
}

pub fn assert_no_imports_in_sources(crate_root: &str, forbidden_module: &str) {
    use walkdir::WalkDir;
    for entry in WalkDir::new(format!("{crate_root}/src")).into_iter().flatten() {
        if entry.path().extension().map_or(false, |e| e == "rs") {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains(forbidden_module),
                "Found {forbidden_module} import in {:?}",
                entry.path()
            );
        }
    }
}
```

Add new tests:

`codelet/core/tests/no_napi_dependency.rs`:
```rust
#[test] fn no_codelet_napi_in_dependency_graph() {
    codelet_test_helpers::dependency_rules::assert_no_dependency(
        "codelet-core", "codelet-napi",
    );
}
#[test] fn no_napi_imports_in_source() {
    codelet_test_helpers::dependency_rules::assert_no_imports_in_sources(
        "../core", "codelet_napi",
    );
}
```

Add equivalent for:
- `codelet/rpc-types/tests/no_napi_dependency.rs`

Migrate the existing four `no_napi_dependency.rs` tests to use the shared helper.

## Acceptance criteria

1. `codelet/test-helpers/src/dependency_rules.rs` exists with reusable helpers.
2. New tests added: `codelet/core/tests/no_napi_dependency.rs`, `codelet/rpc-types/tests/no_napi_dependency.rs`.
3. Existing tests migrated to use shared helpers.
4. `cargo test --workspace` passes all dependency-rule tests.
5. Sabotage test (run locally): add `codelet_napi = { path = "../napi" }` to `codelet/fspec/Cargo.toml` → assert dependency-rule test FAILS.

## Risks

- `cargo tree` is slow on cold cache. Tests may add 5-10s to CI time. Acceptable.
- `walkdir`-based source scanning matches strings — but `// use codelet_napi` in a comment would false-positive. Mitigation: regex on actual `use` statements only.

## Out of scope

- More sophisticated import graphs (e.g., asserting layer ordering). The current rules are sufficient.

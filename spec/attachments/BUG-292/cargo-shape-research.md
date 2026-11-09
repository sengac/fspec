# Research: `cargo_shape` locked-shape assertions are stale

Target: `cargo test --profile ci-test -p codelet-fspec --test cargo_shape`
Result on current tree **and on a clean HEAD worktree (81ee9f15)**: 6 passed, 3 failed, 9 ignored.
Found during REFAC-020/REFAC-019/BUG-284 validation (2026-10-30); not caused by those cards.

## Failing scenarios

### 1. `scenario_codelet_fspec_is_registered_as_a_workspace_member` (cargo_shape.rs:290)
The test pins the exact alphabetical `[workspace] members` list. `rust/Cargo.toml` now also
contains `attachment-viewer`, which the locked list does not include. `rust/Cargo.toml` is
unchanged versus HEAD, so the crate was added earlier without updating the lock.

### 2. `scenario_fspec_src_contains_exactly_the_locked_file_layout` (cargo_shape.rs:721)
Top-level `rust/fspec/src/*.rs` files not in the allow-list (identical now and at HEAD):

- `astgrep.rs`
- `client_mcp_full.rs`, `client_mcp_results.rs`, `client_mcp_service.rs`
- `init_selector.rs`
- `update_cmd.rs`

(REFAC-020's new `src/cli_registration/` directory is not a top-level `.rs` file and is not flagged.)

### 3. `scenario_no_source_file_constructs_its_own_tokio_runtime` (cargo_shape.rs:964)
`rust/fspec/src/init_selector.rs:328` contains `tokio::runtime::Runtime::new().unwrap()`
inside its `#[cfg(test)]` module (a unit test driving the selector). The scan strips comments
but not `#[cfg(test)]` code, so test-only code trips the production invariant
("every module sources its handle from `Handle::current()`").

## Suggested fix
- Decide whether the locked lists are still the intended contract. If so, add
  `attachment-viewer` and the six bridge files (with the work-unit references that introduced
  them) and update the corresponding feature steps; otherwise relax to "contains at least".
- For the runtime scan, either exclude `#[cfg(test)]` modules (parse with `syn`) or change the
  `init_selector.rs` test to `#[tokio::test]` so no runtime is constructed in `src/`.
  The second option keeps the invariant strict.

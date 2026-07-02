@done
@session-management
@napi
@rust
@codelet
@infrastructure
@workspace
@regression
@source-shape
@lift
@test-infrastructure
@rpc
@RPC-067
Feature: Dependency-rule regression tests for fspec, fspec-tui, sessions
  """
  The new codelet/test-helpers/ crate is added to the workspace `members` list AND exposed as `codelet-test-helpers = { path = "test-helpers" }` in [workspace.dependencies] — so consuming crates declare it via `codelet-test-helpers.workspace = true` in [dev-dependencies]
  The helper API has two functions: `assert_no_codelet_napi_in_dependency_graph(crate_name: &str)` (cargo metadata BFS — fails if codelet-napi appears in the transitive set) and `assert_no_codelet_napi_imports_in_sources(crate_dir: &str)` (walks codelet/<crate_dir>/src/*.rs, strips comments, fails on `use codelet_napi` or `codelet_napi::` substrings)
  Helpers do not hardcode `codelet-napi` — they take it as `forbidden: &str` so the same scaffolding can later forbid other arrows (e.g. `core → napi`-third-party, `rpc-types → tokio`). The first wave passes `"codelet-napi"` and `"codelet_napi"` explicitly.
  Refined helper API: `assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str)` and `assert_no_import_in_sources(crate_dir_name: &str, forbidden_module: &str)`. The crate_dir_name is the directory name under codelet/ (e.g. "fspec", "fspec-tui", "sessions", "core", "rpc-types").
  codelet/test-helpers/Cargo.toml depends on serde_json (workspace) ONLY — no other deps. It is a library crate with [lib] (default) plus the workspace lints inherited. The `[lints] workspace = true` block applies.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A new workspace crate codelet-test-helpers exposes assert_no_dependency(from, to) and assert_no_imports_in_sources(crate_root, forbidden_module) helpers that any tests/*.rs binary can call
  #   2. The helpers use the same cargo metadata JSON-walk strategy already used by codelet/fspec/tests/no_napi_dependency.rs (NOT cargo tree text grep) so behaviour is consistent with the existing RPC-044 tests
  #   3. codelet-test-helpers itself MUST NOT depend on codelet-napi (it must be safe to consume from any forbidden-arrow test) — only dev-quality helpers like serde_json
  #   4. New regression test file codelet/core/tests/no_napi_dependency.rs asserts (a) no codelet-napi in codelet-core's transitive cargo-metadata dependency graph and (b) no `codelet_napi::` / `use codelet_napi` substring under codelet/core/src/
  #   5. New regression test file codelet/rpc-types/tests/no_napi_dependency.rs asserts (a) no codelet-napi in codelet-rpc-types' transitive dependency graph (the optional `napi` feature pulls the third-party `napi` crate, NOT `codelet-napi`) and (b) no `codelet_napi::` / `use codelet_napi` substring under codelet/rpc-types/src/
  #   6. The three existing no_napi_dependency.rs files under codelet/fspec/tests, codelet/fspec-tui/tests, codelet/sessions/tests are migrated to delegate to the shared helpers — each file remains a thin wrapper of two #[test] fns that call the helper with the appropriate crate name + source root
  #   7. codelet/rpc-embedded/tests/rpc_006_source_shape.rs is left untouched — it asserts wider invariants (default_fixture cfg-gating, host-runtime handle, loopback-only bind, WorkUnitInfo deduplication, no-bincode push path) that go beyond the no-napi rule and would be lost if collapsed into the helper
  #   8. cargo test --workspace passes after all changes; specifically all five no_napi_dependency.rs tests (core, rpc-types, fspec, fspec-tui, sessions) are green
  #
  # EXAMPLES:
  #   1. Sabotage scenario (manually verified): temporarily adding `codelet-napi = { workspace = true }` to codelet/core/Cargo.toml makes codelet-core's no_napi_dependency.rs FAIL with a clear message naming codelet-napi
  #   2. When the codelet workspace is in its current state, `cargo test -p codelet-core --test no_napi_dependency` exits 0 and reports the transitive package set excludes `codelet-napi`
  #   3. When `cargo test -p codelet-rpc-types --test no_napi_dependency` runs with the default feature set (no `napi` feature), the transitive package walk completes without finding `codelet-napi`, and the source scan of codelet/rpc-types/src/ finds no `use codelet_napi` import
  #   4. After migration, the file codelet/fspec/tests/no_napi_dependency.rs is under 30 lines and contains only `use codelet_test_helpers::dependency_rules::*` plus two #[test] fns that call assert_no_codelet_napi_in_dependency_graph("codelet-fspec") and assert_no_codelet_napi_imports_in_sources("fspec")
  #   5. If a developer adds `codelet-napi = { workspace = true }` to codelet/core/Cargo.toml and re-runs `cargo test -p codelet-core --test no_napi_dependency`, the test FAILS with an error message containing both `codelet-core` and `codelet-napi` and pointing at the forbidden-arrow rule
  #   6. When the source-scan helper walks a crate root that contains a doc comment with the literal text `// codelet_napi was here`, it does NOT flag the file because comments are stripped before substring matching (mirrors strip_rust_comments in the existing tests)
  #   7. When `cargo test --workspace` runs from a clean target dir, every no_napi_dependency.rs test binary (core, rpc-types, fspec, fspec-tui, sessions) completes within 10 seconds each — the cargo metadata invocation dominates runtime and is shared across all binaries
  #
  # ========================================
  Background: User Story
    As a fspec architect
    I want to enforce the no-codelet-napi forbidden-arrow architectural invariant via shared dependency-rule helpers and per-crate regression tests for codelet-core and codelet-rpc-types
    So that any future code change that reintroduces a forbidden dependency on codelet-napi fails the build instead of silently bleeding the JS bridge into pure-Rust crates

  Scenario: codelet-test-helpers crate is wired into the workspace as a library
    Given the codelet workspace root manifest at codelet/Cargo.toml
    When I inspect the [workspace] members list and the [workspace.dependencies] table
    Then "test-helpers" appears in workspace.members
    And `codelet-test-helpers = { path = "test-helpers" }` appears in workspace.dependencies
    And codelet/test-helpers/Cargo.toml declares package.name = "codelet-test-helpers"
    And codelet/test-helpers/Cargo.toml declares serde_json as its ONLY [dependencies] entry beyond workspace-inherited package fields
    And codelet/test-helpers/Cargo.toml inherits [lints] from workspace

  Scenario: codelet-test-helpers exposes the shared dependency-rule helper API
    Given the new crate codelet/test-helpers/ exists with a library entry point at codelet/test-helpers/src/lib.rs
    When I inspect the public surface of the dependency_rules module
    Then `pub mod dependency_rules` is declared in codelet/test-helpers/src/lib.rs
    And codelet/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str)`
    And codelet/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_import_in_sources(crate_dir_name: &str, forbidden_module: &str)`
    And both helpers use `cargo metadata --format-version 1` for the dependency-graph walk
    And the source-scan helper strips Rust line and block comments before substring matching

  Scenario: codelet-test-helpers itself has no transitive dependency on codelet-napi
    Given the codelet-test-helpers crate is published as a workspace member
    When I run `cargo metadata --format-version 1` and walk the transitive dependencies of codelet-test-helpers
    Then the resulting transitive package set does not contain the package name `codelet-napi`
    And codelet/test-helpers/src does not contain any `use codelet_napi` or `codelet_napi::` substring

  Scenario: codelet-core forbidden-arrow regression test passes against the current workspace
    Given the codelet workspace is in its current RPC-067 state
    When I run `cargo test -p codelet-core --test no_napi_dependency`
    Then the command exits with code 0
    And the transitive dependency walk for codelet-core does NOT contain codelet-napi
    And no `.rs` file under codelet/core/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped

  Scenario: codelet-rpc-types forbidden-arrow regression test passes against the current workspace
    Given the codelet workspace is in its current RPC-067 state
    And the codelet-rpc-types crate is built with the default feature set (no `napi` feature)
    When I run `cargo test -p codelet-rpc-types --test no_napi_dependency`
    Then the command exits with code 0
    And the transitive dependency walk for codelet-rpc-types does NOT contain codelet-napi
    And no `.rs` file under codelet/rpc-types/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped

  Scenario: Migrated codelet-fspec regression test delegates to the shared helper
    Given the file codelet/fspec/tests/no_napi_dependency.rs
    When I inspect its source after the RPC-067 migration
    Then it imports the shared helper module from codelet_test_helpers
    And it contains exactly two #[test] fns
    And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-fspec", "codelet-napi")`
    And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("fspec", "codelet_napi")`
    And codelet/fspec/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]

  Scenario: Migrated codelet-fspec-tui regression test delegates to the shared helper
    Given the file codelet/fspec-tui/tests/no_napi_dependency.rs
    When I inspect its source after the RPC-067 migration
    Then it imports the shared helper module from codelet_test_helpers
    And it contains exactly two #[test] fns
    And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-fspec-tui", "codelet-napi")`
    And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("fspec-tui", "codelet_napi")`
    And codelet/fspec-tui/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]

  Scenario: Migrated codelet-sessions regression test delegates to the shared helper
    Given the file codelet/sessions/tests/no_napi_dependency.rs
    When I inspect its source after the RPC-067 migration
    Then it imports the shared helper module from codelet_test_helpers
    And it contains exactly two #[test] fns
    And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-sessions", "codelet-napi")`
    And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("sessions", "codelet_napi")`
    And codelet/sessions/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]

  Scenario: rpc_006_source_shape.rs is left untouched
    Given the file codelet/rpc-embedded/tests/rpc_006_source_shape.rs
    When I diff its content against the pre-RPC-067 baseline
    Then no lines are removed
    And no test function is replaced by a helper invocation

    # NOTE: the file asserts wider invariants beyond the no-napi rule
  Scenario: cargo test --workspace passes after all RPC-067 changes
    Given all RPC-067 source changes are applied to the workspace
    When I run `cargo test --workspace --tests --no-fail-fast`
    Then the command exits with code 0
    And all five no_napi_dependency.rs test binaries (codelet-core, codelet-rpc-types, codelet-fspec, codelet-fspec-tui, codelet-sessions) report green

  Scenario: Sabotaging codelet-core by adding a codelet-napi dependency makes its test fail
    Given a developer adds `codelet-napi = { workspace = true }` to codelet/core/Cargo.toml
    When the developer runs `cargo test -p codelet-core --test no_napi_dependency`
    Then the test exits with a non-zero code
    And the failure message contains the substring "codelet-napi"
    And the failure message contains the substring "codelet-core"

  Scenario: Source-scan helper ignores comments containing the forbidden substring
    Given a Rust file containing only the line `// codelet_napi was here`
    When `assert_no_import_in_sources` walks the file
    Then the helper does NOT flag the file as an offender

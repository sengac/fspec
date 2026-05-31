@done
@codelet
@session-management
@infrastructure
@scaffolding
@rpc
@RPC-038
Feature: Create codelet-sessions crate skeleton
  """
  indexmap and parking_lot are not currently workspace-versioned in root Cargo.toml — adding them now keeps the attachment's invariant ("All dependencies are workspace-versioned") satisfied and benefits future crates
  Placeholder module files use `//! Placeholder. Populated by RPC-0xx.` doc-comments to satisfy `cargo build` without code
  The integration test `codelet/sessions/tests/smoke.rs` is the test artifact for ACDD coverage linking — `cargo test -p codelet-sessions` is the verifying command
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The crate's package name is `codelet-sessions` and the path is `codelet/sessions/`
  #   2. Cargo.toml uses workspace-versioned dependencies for everything (codelet-common, codelet-tools, codelet-providers, codelet-cli, codelet-git, codelet-core, codelet-rpc-types, tokio, uuid, serde, serde_json, indexmap, parking_lot, thiserror, async-trait, tracing, chrono)
  #   3. codelet-rpc-types is depended on WITHOUT the `napi` feature — codelet-sessions must never carry a transitive napi dependency
  #   4. `codelet/sessions/src/lib.rs` declares two placeholder modules: `pub mod background_session;` and `pub mod session_manager;` — both populated by later cards (RPC-039 / RPC-040)
  #   5. The new crate is added to the root workspace `members` array so `cargo build -p codelet-sessions` resolves
  #   6. `codelet-sessions` is added as a workspace-level dependency entry in the root Cargo.toml so downstream crates (codelet-fspec, codelet-napi) can pick it up via `codelet-sessions.workspace = true` in later cards
  #   7. `cargo build -p codelet-sessions` succeeds against the empty modules
  #   8. `cargo metadata` shows zero `codelet-napi` entries in the transitive dependencies of `codelet-sessions`
  #   9. Smoke test exists at `codelet/sessions/tests/smoke.rs` with a `#[test] fn crate_compiles() {}` to lock in the compile-only contract
  #   10. `workspace.lints` is inherited via `[lints] workspace = true` so the same clippy/rust deny lints apply to the new crate
  #
  # EXAMPLES:
  #   1. Developer runs `cargo build -p codelet-sessions` after creating the crate — build succeeds with no errors
  #   2. Developer runs `cargo metadata -p codelet-sessions --format-version 1` and verifies the JSON contains no package with name `codelet-napi`
  #   3. Developer runs `cargo test -p codelet-sessions` — the smoke test `crate_compiles` runs and passes
  #   4. Developer opens `codelet/sessions/src/lib.rs` and sees `pub mod background_session;` and `pub mod session_manager;` with placeholder doc comments referring forward to RPC-039 / RPC-040
  #   5. Developer opens the root `Cargo.toml` and sees `codelet/sessions` listed in the workspace `members` array alongside the existing crates
  #   6. Developer runs `cargo clippy -p codelet-sessions -- -D warnings` — clippy passes because the workspace lints table is inherited and no code violates the deny rules
  #
  # ========================================
  Background: User Story
    As a Rust developer porting BackgroundSession from codelet-napi
    I want to have an empty codelet-sessions crate skeleton wired into the workspace
    So that RPC-039 and RPC-040 can move the SessionManager and BackgroundSession code into a NAPI-free home

  Scenario: Cargo workspace recognises the new codelet-sessions crate
    Given the root `Cargo.toml` lists `codelet/sessions` as a workspace member and `codelet-sessions` as a workspace dependency
    When I run `cargo metadata -p codelet-sessions --format-version 1`
    Then the output JSON includes a package named `codelet-sessions` at version 0.1.0 with manifest path ending in `codelet/sessions/Cargo.toml`

  Scenario: codelet-sessions builds standalone against the empty modules
    Given the codelet-sessions crate has been scaffolded with empty `background_session` and `session_manager` modules
    When I run `cargo build -p codelet-sessions`
    Then the build completes successfully with no errors

  Scenario: codelet-sessions has no transitive dependency on codelet-napi
    Given the codelet-sessions crate depends on codelet-rpc-types WITHOUT the `napi` feature
    When I run `cargo metadata -p codelet-sessions --format-version 1` and inspect the transitive package set
    Then no package named `codelet-napi` appears anywhere in the transitive dependency graph
    And the only NAPI-related crates that appear are the third-party `napi` / `napi-derive` bindings pulled in by an existing inbound dependency outside the scope of this card — never the local `codelet-napi` crate

  Scenario: Smoke test runs and passes
    Given `codelet/sessions/tests/smoke.rs` contains a `crate_compiles` test
    When I run `cargo test -p codelet-sessions`
    Then the `crate_compiles` test is discovered and passes with status `ok`

  Scenario: lib.rs declares the placeholder modules for later RPC cards
    Given the codelet-sessions crate skeleton has been created
    When I read `codelet/sessions/src/lib.rs`
    Then it declares `pub mod background_session;`
    And it declares `pub mod session_manager;`
    And both module files exist with placeholder doc comments naming the cards that will populate them

  Scenario: Workspace lints are inherited and clippy passes
    Given `codelet/sessions/Cargo.toml` declares `[lints]` with `workspace = true`
    When I run `cargo clippy -p codelet-sessions --all-targets -- -D warnings`
    Then clippy completes successfully with no warnings or errors

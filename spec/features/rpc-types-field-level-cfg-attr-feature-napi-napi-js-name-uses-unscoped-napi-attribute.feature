@done
@BUG-150
@infrastructure
@rpc
@rust
@BUG-146
Feature: rpc-types: field-level #[cfg_attr(feature = "napi", napi(js_name = ...))] uses unscoped `napi` attribute

  # ARCHITECTURE NOTES (revised 2026-05-21 after empirical investigation)
  #
  # ROOT CAUSE (re-verified):
  # rust/rpc-types/src/lib.rs uses TWO different attribute spellings.
  #   * Struct-level: #[cfg_attr(feature = "napi", napi_derive::napi(object))]
  #     ~40 sites; works correctly under the napi feature.
  #   * Field-level:  #[cfg_attr(feature = "napi", napi(js_name = "..."))]
  #     34 sites at lib.rs lines 40, 58, 217, 426, 457, 459, 461, 471, 473,
  #     705, 707, 712, 714, 718, 720, 722, 726, 728, 730, 734, 736, 738, 749,
  #     756, 771, 775, 779, 783, 787, 791, 793, 799, 804, 806.
  #     These FAIL when codelet-napi is built with --features noop (or any
  #     path that transitively enables rpc-types' napi feature).
  #
  # WHY: Rust's cfg_attr expansion runs BEFORE outer-struct proc-macro
  # invocation but Rust eagerly validates the resolved field-level attribute
  # path. With napi NOT in scope it errors `cannot find attribute napi in
  # this scope`. With napi in scope (via `use napi_derive::napi;` OR
  # `#[macro_use] extern crate napi_derive;`) it errors `expected non-macro
  # attribute, found attribute macro napi` because Rust disallows attribute
  # MACROS at field positions. The canonical napi-rs pattern
  # `#[napi(object)] struct X { #[napi(js_name)] pub y: T }` works only when
  # the inner attrs are NOT produced via cfg_attr — the parent macro
  # consumes them during its expansion. Known upstream bug:
  # https://github.com/napi-rs/napi-rs/issues/2635.
  #
  # FIX STRATEGY (chosen by user 2026-05-21):
  # Option (b) — replace each #[cfg_attr(feature = "napi", napi(js_name = "X"))]
  # with #[serde(rename = "X")]. napi-derive v3 is expected to honor
  # serde rename attributes on #[napi(object)] fields. If empirical test
  # shows it does NOT, fall back to Option (c) (struct duplication).
  #
  # ACCEPTED WIRE-SHAPE CHANGE:
  # tarpc/embedded serde JSON keys for these 34 fields move from
  # snake_case (e.g. work_type) to camelCase (e.g. workType). All transport
  # consumers must be audited as part of this card.
  Background: User Story
    As a codelet developer
    I want to build codelet-rpc-types (and any transitive consumer like codelet-napi) with the `napi` feature enabled — including under the noop feature of codelet-napi
    So that the crate compiles cleanly so downstream cards (RPC-043 and beyond) can perform their own builds and shape tests

  Scenario: Reproducing the bug before the fix shows 34 unscoped-napi-attribute errors
    Given I am at the codelet workspace root
    And the file rust/rpc-types/src/lib.rs still contains 34 `#[cfg_attr(feature = "napi", napi(js_name = "..."))]` field-level decorations
    When I run `cargo build -p codelet-napi --features noop`
    Then the build fails
    And the stderr contains 34 errors of the form "cannot find attribute `napi` in this scope"
    And each error points at one of the 34 known field-level cfg_attr sites in lib.rs

  Scenario: After the fix, codelet-rpc-types builds cleanly with the napi feature enabled
    Given the fix (replacing every field-level `napi(js_name = "X")` with `serde(rename = "X")`) has been applied to rust/rpc-types/src/lib.rs
    When I run `cargo build -p codelet-rpc-types --features napi`
    Then the build succeeds with exit code 0
    And the stderr contains "Compiling codelet-rpc-types" or "Finished"
    And the stderr ends with a "Finished" line
    And the stderr does NOT contain "cannot find attribute `napi`"
    And the stderr does NOT contain "expected non-macro attribute"

  Scenario: The default codelet-rpc-types build remains free of napi-derive after the fix
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I run `cargo build -p codelet-rpc-types`
    Then the build succeeds with exit code 0
    When I run `cargo tree -p codelet-rpc-types -e normal --no-default-features`
    Then the listed normal dependencies do NOT include `napi` or `napi-derive`

  Scenario: codelet-napi default features build succeeds after the fix
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I run `cargo build -p codelet-napi`
    Then the build succeeds with exit code 0
    And the stderr no longer contains "cannot find attribute `napi`"

  Scenario: codelet-napi noop feature build succeeds after the fix
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I run `cargo build -p codelet-napi --features noop`
    Then the build succeeds with exit code 0
    And the stderr no longer contains "cannot find attribute `napi`"

  Scenario: codelet-rpc-types JSON round-trip tests still pass with the napi feature
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I type-check the test suite with the napi feature enabled via `cargo check -p codelet-rpc-types --features napi --tests`
    Then the type check succeeds with exit code 0
    And the output contains no compile errors

  Scenario: The fix replaces every field-level napi(js_name) with serde(rename) in the on-disk source
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I inspect the on-disk contents of rust/rpc-types/src/lib.rs
    Then the source contains zero field-level `napi(js_name` attributes outside comments
    And each of the 34 documented renames X appears as `#[serde(rename = "X")]` with at least its documented multiplicity
    And the source contains at least 34 `#[serde(rename = ` attributes in total
    And NO `use napi_derive::napi;` import is present in the source
    And the struct-level `napi_derive::napi(object)` decorations remain in place
    When I inspect the on-disk contents of rust/rpc-types/Cargo.toml
    Then the napi and napi-derive dependencies remain optional and gated behind the napi feature

  Scenario: TypeScript surface preserves every camelCase field after regeneration
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    When I run `cargo build -p codelet-napi --release` to regenerate rust/napi/index.d.ts
    Then the regenerated index.d.ts contains every expected camelCase field name from the 34 original renames
    And the camelCase names appear in the same struct positions they did before

  Scenario: BUG-146 fix unblocks the RPC-043 noop-build assertion
    Given the fix has been applied to rust/rpc-types/src/lib.rs
    And RPC-043 (the 7-module split of rust/napi/src/session_manager.rs) has NOT yet landed
    When I run `cargo build -p codelet-napi --features noop`
    Then the failure mode is no longer "cannot find attribute `napi` in this scope"
    And the failure mode is either success or an RPC-043 structural error (not an rpc-types attribute error)

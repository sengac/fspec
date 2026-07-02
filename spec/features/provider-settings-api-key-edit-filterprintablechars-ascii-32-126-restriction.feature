@done
@rpc-153
@rust
@source-shape
@keyboard-navigation
@ts-parity
@regression
@tui
@provider-settings
@validation
@RPC-153
Feature: Provider settings api-key edit: filterPrintableChars ASCII 32-126 restriction
  """
  Pattern mirrors RPC-152 regression-shape coverage: read source as string, use brace-balancing to scope assertions to a function body, and use byte-offset ORDER assertions for sequencing invariants
  Implementation already exists from RPC-161 — this card is coverage-only, structural pinning of the shape so a regression breaks the test before reaching CI
  Test file: codelet/fspec-tui/tests/rpc153_filter_printable_chars_shape.rs — integration test that reads detail.rs from CARGO_MANIFEST_DIR-relative path
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A helper named `is_printable_ascii` MUST exist in codelet/fspec-tui/src/views/provider_settings/detail.rs with signature `fn is_printable_ascii(c: char) -> bool`
  #   2. The body of `is_printable_ascii` MUST evaluate the inclusive range `(32..=126).contains(&code)` — proving the TS filterPrintableChars boundaries are preserved
  #   3. The `handle_edit_key` function body MUST contain a call to `is_printable_ascii(c)` guarding the `draft.push(c)` append on the `KeyCode::Char(c)` arm
  #   4. The `KeyCode::Char(c)` arm of `handle_edit_key` MUST NOT contain any unconditional `draft.push(c)` outside the `is_printable_ascii` guard
  #
  # EXAMPLES:
  #   1. Source-shape test reads detail.rs as a string and asserts the substring `fn is_printable_ascii(c: char) -> bool` is present
  #   2. Source-shape test asserts the substring `(32..=126).contains(&code)` appears within the brace-balanced body of `is_printable_ascii`
  #   3. Source-shape test extracts the brace-balanced body of `handle_edit_key` and asserts `is_printable_ascii(c)` is referenced inside it
  #   4. Source-shape test verifies byte-offset ORDER: the `is_printable_ascii(c)` call appears BEFORE the `draft.push(c)` line inside the `KeyCode::Char(c)` arm, proving the guard precedes the append
  #
  # ========================================
  Background: User Story
    As a agent maintainer
    I want to pin the structural shape of the filterPrintableChars guard in the API-key edit handler
    So that a future refactor cannot silently drop the ASCII 32-126 restriction and let control chars or non-ASCII bytes leak into provider credentials

  Scenario: is_printable_ascii helper exists in detail.rs with the canonical signature
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I scan the file as a string
    Then the source must contain "fn is_printable_ascii(c: char) -> bool"

  Scenario: is_printable_ascii body evaluates the inclusive ASCII 32..=126 range
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the body of the "fn is_printable_ascii(c: char) -> bool" function
    Then the function body must contain "(32..=126).contains(&code)"

  Scenario: handle_edit_key body guards draft.push(c) through is_printable_ascii
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the handle_edit_key function body
    Then the function body must contain "is_printable_ascii(c)"
    And the function body must contain "draft.push(c)"

  Scenario: is_printable_ascii guard precedes draft.push(c) in handle_edit_key
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the handle_edit_key function body
    Then the function body must contain "is_printable_ascii(c)"
    And the function body must contain "draft.push(c)"
    And the offset of "is_printable_ascii(c)" must be less than the offset of "draft.push(c)"

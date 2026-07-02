@done
@rpc-155
@keyboard-navigation
@ts-parity
@regression
@provider-settings
@tui
@rust
@source-shape
@RPC-155
Feature: Provider settings api-key edit: Delete key removes draft chars (in addition to Backspace)
  """
  Pattern mirrors RPC-152 / RPC-153 / RPC-156 / RPC-151 / RPC-149 regression-shape coverage: read source as string, brace-balance to scope assertions to a function body, byte-offset ORDER for sequencing invariants
  Implementation already exists from RPC-163 in detail.rs:134-146 — this card is coverage-only structural pinning so a regression breaks the test before reaching CI
  Test file: codelet/fspec-tui/tests/rpc155_delete_key_removes_draft_chars_shape.rs — integration test that reads detail.rs from CARGO_MANIFEST_DIR-relative path; sub-millisecond execution
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The `handle_edit_key` function body in codelet/fspec-tui/src/views/provider_settings/detail.rs MUST contain a merged match arm `KeyCode::Backspace | KeyCode::Delete =>` matching both keycodes together
  #   2. The brace-balanced body of the merged `KeyCode::Backspace | KeyCode::Delete` arm MUST contain `draft.pop()` to remove the previous draft character
  #   3. The `handle_edit_key` body MUST NOT contain a standalone `KeyCode::Delete =>` arm — Delete may only appear merged with Backspace, so the two key paths cannot diverge
  #   4. The merged `KeyCode::Backspace | KeyCode::Delete` arm MUST appear in the source before the `KeyCode::Char(c)` arm inside `handle_edit_key`, so that the deletion path is evaluated before the printable-append path
  #
  # EXAMPLES:
  #   1. Source-shape test reads detail.rs and asserts the substring `KeyCode::Backspace | KeyCode::Delete =>` is present inside the brace-balanced body of `handle_edit_key`
  #   2. Source-shape test extracts the merged arm body and asserts it contains `draft.pop()` — proving the deletion behaviour is wired in
  #   3. Source-shape test asserts the `handle_edit_key` body contains zero occurrences of a standalone `KeyCode::Delete =>` arm (i.e. Delete only appears in the merged form, never on its own line)
  #   4. Source-shape test verifies byte-offset ORDER: the `KeyCode::Backspace | KeyCode::Delete =>` arm appears before the `KeyCode::Char(c) =>` arm inside the brace-balanced body of `handle_edit_key`
  #
  # ========================================
  Background: User Story
    As a agent maintainer
    I want to pin the structural shape of the merged `KeyCode::Backspace | KeyCode::Delete` arm in the API-key edit handler
    So that a future refactor cannot silently drop the Delete-key binding and let Rust diverge from Ink's `key.backspace || key.delete` parity

  Scenario: handle_edit_key body contains the merged KeyCode::Backspace | KeyCode::Delete arm
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the handle_edit_key function body
    Then the function body must contain "KeyCode::Backspace | KeyCode::Delete =>"

  Scenario: merged Backspace|Delete arm body contains draft.pop() deletion call
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the brace-balanced body of the "KeyCode::Backspace | KeyCode::Delete =>" arm inside handle_edit_key
    Then the arm body must contain "draft.pop()"

  Scenario: handle_edit_key body contains zero standalone KeyCode::Delete arms
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the handle_edit_key function body
    Then the function body must contain zero occurrences of the standalone substring "KeyCode::Delete =>" with no preceding "Backspace | " prefix

  Scenario: merged Backspace|Delete arm precedes the KeyCode::Char(c) arm in handle_edit_key
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    When I extract the handle_edit_key function body
    Then the function body must contain "KeyCode::Backspace | KeyCode::Delete =>"
    And the function body must contain "KeyCode::Char(c) =>"
    And the offset of "KeyCode::Backspace | KeyCode::Delete =>" must be less than the offset of "KeyCode::Char(c) =>"

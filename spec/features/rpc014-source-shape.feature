@done
@RPC-014
@rust
@tui
@board-view
@regression
@file-structure
@rpc
Feature: RPC-014 source-shape regressions — board grid + details strip modules

  """
  RPC-014 (slice 3 of 3) — Source-shape regressions pin the file layout
  and cross-crate invariants introduced by the rich BoardView port:

    - WorkUnitInfo gains a `pub attachments: Vec<String>` field;
    - codelet/core/src/work_units.rs parses `attachments` with
      `#[serde(default)]` so legacy JSON without the field still loads;
    - the new modules `codelet/fspec-tui/src/views/board/grid.rs` and
      `codelet/fspec-tui/src/views/board/details_strip.rs` exist and stay
      under the 300 LoC ceiling;
    - the orchestrator `codelet/fspec-tui/src/views/board.rs` stays under
      the 300 LoC ceiling;
    - RPC-013 source-shape invariants stay green — Navigator still does
      not reserve a Length(1) footer row, FooterView is still removed.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a source-shape regression test that pins the RPC-014 module split, the WorkUnitInfo attachments field, and the persisting RPC-013 invariants
    So that future cards cannot silently merge the grid helpers back into board.rs or strip the attachments field

  Scenario: Grid helpers and details_strip modules exist as separate files
    Given the codelet/fspec-tui crate after RPC-014 lands
    When a developer scans the views directory
    Then the file codelet/fspec-tui/src/views/board/grid.rs exists
    And the file codelet/fspec-tui/src/views/board/details_strip.rs exists
    And the file codelet/fspec-tui/src/views/board.rs exists

  Scenario: New and modified board modules stay under 300 lines
    Given the directory codelet/fspec-tui/src/views/board/
    When a test counts the line-count of every .rs file in views/board/ plus views/board.rs
    Then views/board.rs has fewer than 300 lines
    And views/board/grid.rs has fewer than 300 lines
    And views/board/details_strip.rs has fewer than 300 lines

  Scenario: WorkUnitInfo gains the attachments field
    Given codelet/rpc-types/src/lib.rs after RPC-014 lands
    When a developer reads the WorkUnitInfo struct body
    Then the body contains the substring "pub attachments: Vec<String>"

  Scenario: Core work_units parser reads attachments with serde default
    Given codelet/core/src/work_units.rs after RPC-014 lands
    When a developer reads the WorkUnitRecord struct body
    Then the body contains the field name "attachments"
    And the field carries a `#[serde(default)]` attribute so missing fields parse as Vec::new()

  Scenario: RPC-013 invariants preserved
    Given codelet/fspec-tui/src/views/navigator.rs after RPC-014 lands
    Then the file does NOT contain "Constraint::Length(1)"
    And codelet/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    And codelet/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    And the file codelet/fspec-tui/src/views/footer.rs does NOT exist

  Scenario: BoardView still emits Action variants and renders the RPC-013 footer
    Given codelet/fspec-tui/src/views/board.rs after RPC-014 lands
    When a developer scans the file source raw
    Then the file contains the substring "Action::EnterWorkUnit"
    And the file contains the substring "Action::FocusNextColumn"
    And the file contains the substring "Action::ReorderUp"
    And the file contains the substring "← → Columns"
    And the file contains the substring "↵ Work Agent"
    And the file contains the substring "ESC Back"

  Scenario: Views still avoid encapsulated transport crates and host runtime construction
    Given the directory codelet/fspec-tui/src/views/ (including the new board/ subdir)
    When a test scans every *.rs file
    Then NO file imports `codelet_napi::` or `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
    And NO file contains `tokio::runtime::Builder` or `Runtime::new()`

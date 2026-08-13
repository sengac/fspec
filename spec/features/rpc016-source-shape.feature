@done
@RPC-016
@rust
@tui
@board-view
@regression
@file-structure
@rpc
@source-shape
Feature: RPC-016 source-shape regressions — viewport module + WorkUnitInfo.last_state_change_at + Action variants
  """
  RPC-016 (slice 3 of 3) — Source-shape regressions pin the file layout
  and cross-crate invariants introduced by the BoardView per-column
  scroll port:

  - WorkUnitInfo in rust/rpc-types/src/lib.rs gains
  `pub last_state_change_at: Option<String>` (ISO-8601 UTC).
  - rust/core/src/work_units.rs reads `stateHistory[last].timestamp`
  from spec/work-units.json and writes it into
  `WorkUnitInfo.last_state_change_at`.
  - The Action enum in rust/fspec-tui/src/components/mod.rs gains
  four new variants: ScrollFocusedColumnUp, ScrollFocusedColumnDown,
  SelectFirstInFocused, SelectLastInFocused.
  - The BoardStore in rust/fspec-tui/src/store/board.rs declares
  the new `scroll_offsets` field and the matching mutation methods.
  - The new viewport painter module exists under
  rust/fspec-tui/src/views/board/ and stays < 300 LoC.
  - RPC-012 / RPC-013 / RPC-014 / RPC-015 source-shape invariants
  stay green.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a source-shape regression test that pins the RPC-016 viewport module, the additive last_state_change_at field, the new Action variants and the persisting RPC-012..015 invariants
    So that future cards cannot silently merge the viewport painter back into board.rs or strip the new fields

  Scenario: WorkUnitInfo gains the last_state_change_at field
    Given rust/rpc-types/src/lib.rs after RPC-016 lands
    When a developer reads the file source raw
    Then the file contains the substring "pub last_state_change_at: Option<String>"

  Scenario: codelet_core::work_units reads stateHistory into last_state_change_at
    Given rust/core/src/work_units.rs after RPC-016 lands
    When a developer reads the file source raw
    Then the file contains the substring "stateHistory"
    And the file contains the substring "last_state_change_at"

  Scenario: Action enum gains the four new viewport variants
    Given rust/fspec-tui/src/components/mod.rs after RPC-016 lands
    When a developer reads the file source raw
    Then the file contains the substring "ScrollFocusedColumnUp"
    And the file contains the substring "ScrollFocusedColumnDown"
    And the file contains the substring "SelectFirstInFocused"
    And the file contains the substring "SelectLastInFocused"

  Scenario: BoardStore declares the scroll_offsets field and viewport methods
    Given rust/fspec-tui/src/store/board.rs after RPC-016 lands
    When a developer reads the file source raw
    Then the file contains the substring "scroll_offsets"
    And the file contains the substring "pub fn scroll_offset_for"
    And the file contains the substring "pub fn set_scroll_offset_for"
    And the file contains the substring "pub fn move_selection"
    And the file contains the substring "pub fn scroll_focused_column"
    And the file contains the substring "pub fn select_first_in_focused"
    And the file contains the substring "pub fn select_last_in_focused"

  Scenario: Viewport painter module exists as a separate file
    Given the rust/fspec-tui crate after RPC-016 lands
    When a developer scans the views/board/ directory
    Then the file rust/fspec-tui/src/views/board/viewport.rs exists

  Scenario: New and modified board modules stay under 300 lines
    Given the directory rust/fspec-tui/src/views/board/ plus the views/board.rs orchestrator and store/board.rs
    When a test counts the line-count of every .rs file in views/board/ plus views/board.rs plus store/board.rs
    Then views/board.rs has fewer than 300 lines
    And store/board.rs has fewer than 300 lines
    And every .rs file under views/board/ has fewer than 300 lines

  Scenario: RPC-013 / RPC-014 / RPC-015 invariants preserved
    Given rust/fspec-tui/src/views/board.rs after RPC-016 lands
    When a developer reads the file source raw
    Then the file contains the substring "Action::EnterWorkUnit"
    And the file contains the substring "Action::FocusNextColumn"
    And the file contains the substring "Action::ReorderUp"
    And the file does NOT contain the identifier "FooterView"

  Scenario: Views still avoid encapsulated transport crates and host runtime construction
    Given the directory rust/fspec-tui/src/views/ (including views/board/)
    When a test scans every *.rs file
    Then NO file imports `codelet_napi::` or `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
    And NO file contains `tokio::runtime::Builder` or `Runtime::new()`

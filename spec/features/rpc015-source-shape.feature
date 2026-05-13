@done
@RPC-015
@rust
@tui
@board-view
@regression
@file-structure
@rpc
@header
Feature: RPC-015 source-shape regressions — header widgets + shared CheckpointCounts type

  """
  RPC-015 (slice 3 of 3) — Source-shape regressions pin the file layout and
  cross-crate invariants introduced by the BoardView header port:

    - CheckpointCounts shared type lives in codelet/rpc-types/src/lib.rs and
      carries `pub manual: u32` + `pub auto: u32` fields plus the napi cfg
      attribute for cross-binding parity.
    - The FspecService trait in codelet/rpc/src/lib.rs gains the new
      `async fn checkpoint_counts() -> CheckpointCounts` method.
    - The NAPI surface gains an additive `pub fn count_checkpoints` export
      in codelet/napi/src/git.rs that delegates to the shared
      `codelet_git::ghost_commit::count_checkpoints` helper.
    - The three new header widgets logo.rs / checkpoint_status.rs /
      keybinding_shortcuts.rs exist under codelet/fspec-tui/src/views/board/
      and stay under the 300 LoC ceiling.
    - The orchestrator codelet/fspec-tui/src/views/board.rs stays under
      300 LoC after the new layout split lands.
    - RPC-013 and RPC-014 source-shape invariants stay green.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a source-shape regression test that pins the RPC-015 module split, the new shared CheckpointCounts type, the new RPC method, and the persisting RPC-013/014 invariants
    So that future cards cannot silently merge the header widgets back into board.rs or strip the new fields

  Scenario: Header widget modules exist as separate files
    Given the codelet/fspec-tui crate after RPC-015 lands
    When a developer scans the views/board/ directory
    Then the file codelet/fspec-tui/src/views/board/logo.rs exists
    And the file codelet/fspec-tui/src/views/board/checkpoint_status.rs exists
    And the file codelet/fspec-tui/src/views/board/keybinding_shortcuts.rs exists

  Scenario: New and modified board modules stay under 300 lines
    Given the directory codelet/fspec-tui/src/views/board/ plus the views/board.rs orchestrator
    When a test counts the line-count of every .rs file in views/board/ plus views/board.rs
    Then views/board.rs has fewer than 300 lines
    And views/board/logo.rs has fewer than 300 lines
    And views/board/checkpoint_status.rs has fewer than 300 lines
    And views/board/keybinding_shortcuts.rs has fewer than 300 lines

  Scenario: CheckpointCounts shared type lives in rpc-types
    Given codelet/rpc-types/src/lib.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "pub struct CheckpointCounts"
    And the file contains the substring "pub manual: u32"
    And the file contains the substring "pub auto: u32"

  Scenario: FspecService trait gains the checkpoint_counts RPC method
    Given codelet/rpc/src/lib.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "async fn checkpoint_counts() -> CheckpointCounts"

  Scenario: FspecBackend trait gains the checkpoint_counts method
    Given codelet/fspec-tui/src/transport/mod.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "async fn checkpoint_counts"
    And the file contains the substring "CheckpointCounts"

  Scenario: Action enum gains CheckpointCountsLoaded variant
    Given codelet/fspec-tui/src/components/mod.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "CheckpointCountsLoaded"

  Scenario: NAPI surface exposes the additive count_checkpoints export
    Given codelet/napi/src/git.rs after RPC-015 lands
    When a developer reads the file source raw
    Then the file contains the substring "pub fn count_checkpoints"
    And the file contains the substring "codelet_git::ghost_commit::count_checkpoints"

  Scenario: RPC-013 / RPC-014 invariants preserved
    Given codelet/fspec-tui/src/views/navigator.rs after RPC-015 lands
    Then the file does NOT contain "Constraint::Length(1)"
    And codelet/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    And codelet/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    And the file codelet/fspec-tui/src/views/footer.rs does NOT exist
    And codelet/fspec-tui/src/views/board.rs still contains the substring "Action::EnterWorkUnit"
    And codelet/fspec-tui/src/views/board.rs still contains the substring "Action::FocusNextColumn"
    And codelet/fspec-tui/src/views/board.rs still contains the substring "Action::ReorderUp"

  Scenario: Views still avoid encapsulated transport crates and host runtime construction
    Given the directory codelet/fspec-tui/src/views/ (including views/board/)
    When a test scans every *.rs file
    Then NO file imports `codelet_napi::` or `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
    And NO file contains `tokio::runtime::Builder` or `Runtime::new()`

@done
@RPC-013
@rust
@tui
@infrastructure
@rpc
Feature: RPC-013 source-shape — Navigator restructure + FooterView deletion + AgentView layout
  """
  RPC-013 (slice 3 of 3) — Source-shape regression locking the structural
  invariants for the view-aware footer refactor:
  [1] rust/fspec-tui/src/views/footer.rs is DELETED.
  [2] `FooterView` identifier disappears from views/mod.rs, lib.rs, and
  app/state.rs (after comment stripping).
  [3] Navigator::render_with_stores no longer reserves a Length(1)
  footer row.
  [4] AgentView::render_with_store splits into Min(0) + Length(3) +
  Length(1) and paints the placeholder footer literal.
  [5] BoardView source carries the literal UnifiedBoardLayout footer
  string.
  [6] File-size invariant (< 300 LoC) preserved for every modified
  view file.

  Pair: tests live in rust/fspec-tui/tests/source_shape_rpc013.rs.
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want a source-shape regression that pins the structural invariants of the view-aware footer refactor
    So that future cards cannot accidentally re-introduce the deleted FooterView, the navigator's footer constraint, or break the file-size ceiling

  Scenario: Navigator no longer reserves a Length(1) footer row
    Given the Navigator render path in rust/fspec-tui/src/views/navigator.rs
    When a developer scans the render_with_stores method body
    Then the method does NOT contain "Constraint::Length(1)" anywhere
    And the method does NOT reference `self.footer`

  Scenario: AgentView splits its area into scrollback + input + footer rows
    Given an AgentView module at rust/fspec-tui/src/views/agent.rs
    When a developer scans the render_with_store method body
    Then the method contains a Layout split with a Min(0) flex row and a trailing Length(1) footer row
    And the bottom 1-row chunk is painted with the placeholder footer string "Enter=send  Ctrl+C=interrupt  ESC=back"

  Scenario: FooterView module and its re-exports are removed
    Given the rust/fspec-tui crate after RPC-013 lands
    When a developer scans the crate source tree
    Then the file rust/fspec-tui/src/views/footer.rs does NOT exist
    And rust/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    And rust/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    And rust/fspec-tui/src/app/state.rs does NOT contain the identifier "FooterView"

  Scenario: BoardView source contains the literal UnifiedBoardLayout footer string
    Given the BoardView module at rust/fspec-tui/src/views/board.rs
    When a developer scans the source after comment stripping
    Then the file contains the substring "← → Columns"
    And the file contains the substring "↑↓ Work Units"
    And the file contains the substring "[ Priority Up"
    And the file contains the substring "] Priority Down"
    And the file contains the substring "↵ Work Agent"
    And the file contains the substring "ESC Back"
    And the file does NOT contain the substring "? help"
    And the file does NOT contain the substring "switch pane"

  Scenario: File-size invariant preserved for every modified view file
    Given the directory rust/fspec-tui/src/views/
    When a test counts the line-count of every .rs file under that directory
    Then views/board.rs has fewer than 300 lines
    And views/agent.rs has fewer than 300 lines
    And views/navigator.rs has fewer than 300 lines
    And views/mod.rs has fewer than 300 lines

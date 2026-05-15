@done
@navigation
@tui
@RPC-023
Feature: RPC-023 mouse-port source-shape invariants

  """
  Source-shape regressions that pin:
    - the codelet/fspec-tui/src/mouse/ module layout,
    - absence of raw SGR mouse escape strings outside terminal.rs,
    - locality of EnableMouseCapture / DisableMouseCapture to
      terminal.rs + mouse/toggle.rs,
    - dialog-priority components remain Event::Key-only,
    - the 300 LoC ceiling per mouse file + views/board.rs,
    - Action enum gains the three new variants,
    - rect_contains half-open edge semantics.
  """

  Background: User Story
    As a maintainer of the Rust fspec TUI
    I want source-shape invariants enforced by tests
    So that future cards cannot silently regress the mouse-port architecture

  @source-shape
  Scenario: codelet/fspec-tui/src/mouse module exists with the expected files
    Given the fspec-tui crate after RPC-023 lands
    When a developer scans the src/ directory
    Then the file codelet/fspec-tui/src/mouse/mod.rs exists
    And the file codelet/fspec-tui/src/mouse/hit_test.rs exists
    And the file codelet/fspec-tui/src/mouse/toggle.rs exists

  @source-shape
  Scenario: No raw SGR mouse escape strings appear outside terminal.rs
    Given the directory codelet/fspec-tui/src
    When a test scans every .rs file with comments stripped
    Then no file contains the literal byte sequence "\x1b[?1000h"
    And no file contains the literal byte sequence "\x1b[?1006h"
    And no file contains the literal byte sequence "\x1b[?1006l"
    And no file contains the literal byte sequence "\x1b[?1000l"

  @source-shape
  Scenario: EnableMouseCapture and DisableMouseCapture appear only in terminal.rs and mouse/toggle.rs
    Given the directory codelet/fspec-tui/src
    When a test scans every .rs file with comments stripped for EnableMouseCapture / DisableMouseCapture identifiers
    Then the only files containing these identifiers are src/terminal.rs and src/mouse/toggle.rs

  @source-shape
  Scenario: Dialog-priority components match Event::Key exclusively
    Given the source of components/disconnect_dialog.rs and components/help_dialog.rs
    When a test scans the stripped source for Event::Mouse pattern arms
    Then neither file contains an Event::Mouse match arm

  @source-shape
  Scenario: Mouse module files and views/board.rs stay under 300 lines
    Given codelet/fspec-tui/src/mouse/*.rs and codelet/fspec-tui/src/views/board.rs after RPC-023 lands
    When a test counts the lines in each file
    Then each file has fewer than 300 lines

  @source-shape
  Scenario: Action enum gains the three new variants
    Given codelet/fspec-tui/src/components/mod.rs after RPC-023 lands
    When a developer reads the file source raw
    Then the file contains the substring "SetFocusedColumn"
    And the file contains the substring "SelectIndexInFocused"
    And the file contains the substring "ReEnableMouseTracking"

  Scenario: rect_contains is half-open on the right and bottom edges
    Given a Rect with x=5, y=5, width=10, height=10
    When rect_contains is evaluated for several points
    Then rect_contains returns true for (5, 5)
    And rect_contains returns true for (14, 14)
    And rect_contains returns false for (15, 14)
    And rect_contains returns false for (14, 15)
    And rect_contains returns false for (4, 5)

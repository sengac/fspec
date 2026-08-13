@done
@RPC-012
@rust
@tui
@infrastructure
@rpc
Feature: RPC-012 BoardStore — work-units grouped into 7 columns with focus + selection
  """
  RPC-012 — BoardStore unit-level contract. Plain owned Rust struct in
  rust/fspec-tui/src/store/board.rs; mutated only on the App task per
  the RPC-009 single-task tenere pattern.

  TS reference: src/tui/store/fspecStore.ts (work-units list + status
  grouping + session_attachments map) and src/tui/components/UnifiedBoardLayout.tsx
  (canonical 7-column STATES order).
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want a plain owned BoardStore that groups work units by status into 7 columns and tracks per-column selection
    So that future BoardView / Navigator / App::dispatch code can read selection + grouping without any Mutex/RwLock

  Scenario: BoardStore seeds work units grouped into 7 columns with focus at backlog
    Given a freshly constructed BoardStore via BoardStore::default()
    When the developer calls store.replace_work_units with [AUTH-001 backlog, AUTH-002 implementing, AUTH-003 done]
    Then store.column_units("backlog") returns exactly [AUTH-001]
    And store.column_units("implementing") returns exactly [AUTH-002]
    And store.column_units("done") returns exactly [AUTH-003]
    And store.focused_column() returns "backlog"
    And store.selected_index_for("backlog") returns 0

  Scenario: BoardStore re-grouping preserves focus and clamps selection when columns shrink
    Given a BoardStore seeded with [AUTH-001 backlog, AUTH-002 implementing, AUTH-003 done]
    And store.focused_column() returns "implementing"
    And store.selected_index_for("implementing") returns 0
    When store.replace_work_units is called with [AUTH-001 backlog, AUTH-002 validating, AUTH-003 done]
    Then store.column_units("validating") returns exactly [AUTH-002]
    And store.column_units("implementing") returns an empty slice
    And store.focused_column() still returns "implementing"
    And store.selected_index_for("implementing") returns 0

@done
@diff-viewer
@tui
@RPC-363
Feature: Refactor: lift changed_files diff/row/scrollbar helpers into a shared diff-viewer module
  """
  Shared module at views/diff_common/ (mod.rs + diff_render.rs + row.rs) exposes pub diff_line, classify, file_row, status_color, truncate_path. Pane-scrollbar gutter helper render_pane_scrollbar lives here and delegates to components::list_scrollbar::render_list_scrollbar. views/changed_files imports these — no duplicated diff/row logic. Pure refactor: byte-identical colors/layout, all changed_files tests stay green (18/18).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. diff_line, classify, file_row, status_color and truncate_path live in one shared module exposed as pub
  #   2. The pane-scrollbar gutter helper lives in the shared module and delegates to list_scrollbar
  #   3. changed_files imports the shared helpers and contains no duplicated diff/row logic
  #   4. The refactor changes no observable behavior: all existing changed_files scenarios stay green
  #
  # EXAMPLES:
  #   1. diff_line classifies a +added line as green, a -removed line as red, and an @@ hunk header as dim/cyan from the shared module
  #   2. file_row from the shared module shows a > cursor on the selected row and truncates a long path with an ellipsis
  #   3. After the refactor the full changed_files lib suite still reports 18/18 scenarios covered and all unit tests pass
  #
  # ========================================
  Background: User Story
    As a fspec TUI developer
    I want to import diff-line, file-row and pane-scrollbar helpers from one shared module
    So that both ChangedFilesView and CheckpointsView reuse identical rendering without duplication

  Scenario: Shared diff_line colors added green removed red and hunk dim/cyan
    Given the shared diff-viewer module exposes diff_line as a public helper
    When I build diff lines for a +added line, a -removed line, and an @@ hunk header
    Then the added line is green, the removed line is red, and the hunk header is dim cyan

  Scenario: Shared file_row shows cursor on selected row and truncates long path
    Given the shared diff-viewer module exposes file_row, status_color and truncate_path as public helpers
    When I build a file_row for a selected file with a long path in a narrow pane
    Then the row begins with a > cursor and the path is truncated with an ellipsis

  Scenario: changed_files reuses the shared pane-scrollbar helper that delegates to list_scrollbar
    Given the shared diff-viewer module exposes a pane-scrollbar helper that delegates to list_scrollbar
    When I render a pane-scrollbar gutter for a list that overflows its pane
    Then a proportional scrollbar thumb is painted in the gutter column

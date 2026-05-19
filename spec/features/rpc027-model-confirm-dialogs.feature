@done
@refactor
@rust
@ui-refinement
@tui
@dialog
@rpc
@RPC-027
Feature: RPC-027 — ModelSelectorDialog and ConfirmDialog migration

  """
  RPC-027 Sections E (ModelSelectorDialog) and F (ConfirmDialog).
  Both consume the shared dialog_theme.rs renderer.
  """

  Background: User Story
    As a developer maintaining the codelet/fspec-tui Rust ratatui frontend
    I want ModelSelectorDialog and ConfirmDialog to use the canonical theme
    So that the inverse selection highlight and yellow button focus match the TypeScript reference

  # ============================================================
  # Section E — ModelSelectorDialog
  # ============================================================

  Scenario: ModelSelectorDialog renders with the cyan accent and "Select Model" inner title
    Given a ModelSelectorDialog seeded with a non-empty provider list
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Cyan
    And the body's first non-padding row contains the text "Select Model"
    And the title cells have foreground color Color::Cyan with BOLD modifier

  Scenario: ModelSelectorDialog applies the inverse highlight only to selectable rows
    Given a ModelSelectorDialog seeded with two providers each having two models
    When I render it onto an 80x24 TestBackend buffer with selected_index pointing at a model row
    Then the selected model row has background Color::Cyan and foreground Color::Black
    And the provider header rows render with the default background and no highlight
    And the selected row begins with the two-character marker "▸ "
    And the other model rows begin with the two-character marker "  "

  Scenario: ModelSelectorDialog renders capability badges with the DIM modifier
    Given a ModelSelectorDialog row whose model has reasoning, vision, and a 200k context
    When I render it onto an 80x24 TestBackend buffer
    Then the badge segment "[R] [V] [200k]" appears after the model name
    And every cell of the badge segment carries Modifier::DIM (for unselected rows)

  Scenario: ModelSelectorDialog footer includes the "Custom models" notice and navigation hints
    Given a ModelSelectorDialog seeded with a non-empty provider list
    When I render it onto an 80x24 TestBackend buffer
    Then the footer contains the line "↑↓ Navigate │ Enter Select │ Esc Close"
    And the footer contains the line "Custom models: not yet supported"
    And every footer cell carries Modifier::DIM

  # ============================================================
  # Section F — ConfirmDialog
  # ============================================================

  Scenario: ConfirmDialog renders with the yellow accent and caller-supplied title
    Given a ConfirmDialog with title "Delete Session" and body "Delete this session?"
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Yellow
    And the body's first non-padding row contains the text "Delete Session"
    And the title cells have foreground color Color::Yellow with BOLD modifier
    And the source no longer uses Block::default().borders(Borders::ALL)

  Scenario: ConfirmDialog button row uses inverse highlight on the focused button
    Given a ConfirmDialog with primary "Delete", secondary "Archive", cancel "Cancel" and focused index 0
    When I render it onto an 80x24 TestBackend buffer
    Then the " Delete " span has background Color::Yellow and foreground Color::Black with BOLD modifier
    And the " Archive " and " Cancel " spans have default Style
    And the spans are separated by " │ "

  Scenario: ConfirmDialog Left and Right cycle button focus
    Given a ConfirmDialog with three buttons and focused index 0
    When I send KeyCode::Right
    Then focused is 1
    When I send KeyCode::Right
    Then focused is 2
    When I send KeyCode::Right
    Then focused is 0
    When I send KeyCode::Left
    Then focused is 2

  Scenario: ConfirmDialog Esc returns Cancel from any focused index
    Given a ConfirmDialog with three buttons and focused index 1
    When I send KeyCode::Esc
    Then the outcome is ConfirmDialogOutcome::Cancel

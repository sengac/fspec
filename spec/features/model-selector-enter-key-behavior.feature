@done
@model-selector
@tui
@PROV-117
Feature: Enter on a collapsed section header does not expand it in the /model view

  """
  Enter handling lives in ModelSelectorView::handle_key (codelet/fspec-tui/src/views/model_selector/dispatch.rs). TS reference: ModelSelectorScreen.tsx:203-210 — Enter on a 'section' item calls toggleSectionExpansion(providerId); Enter on a 'model' item builds a selection (selectModel) then onSelectModel + onClose. Rust parity: when the focused row is non-selectable (a provider/profile header) Enter toggles expansion via toggle_expansion(!is_expanded(key)); when the focused row is selectable Enter emits Action::ModelSelected gated by PROV-101 has_selection and a present session_id. Headers are non-selectable (model_selector_dialog_rows.rs); selectable model rows carry a model_id. Left/Right continue to collapse/expand explicitly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Enter on a non-selectable provider/profile header toggles its expansion (expand if collapsed, collapse if expanded), mirroring TS toggleSectionExpansion
  #   2. Enter on a selectable model row still emits Action::ModelSelected and closes the selector (PROV-101 has_selection + session guards apply only to this selection path)
  #   3. Expanding a section via Enter reveals its model rows so a subsequent navigation + Enter can select a model
  #
  # EXAMPLES:
  #   1. On a fresh collapse-by-default open, the cursor sits on a collapsed provider header; pressing Enter expands that section and its models appear
  #   2. Pressing Enter again on an already-expanded provider header collapses it (toggle)
  #   3. After expanding a section with Enter, pressing Down to a model row and pressing Enter selects that model and closes the view
  #
  # ========================================

  Background: User Story
    As a fspec TUI user choosing a model in the /model view
    I want to press Enter on a collapsed provider section to expand it, then press Enter on a model to select it
    So that I can reach and pick a model using Enter alone, matching the TypeScript UI

  Scenario: Pressing Enter on a collapsed provider header expands the section
    Given the /model view is open with a provider section that is collapsed and the cursor is on its header
    When I press Enter
    Then the section expands and its model rows become visible


  Scenario: Pressing Enter again on an expanded provider header collapses the section
    Given the /model view is open with a provider section that is expanded and the cursor is on its header
    When I press Enter
    Then the section collapses and its model rows are hidden


  Scenario: Pressing Enter on a selectable model row selects the model and closes the view
    Given the /model view is open for an active session and the cursor is on a selectable model row
    When I press Enter
    Then a model selection is emitted for the current session, provider and model
    And the model selector view closes


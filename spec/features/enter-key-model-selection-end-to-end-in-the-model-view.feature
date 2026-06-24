@done
@PROV-117
@tui
@model-selector
@e2e
Feature: Enter key model selection end-to-end in the /model view
  """
  End-to-end tier (@microsoft/tui-test): launches the real fspec binary in a PTY (HOME redirected to a temp fspec-config.json with an openai local-server profile carrying customModels incl. a date-suffixed id), drives /model with real keystrokes, and asserts the selection actually applies. Root cause #1 (PROV-117): TS createModelSelection sends modelId=extractModelIdForRegistry(model.id) (strips -YYYYMMDD) for registry/marker comparison while keeping apiModelId=model.id raw for the API call. Rust compared verbatim at 3 sites (state.rs:51 auto-expand, rows.rs:149 cursor seed, rows_render.rs:166 (current) marker), so a normalized current id never matched a dated row id -> cursor never seeded -> has_selection stayed false -> Enter no-op. Fix: model_selector/model_id.rs::model_ids_match normalizes BOTH operands at the compare sites; Action::ModelSelected shape and the raw id to ProviderManager::select_model are unchanged so the API still gets the dated id.
  """

  Background: User Story
    As a fspec TUI user choosing a model in the /model view
    I want to press Enter on a model row and have that model actually selected and applied
    So that the model sticks (the view closes, the agent reflects it, and reopening shows it as current) even for date-suffixed cloud model ids

  Scenario: End-to-end: pressing Enter on a model row selects the model and the selector closes against the real binary
    Given the fspec binary is launched with a temp HOME config whose openai local-server profile carries custom models and the /model view is open in an active session
    When I press Enter
    Then the model selector view closes and returns to the agent view
    And I expand a provider section and move the cursor onto a selectable model row
    And the chosen model is applied so the agent view reflects it and reopening /model shows the (current) marker on that model

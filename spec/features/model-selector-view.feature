@model-selection
@done
@ts-parity
@rust
@model-selector
@tui
@RPC-337
@BUG-300
@BUG-508
Feature: Model selector view interaction
  """
  The model selector is a full-screen Navigator mode-view (ViewMode::ModelSelector), replacing the Compositor modal. /model dispatches Action::OpenModelSelectorView (and spawns list_providers); Esc returns to Agent. Provider header rows stay non-selectable; navigation (up/down, PageUp/Down, Home/End, mouse-wheel) skips them with wrap-around. Selecting a model row with Enter emits Action::ModelSelected(Option<session_id>, provider_key, model_id) and returns to the prior view. Following the TypeScript implementation (ModelSelectorScreen.tsx Enter handler + modelSelectionService.selectModel), the Enter handler has NO session-existence guard: the selection is always emitted and the view always closes; only the downstream backend write (set_session_model) and the green (current) marker are gated on a present session. With no active session the selector still closes but no backend write fires. The mode-view supports filter (/), expand/collapse of provider groups (left/right arrows) and refresh (r); footer reads 'Enter Select | <-> Expand/Collapse | / Filter | r Refresh | Esc Close'.
  """

  Background: User Story
    As a fspec TUI user
    I want to open the model selector as a full-screen view and drive it with the keyboard
    So that it matches the original TypeScript ModelSelectorView UX

  Scenario: Open the model selector full-screen via the slash command
    Given I am in the Agent view
    When I run the "/model" slash command
    Then the model selector replaces the screen as a full-screen view
    And the title reads "Select Model (N models)"
    And the provider list is requested asynchronously

  Scenario: Close the model selector with Esc returns to Agent
    Given I am in the model selector mode-view
    When I press Esc
    Then the model selector closes
    And I am returned to the Agent view

  Scenario: Navigation moves through provider headers
    Given the model selector shows a provider header followed by model rows
    And the cursor is on the last model row above a provider header
    When I press the down arrow
    Then the cursor moves to the next row (the provider header)
    And the cursor rests on the non-selectable provider header (TS parity, PROV-104: no header-skipping)

  Scenario: Selecting a model with an active session commits the choice
    Given the model selector is open with an active session
    And the cursor is on the model row "claude-sonnet [R] [V] [200k]"
    When I press Enter
    Then a model selection is emitted for the current session, provider and model
    And the model selector view closes

  Scenario: The session header badge shows the model chosen in the model selector
    Given an agent session whose header shows the model "claude-haiku"
    And the model selector is open with that session and the cursor on the model row "claude-sonnet"
    When I press Enter and the backend accepts the model
    Then the model selector closes back to the agent view
    And the session header row shows "claude-sonnet"
    And the session header row no longer shows "claude-haiku"

  Scenario: Selecting a model with no active session still emits the selection
    Given the model selector is open with no current session
    And the cursor is on a selectable model row
    When I press Enter
    Then a model selection is emitted with no session id
    And the model selector view closes

  Scenario: Filtering narrows the model list
    Given the model selector is showing all providers and models
    When I press "/" and type filter text
    Then the list narrows to models matching the filter
    And clearing the filter restores the full list

  Scenario: Expanding and collapsing a provider group
    Given the model selector shows an expanded provider group
    When I press the left arrow on the provider group
    Then the group collapses and hides its model rows
    When I press the right arrow on the provider group
    Then the group expands and shows its model rows

  Scenario: Overflowing list shows scroll indicators and wheel navigates
    Given the model list overflows the viewport
    When the list is rendered
    Then a scrollbar column shows the scroll position beside the list
    When I scroll the mouse wheel down
    Then the selection advances skipping provider headers

  Scenario: Refreshing the model list
    Given the model selector is open
    When I press "r"
    Then the provider's models are refreshed
    And the title shows "(refreshing...)" while the refresh is in flight
    And the list updates once the refreshed models arrive

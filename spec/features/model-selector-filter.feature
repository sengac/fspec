@done
@filtering
@tui
@MODEL-007
Feature: Model selector view never renders the filter input row

  """
  The filter row is carved off the TOP of body_area inside the browse-list body closure in model_selector/render.rs, BEFORE visible_rows is computed. When (filter_mode || !filter.is_empty()) && body_area.height > 0, a 1-line Rect is taken from the top of body_area and a Paragraph renders 'Filter: {}_' (filter_mode active, trailing cursor) or 'Filter: {}' (committed). body_area is then shrunk (y += 1, height -= 1). visible_rows is computed from the reduced body_area.height (saturating_sub(1)), so the filter line is reserved and no model row is hidden. This mirrors provider_settings/list.rs:200-223. State fields: filter: String, filter_mode: bool on ModelSelectorView.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the filter is active (filter_mode), a "Filter: <text>_" row with a trailing cursor underscore is rendered at the top of the body while typing
  #   2. When a filter is committed (not in filter_mode) but non-empty, the row renders "Filter: <text>" without the trailing cursor
  #   3. When there is no filter (empty and not in filter_mode), no filter row is rendered and the full body height is used for model rows
  #   4. visible_rows / scroll math reserves the filter line (body height reduced by 1) so no model row is hidden behind the prompt
  #   5. Yes. ModelSelectorView has the same two fields as ProviderSettingsView: `filter: String` (mod.rs:75) and `filter_mode: bool` (mod.rs:76). crud.rs manages both (clear/pop/push filter, set filter_mode). The render should mirror provider_settings/list.rs:204-216: show the row when (filter_mode || !filter.is_empty()), with trailing '_' only when filter_mode is true.
  #
  # EXAMPLES:
  #   1. User opens /model, presses filter key and types "opus" (filter_mode active) → top body row shows "Filter: opus_" and the list shows only matching models
  #   2. User commits the filter (leaves filter_mode) with filter still "opus" → top body row shows "Filter: opus" with no trailing underscore
  #   3. No filter active (filter empty, filter_mode false) → no filter row is rendered and the model list starts at the very top of the body
  #   4. With a filter row present, visible_rows equals (body_area.height - 1 for the filter line - 1 existing reservation), so a model row that would otherwise fill the last line is not hidden behind the prompt
  #
  # QUESTIONS (ANSWERED):
  #   Q: @self: Does the model selector distinguish filter_mode (actively typing) vs committed filter like the provider view?
  #   A: Yes. ModelSelectorView has the same two fields as ProviderSettingsView: `filter: String` (mod.rs:75) and `filter_mode: bool` (mod.rs:76). crud.rs manages both (clear/pop/push filter, set filter_mode). The render should mirror provider_settings/list.rs:204-216: show the row when (filter_mode || !filter.is_empty()), with trailing '_' only when filter_mode is true.
  #
  # ========================================

  Background: User Story
    As a user of the Rust TUI model selector
    I want to see a "Filter: <text>_" prompt showing what I've typed
    So that I know the active filter, matching the provider settings view

  Scenario: Active filter renders prompt with trailing cursor
    Given the model selector is in browse mode with models loaded
    When the model selector view is rendered
    Then the top row of the body shows "Filter: opus_"
    And filter mode is active and the filter text is "opus"
    And only models matching "opus" are listed below the prompt


  Scenario: Committed non-empty filter renders prompt without cursor
    Given the model selector is in browse mode with models loaded
    When the model selector view is rendered
    Then the top row of the body shows "Filter: opus" without a trailing underscore
    And filter mode is not active but the filter text is "opus"


  Scenario: No filter renders no prompt row
    Given the model selector is in browse mode with models loaded
    When the model selector view is rendered
    Then no "Filter:" row is rendered
    And filter mode is not active and the filter text is empty
    And the model list starts at the very top of the body


  Scenario: Filter row reserves a line so no model row is hidden
    Given the model selector is in browse mode with models loaded
    When the model selector view is rendered into a fixed-height buffer
    Then visible_rows is reduced by one to reserve the filter line
    And a filter row is present because filter mode is active
    And no model row is hidden behind the filter prompt


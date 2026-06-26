@done
@model-selection
@provider-settings
@rust
@tui
@RPC-353
Feature: Mouse wheel + Page/Home/End scroll missing for /provider and /model views

  """
  Navigator routes Event::Mouse into a handle_mouse on the active mode-view (navigator_events.rs). /model's existing handle_mouse (model_selector/dispatch.rs) becomes live; /provider gains a handle_mouse. Both use the shared WheelVelocity 1x-5x ramp from components/scroll_viewport.rs. /provider list.rs handle_list_key gains PageUp/PageDown/Home/End (filter_mode=false only) reusing view.move_clamped. The chat/agent view is the unchanged reference pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Navigator forwards Event::Mouse to the active mode-view (handle_provider_settings_event / handle_model_selector_event route mouse into a handle_mouse) instead of dropping it
  #   2. Mouse wheel ScrollUp moves the selection toward the top and ScrollDown toward the bottom on both /model and /provider, then adjusts scroll
  #   3. Wheel scrolling uses the shared WheelVelocity 1x-5x ramp so rapid wheel events move multiple rows per event (same feel as the chat view)
  #   4. /provider List mode binds PageUp/PageDown to move selection by one visible_rows page (clamped, no wrap) and Home/End to first/last item, then adjusts scroll
  #   5. /provider filter mode must not hijack Page/Home/End — those keys only apply when filter_mode is false; printable-char accumulation is unchanged
  #   6. /model's existing Page/Home/End and arrow/Enter/Tab behaviour is preserved (regression guard); the chat/agent view is unchanged
  #
  # EXAMPLES:
  #   1. A ScrollDown mouse event routed through the navigator while /provider is active moves the provider selection downward
  #   2. A ScrollDown mouse event routed through the navigator while /model is active moves the model selection downward
  #   3. Rapid successive ScrollDown wheel events move the /provider selection by more than one row (velocity ramp engaged)
  #   4. Pressing PageDown then PageUp on /provider moves the selection down a page then back up a page (clamped at ends)
  #   5. Pressing End on /provider selects the last item and Home selects the first item
  #   6. While /provider filter mode is active, pressing PageDown does not move the selection (paging keys are inert in filter mode)
  #
  # ========================================

  Background: User Story
    As a fspec-tui user on the /provider and /model views
    I want to scroll the list with the mouse wheel and Page/Home/End keys
    So that I get the same scroll affordances and feel as the chat view

  Scenario: Mouse wheel scrolls the provider list
    Given the /provider view is active with a list longer than the viewport
    When a ScrollDown mouse event is routed through the navigator
    Then the provider selection moves downward

  Scenario: Mouse wheel scrolls the model list
    Given the /model view is active with a list longer than the viewport
    When a ScrollDown mouse event is routed through the navigator
    Then the model selection moves downward

  Scenario: Rapid wheel events ramp the scroll velocity
    Given the /provider view is active with a list longer than the viewport
    When several ScrollDown wheel events arrive in rapid succession
    Then the provider selection moves by more than one row

  Scenario: PageDown and PageUp page the provider list
    Given the /provider view is active with a list longer than the viewport
    When I press PageDown and then PageUp
    Then the selection moves down a page and then back up a page

  Scenario: Home and End jump to the ends of the provider list
    Given the /provider view is active with a list longer than the viewport
    When I press End and then Home
    Then End selects the last item and Home selects the first item

  Scenario: Paging keys are inert in provider filter mode
    Given the /provider view is active and filter mode is on
    When I press PageDown
    Then the provider selection does not move

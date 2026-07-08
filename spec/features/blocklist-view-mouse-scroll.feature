@done
@rust
@tui-component
@agent-view
@tui
@BLOCK-011
Feature: BlocklistView: add mouse-wheel scroll support (parity with model_selector/provider_settings wheel handling)
  """
  BlocklistView gains a wheel: scroll_viewport::WheelVelocity field and a handle_mouse(MouseEvent) -> BlocklistEvent method (views/blocklist/mod.rs) that maps MouseEventKind::ScrollUp/ScrollDown to WheelDirection and repeats move_up/move_down by self.wheel.step(dir); non-wheel kinds return Ignored. move_up/move_down already reconcile scroll_offset via adjust_scroll (shared scroll_viewport::ensure_visible). navigator_events::handle_blocklist_event routes Event::Mouse into handle_mouse before the Event::Key guard, mirroring handle_model_selector_event. The WheelVelocity derive conflict with BlocklistView's #[derive(Debug, Clone, Default)] is resolved by deriving Debug+Clone on WheelVelocity (additive) or a manual Default impl on the view. Mirrors model_selector and provider_settings wheel handling; no new scroll math.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Scrolling the wheel down moves the selection down by the WheelVelocity step count, clamped to the last rule
  #   2. Scrolling the wheel up moves the selection up by the WheelVelocity step count, clamped to the first rule
  #   3. Rapid consecutive wheel events in the same direction accelerate the step count via the shared 1x-5x WheelVelocity ramp
  #   4. Non-wheel mouse events (move, click, drag) are ignored and do not change the selection
  #   5. The navigator routes Event::Mouse into BlocklistView::handle_mouse and treats a Consumed outcome as consumed, otherwise ignored
  #
  # EXAMPLES:
  #   1. With 20 rules and the selection at index 0, a single wheel-down event moves the selection to index 1 and reconciles the scroll window
  #   2. With the selection at the last rule (index 19), a wheel-down event leaves the selection at index 19 (clamped)
  #   3. With the selection at index 0, a wheel-up event leaves the selection at index 0 (clamped)
  #   4. A mouse move event over the blocklist view leaves the selection unchanged and is reported as ignored
  #   5. A wheel event delivered through the navigator's blocklist event handler is consumed and updates the view's selection
  #
  # ========================================
  Background: User Story
    As a mouse user of the Rust TUI /blocklist view
    I want to scroll the blocklist rules with the mouse wheel
    So that I can browse a long rules list the same way I do in the model and provider views

  Scenario: Wheel down moves the selection down
    Given the blocklist view has 20 rules loaded
    And the selection is at index 0
    When I scroll the mouse wheel down once
    Then the selection moves to index 1
    And the visible window scrolls to keep the selection visible

  Scenario: Wheel down clamps at the last rule
    Given the blocklist view has 20 rules loaded
    And the selection is at the last rule index 19
    When I scroll the mouse wheel down once
    Then the selection stays at index 19

  Scenario: Wheel up clamps at the first rule
    Given the blocklist view has 20 rules loaded
    And the selection is at index 0
    When I scroll the mouse wheel up once
    Then the selection stays at index 0

  Scenario: Non-wheel mouse events are ignored
    Given the blocklist view has 20 rules loaded
    And the selection is at index 5
    When I move the mouse over the view
    Then the selection stays at index 5
    And the mouse event is reported as ignored

  Scenario: The navigator routes wheel events to the view
    Given the blocklist view has 20 rules loaded
    And the selection is at index 0
    When a mouse wheel down event is delivered through the navigator
    Then the navigator reports the event as consumed
    And the selection moves down

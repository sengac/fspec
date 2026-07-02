@done
@provider-settings
@rust
@tui
@RPC-352
Feature: Provider settings list-mode has no scrollbar (TS + /model parity)
  """
  The /provider List view renders via body_render.rs -> list::render_list -> list_nav_render::render_nav_items. A shared scrollbar painter (components::list_scrollbar::render_list_scrollbar) is reused by both /provider and /model (model_selector/rows_render.rs). Render-only change: scroll-state logic (adjust_scroll/ensure_visible) is unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the provider nav-item list overflows the viewport (nav_items.len() > visible_rows), the List view renders a 1-cell-wide scrollbar column on the right edge of the body area
  #   2. The scrollbar uses a proportional DIM ■ thumb over a DIM │ track, with thumb_h = ((visible_rows * h) / total).max(1) and thumb_pos = (scroll_offset * h) / total — matching /model exactly
  #   3. When the list fits the viewport (no overflow), no scrollbar is drawn and the list keeps the full body width
  #   4. When the scrollbar is drawn, the list content width shrinks by 1 column (list_width = body_area.width - 1) so rows never paint under the scrollbar
  #   5. Both /model and /provider paint via a shared scrollbar painter; /model's rendered output stays byte-identical (existing /model tests still pass)
  #
  # EXAMPLES:
  #   1. Given 30 providers in a 10-content-row viewport, scrolled down, the rendered body contains a ■ thumb and │ track column
  #   2. Given 3 providers in a tall viewport (no overflow), no ■ or │ scrollbar column is painted and rows use the full width
  #   3. Given an overflowing provider list, every visible content row still paints its provider text beside the scrollbar (no row stolen)
  #   4. Scrolling the provider list down moves the ■ thumb to a lower row than at scroll_offset 0
  #
  # ========================================
  Background: User Story
    As a fspec-tui user on the /provider list view
    I want to see a proportional scrollbar when the provider list overflows
    So that I have the same visual scroll feedback as the /model view

  Scenario: Overflowing provider list paints a scrollbar column
    Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    And the list is scrolled down away from the top
    When the List body is rendered
    Then a scrollbar column is painted beside the list

  Scenario: A fitting provider list paints no scrollbar
    Given a provider nav-item list of 3 items in a viewport tall enough to show them all
    When the List body is rendered
    Then no scrollbar column is painted
    And the provider rows use the full body width

  Scenario: The scrollbar steals no content row
    Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    And the list is scrolled down away from the top
    When the List body is rendered
    Then every visible content row still paints its provider text beside the scrollbar

  Scenario: Scrolling moves the scrollbar thumb
    Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    When the list is scrolled down
    Then the thumb is painted on a lower row than at the top of the list

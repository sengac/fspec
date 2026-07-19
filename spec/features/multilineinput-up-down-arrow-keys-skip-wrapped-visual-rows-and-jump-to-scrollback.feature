@RPC-429
Feature: MultiLineInput Up/Down arrow keys skip wrapped visual rows and jump to scrollback

  """
  Fix uses existing functions from multiline_wrap.rs: total_visual_rows() and cursor_visual_position(). MultiLineInput needs a cached last_body_width field updated by sync_viewport/render. The boundary check in handle_key_gated replaces logical row/line_count with visual row/total_visual_rows.
  """

  Background: User Story
    As a user typing a long message
    I want to navigate between wrapped visual rows with Up/Down arrow keys
    So that I can edit the message at the correct visual line without jumping to scrollback

  Scenario: Down arrow returns Continued when cursor is not at visual bottom
    Given the input contains a wrapped string with multiple visual rows
    And the cursor is on visual row 3
    When I press Down
    Then the event outcome is Continued
    And the cursor moves to a later visual row

  Scenario: Up arrow returns Continued when cursor is not at visual top
    Given the input contains a wrapped string with multiple visual rows
    And the cursor is on visual row 2
    When I press Up
    Then the event outcome is Continued
    And the cursor moves to an earlier visual row

  Scenario: Up arrow at visual top returns Ignored for scrollback navigation
    Given the input contains a wrapped string with multiple visual rows
    And the cursor is on visual row 0
    When I press Up
    Then the event outcome is Ignored
    And the cursor remains on visual row 0

  Scenario: Down arrow at visual bottom returns Ignored for scrollback navigation
    Given the input contains a wrapped string with multiple visual rows
    And the cursor is on visual row 4 (the last visual row)
    When I press Down
    Then the event outcome is Ignored
    And the cursor remains on visual row 4

  Scenario: Up arrow on a single visual row returns Ignored for scrollback navigation
    Given the input contains a short string that fits on one visual row
    And the cursor is on visual row 0
    When I press Up
    Then the event outcome is Ignored
    And the cursor remains on visual row 0

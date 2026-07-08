@done
@ui-enhancement
@model-selector
@tui
@MODEL-008
Feature: Model count in title renders in flat style instead of dim two-span style
  """
  The model browse-list title is painted via a two-span title mapping identical to the provider view: name span "Select Model" in bold Color::Yellow, and the count span " (N models)" in dim Color::DarkGray. The count N is state.total_model_count(). This replaces the previous flat single-string title_text() rendered through render_full_screen_scaffold_raw_title (cyan-bold whole string). The fix routes the browse-list title through render_full_screen_scaffold_with_title, whose title closure calls render_two_span_title(area, buf, "Select Model", count, "models") (mode_view_render.rs:41-57). The custom-model overlay path (Add/Edit/Delete) continues to use render_full_screen_scaffold_raw_title and is untouched. render_title_with_count (blue-bold) used by ResumeSession/SearchHistory is a separate helper and is unaffected (RPC-350 R5 guard). Any " (refreshing...)" status suffix is a dim annotation and never joins the bold name span.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The model browse-list title renders "Select Model" in bold-yellow and the count "(N models)" in dim DarkGray, matching the provider view's two-span style
  #   2. The count value (N) equals the total number of models currently listed
  #   3. Custom-model overlay titles (Add/Edit/Delete Custom Model) are unaffected and keep their existing overlay style
  #   4. The shared blue-bold title used by other views (ResumeSession/SearchHistory) is unaffected
  #
  # EXAMPLES:
  #   1. 12 models available renders the title as bold-yellow "Select Model" followed by dim DarkGray " (12 models)"
  #   2. 1 model available renders the count as dim DarkGray " (1 model)" (singular noun)
  #   3. Opening the Add Custom Model overlay still shows "Add Custom Model" in its existing overlay (cyan-bold) style
  #
  # QUESTIONS (ANSWERED):
  #   Q: When the model list is refreshing, the current title appends " (refreshing...)". Should that suffix remain part of the dim count span or the bold name span?
  #   A: The refresh suffix, when preserved, is a dim status annotation and belongs with the dim count span (never the bold name). The core fix scopes render_two_span_title to name="Select Model", count=<total>, suffix="models"; the refreshing state remains a dim-styled trailing annotation, not part of the bold name.
  #
  # ========================================
  Background: User Story
    As a user of the Rust TUI model selector
    I want to see the model count in the title rendered in the same dim two-span style as the provider view
    So that the UI is consistent and readable across sibling views

  Scenario: Model browse-list title renders count in dim two-span style
    Given the model selector is in browse mode with 12 models available
    When the browse list is rendered
    Then the title shows "Select Model" in bold yellow
    And the count " (12 models)" is shown in dim DarkGray

  Scenario: Single model renders singular noun in dim count span
    Given the model selector is in browse mode with 1 model available
    When the browse list is rendered
    Then the count " (1 model)" is shown in dim DarkGray using the singular noun

  Scenario: Custom-model overlay title is unaffected by the two-span title change
    Given the model selector has the Add Custom Model overlay open
    When the overlay is rendered
    Then the title shows "Add Custom Model" in its existing overlay style with no dim DarkGray count span

  Scenario: Refreshing state renders the refresh suffix in the dim count span
    Given the model selector is in browse mode with 3 models available
    When the browse list is rendered
    Then the "(refreshing...)" suffix is shown in the dim DarkGray count span
    And a refresh is in flight
    And the bold-yellow "Select Model" name span never contains the refresh suffix

  Scenario: Browse-list title uses the two-span style, not the shared blue-bold title
    Given the model selector is in browse mode with 3 models available
    When the browse list is rendered
    Then the "Select Model" name span is bold yellow, not the shared blue-bold title style

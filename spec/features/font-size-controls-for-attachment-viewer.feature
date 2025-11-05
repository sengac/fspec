@done
@server
@ui-enhancement
@high
@tui
@web
@attachment-viewer
@TUI-021
Feature: Font size controls for attachment viewer
  """
  Font size state stored in localStorage via vanilla JS (no framework needed for server-rendered HTML). Plus/minus buttons trigger JavaScript that updates CSS custom property --code-font-size applied to code blocks. Default size 16px (increased from current small size). Min 10px, max 24px, increment 2px. Controls positioned in top right next to existing dark mode toggle.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Code blocks must have significantly larger base font size than current implementation
  #   2. Font size controls must be in top right corner next to dark mode toggle
  #   3. Font size must be persisted in localStorage
  #   4. Font size controls must use plus/minus buttons with displayed pixel size
  #   5. Font size must have min/max bounds (10px - 24px) with 2px increment/decrement
  #
  # EXAMPLES:
  #   1. User opens markdown attachment and code blocks display at 16px (larger than current)
  #   2. User clicks plus button and font increases from 16px to 18px
  #   3. User clicks minus button at 10px and size stays at 10px (minimum bound)
  #   4. User closes browser and reopens attachment, font size remains at previously set 20px
  #
  # ========================================
  Background: User Story
    As a TUI user viewing markdown attachments
    I want to adjust code block font sizes and persist preferences
    So that I can read documentation comfortably and have my settings remembered

  Scenario: Default code block font size is larger than current implementation
    Given the attachment server is running
    When I open a markdown attachment with code blocks
    Then the code blocks should display at 16px font size
    And the font size should be larger than the previous default

  Scenario: Increase font size with plus button
    Given I have a markdown attachment open with font size at 16px
    When I click the plus button in the font size controls
    Then the font size should increase to 18px
    And the code blocks should re-render with the new size
    And the size display should show "18px"

  Scenario: Font size respects minimum bound
    Given I have a markdown attachment open with font size at 10px
    When I click the minus button in the font size controls
    Then the font size should remain at 10px
    And the minus button should be disabled or show visual indication

  Scenario: Font size respects maximum bound
    Given I have a markdown attachment open with font size at 24px
    When I click the plus button in the font size controls
    Then the font size should remain at 24px
    And the plus button should be disabled or show visual indication

  Scenario: Font size persists across browser sessions
    Given I have set the font size to 20px in a previous session
    And I have closed the browser
    When I open a markdown attachment
    Then the code blocks should display at 20px font size
    And the localStorage should contain the font size preference

  Scenario: Font size controls display in top right corner
    Given the attachment server is running
    When I open a markdown attachment
    Then the font size controls should be visible in the top right corner
    And the controls should be next to the dark mode toggle
    And the controls should show plus button, size display, and minus button

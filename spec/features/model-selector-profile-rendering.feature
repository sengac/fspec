@done
@RPC-338 @tui @model-selector @ts-parity @wip
Feature: Model selector profile section rendering

  # Work unit: RPC-338. Rendering layer in views/model_selector/rows.rs —
  # magenta 📁 icon, red (unreachable) marker, and the restored legend segment.

  Background: User Story
    As a codelet TUI user
    I want profile sections (📁) and unreachable markers rendered in the model selector
    So that I can pick models from local profiles and tell which providers are down

  @ui-rendering
  Scenario: A reachable profile section renders the folder icon and qualified label
    Given the model selector is showing a provider list
    And a profile section with profile_name "my-profile", display_name "openai: my-profile", 3 models, and is_unreachable false
    When the provider header row is rendered while not selected
    Then a magenta 📁 icon appears after the expand arrow and before the label
    And the header text includes "openai: my-profile (3 models)"
    And the header shows no "(unreachable)" marker

  @ui-rendering
  Scenario: An unreachable profile header renders a red marker and is never hidden
    Given the model selector is showing a provider list
    And an unreachable profile section with profile_name "down-profile", display_name "openai: down-profile", 0 models, and is_unreachable true
    When the provider header row is rendered while not selected
    Then a magenta 📁 icon appears after the expand arrow and before the label
    And a red " (unreachable)" marker appears after the "(0 models)" count
    And the header text contains no duplicated "(unreachable)" before the count
    And the header row remains non-selectable
    And the header row is still present in the list

  @ui-rendering
  Scenario: A selected profile header renders its markers in the selected style
    Given the model selector is showing a provider list
    And a profile section with profile_name set and is_unreachable true
    When the provider header row is rendered while selected
    Then the 📁 icon is rendered in the selected highlight style rather than magenta
    And the " (unreachable)" marker is rendered in the selected highlight style rather than red

  @ui-rendering
  Scenario: A cloud provider header renders without profile or unreachable markers
    Given the model selector is showing a provider list
    And a cloud provider header with profile_name None and is_unreachable false
    When the provider header row is rendered
    Then the header shows no 📁 prefix
    And the header shows no "(unreachable)" marker

  @ui-rendering
  Scenario: The body legend includes the profile segment
    Given the model selector body is rendered
    Then the legend line reads "[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)"

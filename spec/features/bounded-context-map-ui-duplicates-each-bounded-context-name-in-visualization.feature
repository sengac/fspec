@foundation-management
@documentation-generation
@critical
@generator
@foundation
@bug-fix
@BUG-085
Feature: Bounded Context Map UI duplicates each bounded context name in visualization

  """
  Bug exists in src/generators/foundation-md.ts in the generateBoundedContextMermaid function. The default case on line 50 sets description equal to context.text, causing duplication when context doesn't match hardcoded cases. Fix: Change default case to set description to empty string instead of context.text.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each bounded context must appear exactly once in the Mermaid diagram node label
  #   2. Mermaid node labels should display context name on first line and brief description on second line using <br/> tag
  #   3. For bounded contexts without hardcoded descriptions, display only the context name without duplication
  #
  # EXAMPLES:
  #   1. Bounded context 'Work Management' generates node label 'Work Management<br/>Stories, Epics, Dependencies' (two lines: name + description)
  #   2. Bounded context 'Conversation Management' currently generates 'Conversation Management<br/>Conversation Management' (BUG: duplicated), should generate 'Conversation Management' (single line only)
  #   3. After fix, 'Mind Mapping' context generates node label 'Mind Mapping' (single line, no <br/> tag)
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to view bounded context map visualization
    So that I see each context name displayed exactly once without duplication

  Scenario: Hardcoded bounded context generates name and description on two lines
    Given I have a bounded context "Work Management" with hardcoded description
    When the Mermaid diagram is generated
    Then the node label should be "Work Management<br/>Stories, Epics, Dependencies"
    And the label should display context name on first line
    And the label should display description on second line

  Scenario: Non-hardcoded bounded context generates only context name without duplication
    Given I have a bounded context "Conversation Management" without hardcoded description
    When the Mermaid diagram is generated
    Then the node label should be "Conversation Management"
    And the label should NOT contain "<br/>" tag
    And the label should NOT duplicate the context name

  Scenario: Generate bounded context map for multiple contexts without duplication
    Given I have bounded contexts "Mind Mapping", "AI Integration", and "Workspace & Storage"
    And none of them have hardcoded descriptions
    When the bounded context map is generated in FOUNDATION.md
    Then each context should appear exactly once
    And no context name should be duplicated in its own node label

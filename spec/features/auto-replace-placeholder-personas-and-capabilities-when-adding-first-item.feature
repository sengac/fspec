@done
@automation
@discovery
@cli
@foundation-management
@high
@REMIND-011
Feature: Auto-replace placeholder personas and capabilities when adding first item

  """
  Architecture notes:
  - Modifies add-persona and add-capability commands to detect placeholders before adding new items
  - Placeholder detection: checks if name or description contains [QUESTION:] or [DETECTED:] patterns
  - Only removes placeholders when adding the FIRST real item (array has only placeholders, no real items)
  - Works with both foundation.json and foundation.json.draft files
  - Shows user-friendly message when placeholders are removed (e.g., "Removed N placeholder persona(s)")
  - Uses same placeholder detection logic as discover-foundation command for consistency

  Implementation approach:
  - Before adding new persona/capability, scan existing array for placeholders
  - If array contains ONLY placeholders (no real items), remove all placeholders then add new item
  - If array contains at least one real item, add new item without removing placeholders
  - Detection regex: /\[QUESTION:||\[DETECTED:/ applied to name and description fields
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. add-persona must detect and remove placeholder personas when adding the first real persona
  #   2. add-capability must detect and remove placeholder capabilities when adding the first real capability
  #   3. Placeholder detection: name or description contains [QUESTION:] or [DETECTED:] patterns
  #   4. Only remove placeholders when adding the FIRST real item (not subsequent items)
  #   5. Must work with both foundation.json and foundation.json.draft files
  #
  # EXAMPLES:
  #   1. Foundation has placeholder persona '[QUESTION: Who uses this?]', user runs 'fspec add-persona "Developer" "Writes code"', placeholder persona is removed, Developer persona is added
  #   2. Foundation has placeholder capability '[QUESTION: What can users DO?]', user runs 'fspec add-capability "Spec Validation" "Validates Gherkin"', placeholder capability is removed, Spec Validation capability is added
  #   3. Foundation already has real persona 'Developer', user adds 'QA Engineer', NO placeholders are removed (not the first item)
  #   4. Foundation has 3 placeholder personas, user adds first real persona, ALL 3 placeholder personas are removed
  #   5. Foundation has 1 real persona 'Developer' and 1 placeholder persona '[QUESTION: ...]', user adds 'QA Engineer', NO placeholders removed (already has real persona)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the command output show a message when placeholders are removed?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using discover-foundation workflow
    I want to add my first real persona or capability
    So that placeholder items are automatically cleaned up without manual editing

  Scenario: Add first persona removes placeholder persona
    Given a foundation file with placeholder persona "[QUESTION: Who uses this?]"
    When I run "fspec add-persona \"Developer\" \"Writes code\""
    Then the placeholder persona should be removed
    And the foundation should contain persona "Developer" with description "Writes code"
    And the output should show "Removed 1 placeholder persona(s)"

  Scenario: Add first capability removes placeholder capability
    Given a foundation file with placeholder capability "[QUESTION: What can users DO?]"
    When I run "fspec add-capability \"Spec Validation\" \"Validates Gherkin\""
    Then the placeholder capability should be removed
    And the foundation should contain capability "Spec Validation" with description "Validates Gherkin"
    And the output should show "Removed 1 placeholder capability(ies)"

  Scenario: Add subsequent persona does not remove placeholders
    Given a foundation file with real persona "Developer"
    When I run "fspec add-persona \"QA Engineer\" \"Tests features\""
    Then no placeholders should be removed
    And the foundation should contain both personas "Developer" and "QA Engineer"
    And the output should NOT show placeholder removal message

  Scenario: Multiple placeholder personas are all removed
    Given a foundation file with 3 placeholder personas
    When I run "fspec add-persona \"Developer\" \"First real persona\""
    Then all 3 placeholder personas should be removed
    And the foundation should contain only persona "Developer"
    And the output should show "Removed 3 placeholder persona(s)"

  Scenario: Mixed real and placeholder personas - no removal on subsequent add
    Given a foundation file with real persona "Developer" and placeholder persona "[QUESTION: ...]"
    When I run "fspec add-persona \"QA Engineer\" \"Tests features\""
    Then no placeholders should be removed
    And the foundation should contain personas "Developer", "QA Engineer", and the placeholder
    And the output should NOT show placeholder removal message

  Scenario: Auto-remove works with draft file
    Given a draft file "foundation.json.draft" with placeholder persona "[QUESTION: Who uses this?]"
    When I run "fspec add-persona \"Developer\" \"Writes code\" --draft-path foundation.json.draft"
    Then the placeholder persona should be removed from the draft
    And the draft should contain persona "Developer"
    And the output should show "Removed 1 placeholder persona(s)"

  Scenario: Auto-remove works with capability in draft file
    Given a draft file "foundation.json.draft" with placeholder capability "[DETECTED: cli-tool]"
    When I run "fspec add-capability \"Command Execution\" \"Runs CLI commands\" --draft-path foundation.json.draft"
    Then the placeholder capability should be removed from the draft
    And the draft should contain capability "Command Execution"
    And the output should show "Removed 1 placeholder capability(ies)"
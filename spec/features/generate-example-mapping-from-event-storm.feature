@high
@cli
@example-mapping
@event-storm
@EXMAP-014
Feature: Generate Example Mapping from Event Storm
  """
  Architecture notes:
  - Implements Gap 1 from EXMAP-013 analysis
  - Uses fileManager.transaction for atomic work-units.json updates
  - Processes Event Storm items array to derive Example Mapping entries
  - Skips deleted items (deleted: true flag)
  - Generates proper IDs using existing array lengths as next ID
  - Updates work unit and meta timestamps after changes
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must derive business rules from Event Storm policies by combining when/then fields into rule text
  #   2. Command must derive scenario examples from domain events by converting PascalCase event names to natural language
  #   3. Command must derive questions for humans from hotspot concerns
  #   4. Command must skip deleted Event Storm items (deleted: true)
  #   5. Generated rules, examples, and questions must include timestamps and proper ID sequencing
  #
  # EXAMPLES:
  #   1. Policy 'when UserRegistered then SendWelcomeEmail' generates rule 'System must send welcome email after user registration'
  #   2. Event 'UserAuthenticated' generates example 'User user authenticated and is logged in'
  #   3. Hotspot with concern 'Unclear password reset timeout' generates question '@human: What should unclear password reset timeout be?'
  #
  # ========================================
  Background: User Story
    As a AI agent performing Event Storming
    I want to automatically generate Example Mapping from Event Storm artifacts
    So that I reduce manual work and ensure Event Storm knowledge flows to acceptance criteria

  Scenario: Generate business rule from Event Storm policy
    Given work unit "AUTH-001" has Event Storm with policy item
    And the policy has when="UserRegistered" and then="SendWelcomeEmail"
    When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
    Then a new rule should be added to AUTH-001 Example Mapping
    And the rule text should be "System must send welcome email after user registration"
    And the rule should be derived from policy when/then fields

  Scenario: Generate scenario example from domain event
    Given work unit "AUTH-001" has Event Storm with event item
    And the event text is "UserAuthenticated"
    When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
    Then a new example should be added to AUTH-001 Example Mapping
    And the example text should contain "User enters valid credentials and is logged in"
    And the example should be derived from event text

  Scenario: Generate question from Event Storm hotspot
    Given work unit "AUTH-001" has Event Storm with hotspot item
    And the hotspot concern is "Unclear password reset timeout"
    When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
    Then a new question should be added to AUTH-001 Example Mapping
    And the question text should be "@human: What should password reset token timeout be?"
    And the question should be marked as unanswered

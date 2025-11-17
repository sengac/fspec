@EXMAP-013
Feature: Event Storm workflow enhancements and gap fixes
  """
  Extends existing Event Storm commands (EXMAP-005, 006, 007, 010, 011, 012) with automation and validation. New commands: generate-example-mapping-from-event-storm, add-event-storm-diagram, validate-event-storm, validate-bounded-contexts. Enhances generate-scenarios to auto-populate architecture notes. Uses Mermaid for diagram generation, JSON Schema for validation, fuzzy matching for typo detection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command generate-example-mapping-from-event-storm must derive business rules from Event Storm policies (when/then → rule text)
  #   2. Command generate-example-mapping-from-event-storm must derive examples from domain events (event text → scenario example)
  #   3. Command generate-example-mapping-from-event-storm must derive questions from hotspots (concern text → question for human)
  #   4. All add-* Event Storm commands must validate relationship IDs (triggersEvent, relatedTo) exist before creating items
  #   5. Command validate-event-storm must detect broken references (triggersEvent pointing to non-existent IDs, orphaned relatedTo entries)
  #   6. Command add-event-storm-diagram must generate Mermaid diagrams using color codes matching Event Storming convention (orange/blue/yellow/purple/red/pink)
  #   7. Timeline diagrams must use timestamp field for ordering Event Storm items chronologically
  #   8. Show commands (show-event-storm, show-foundation-event-storm) must support --bounded-context filter to show items within specific context only
  #   9. Command validate-bounded-contexts must detect bounded contexts in work unit that don't match foundation-level bounded contexts (typos or missing)
  #   10. Command generate-scenarios must auto-generate architecture notes from Event Storm data (bounded contexts, aggregates, external systems, key events)
  #
  # EXAMPLES:
  #   1. Work unit AUTH-001 has policy 'when UserRegistered then SendWelcomeEmail' → generate-example-mapping creates rule 'System must send welcome email after user registration'
  #   2. Work unit AUTH-001 has event 'UserAuthenticated' → generate-example-mapping creates example 'User enters valid credentials and is logged in'
  #   3. Work unit AUTH-001 has hotspot 'Unclear password reset timeout' → generate-example-mapping creates question '@human: What should password reset token timeout be?'
  #   4. Command 'add-command AUTH-001 Login --triggers-event=99' fails with 'Event ID 99 does not exist' before creating command
  #   5. Command 'validate-event-storm AUTH-001' detects 'Command ID 2 has triggersEvent=99 but event ID 99 does not exist' and suggests fix
  #   6. Command 'add-event-storm-diagram AUTH-001 --type=timeline' generates Mermaid timeline using timestamp field and attaches as spec/attachments/AUTH-001/event-storm-timeline.png
  #   7. Command 'add-event-storm-diagram AUTH-001 --type=flow' generates Mermaid flow diagram showing command→event→policy relationships with correct colors (blue→orange→purple)
  #   8. Command 'show-event-storm AUTH-001 --bounded-context="User Management"' shows only 5 items assigned to User Management context, filters out 3 items from Authentication context
  #   9. Command 'validate-bounded-contexts AUTH-001' detects 'Session Managment' (typo) doesn't match foundation 'Session Management' and suggests fix
  #   10. Command 'generate-scenarios AUTH-001' with Event Storm data creates feature file with architecture notes: 'Bounded Context: User Management, Aggregates: User, Session, External Systems: OAuth2Provider'
  #
  # ========================================
  Background: User Story
    As a AI agent performing Event Storming
    I want to leverage Event Storm data to automate Example Mapping, visualize domain structure, and maintain data integrity
    So that I can reduce manual work, improve documentation quality, and catch modeling errors early

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

  Scenario: Reject command with non-existent triggersEvent ID
    Given work unit "AUTH-001" has Event Storm with 5 items
    And no event with ID 99 exists
    When I run "fspec add-command AUTH-001 'Login' --triggers-event=99"
    Then the command should fail with exit code 1
    And the error message should contain "Event ID 99 does not exist"
    And no command item should be added to Event Storm

  Scenario: Detect broken references in Event Storm
    Given work unit "AUTH-001" has Event Storm with command item ID 2
    And command ID 2 has triggersEvent=99
    And event ID 99 does not exist
    When I run "fspec validate-event-storm AUTH-001"
    Then the validation should fail
    And the output should contain "Command ID 2 has triggersEvent=99 but event ID 99 does not exist"
    And the output should suggest removing triggersEvent field or correcting ID

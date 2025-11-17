@work-unit-management
@event-storming
@cli
@high
@EXMAP-007
Feature: Event Storm artifact commands (policies, hotspots, external systems)
  """
  Extends EXMAP-006 Event Storm commands with policies, hotspots, external systems, and bounded contexts. Follows same pattern: stable IDs, soft-delete, TypeScript discriminated unions. Commands: add-policy (--when/--then), add-hotspot (--concern), add-external-system (--type), add-bounded-context (--description). Color codes: purple (policy), red (hotspot), pink (external), null (bounded context). Data stored in work-units.json eventStorm.items array with type field for discrimination.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Commands add-policy, add-hotspot, add-external-system, add-bounded-context take work-unit-id and text as required arguments
  #   2. Each command creates item with auto-incremented ID, type field, appropriate color code, deleted=false, createdAt timestamp
  #   3. Commands initialize eventStorm section if not present with level='process_modeling', items=[], nextItemId=0
  #   4. All commands support --timestamp flag for timeline visualization (milliseconds) and --bounded-context flag for domain association
  #   5. Commands must validate work unit exists and is not in done/blocked state before adding Event Storm items
  #   6. Color codes: Policies='purple', Hotspots='red', External Systems='pink', Bounded Contexts (default to 'purple' if not specified in Event Storming standard)
  #   7. Yes, support --when and --then flags to capture policy rule (event triggering it and resulting command)
  #   8. Yes, support --concern flag to capture description of risk/uncertainty/problem
  #   9. Yes, support --type flag with values: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM
  #   10. Yes, support --description flag to capture scope (what the bounded context covers)
  #   11. Use null/undefined for color since bounded contexts are conceptual boundaries, not sticky note artifacts
  #
  # EXAMPLES:
  #   1. Run 'fspec add-policy AUTH-001 "Send welcome email" --when "UserRegistered" --then "SendWelcomeEmail"' creates policy with id=0, type='policy', color='purple', when='UserRegistered', then='SendWelcomeEmail'
  #   2. Run 'fspec add-hotspot AUTH-001 "Password reset token expiration" --concern "Unclear timeout duration"' creates hotspot with id=1, type='hotspot', color='red', concern='Unclear timeout duration'
  #   3. Run 'fspec add-bounded-context AUTH-001 "User Management" --description "Handles user registration, authentication, and profile management"' creates bounded context with id=3, type='bounded_context', color=null, description='Handles user registration...'
  #   4. Commands initialize eventStorm section if not present, then append to items array and increment nextItemId counter
  #   5. All commands support optional --timestamp and --bounded-context flags for timeline visualization and domain association
  #   6. Run fspec add-external-system AUTH-001 "OAuth2Provider" --type REST_API creates external system with id=2, type=external_system, color=pink, integrationType=REST_API
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should add-policy command support --when and --then flags to capture the policy rule (when X happens, then Y should occur)?
  #   A: true
  #
  #   Q: Should add-hotspot command support a --concern or --question flag to capture what needs clarification or what the risk/issue is?
  #   A: true
  #
  #   Q: Should add-external-system command support --type or --protocol flags to describe the external system integration (e.g., REST API, Message Queue, Database)?
  #   A: true
  #
  #   Q: Should add-bounded-context command support --description and/or --responsibilities flags similar to add-aggregate?
  #   A: true
  #
  #   Q: What color should bounded contexts use? The Event Storming standard doesn't specify a color for bounded contexts - should we use a specific color or allow --color flag?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent or developer conducting Event Storming
    I want to capture policies, hotspots, external systems, and bounded contexts
    So that I can document complete Event Storm sessions with all artifact types

  Scenario: Add policy with when and then flags
    Given I have a work unit AUTH-001 in specifying status
    When I run fspec add-policy AUTH-001 "Send welcome email" --when "UserRegistered" --then "SendWelcomeEmail"
    Then a policy item should be created with id 0
    And the policy should have type "policy"
    And the policy should have color "purple"
    And the policy should have when "UserRegistered"
    And the policy should have then "SendWelcomeEmail"

  Scenario: Add hotspot with concern flag
    Given I have a work unit AUTH-001 with existing Event Storm items
    When I run fspec add-hotspot AUTH-001 "Password reset token expiration" --concern "Unclear timeout duration"
    Then a hotspot item should be created
    And the hotspot should have type "hotspot"
    And the hotspot should have color "red"
    And the hotspot should have concern "Unclear timeout duration"

  Scenario: Add external system with type flag
    Given I have a work unit AUTH-001 in specifying status
    When I run fspec add-external-system AUTH-001 "OAuth2Provider" --type REST_API
    Then an external system item should be created
    And the external system should have type "external_system"
    And the external system should have color "pink"
    And the external system should have integrationType "REST_API"

  Scenario: Add bounded context with description flag
    Given I have a work unit AUTH-001 in specifying status
    When I run fspec add-bounded-context AUTH-001 "User Management" --description "Handles user registration, authentication, and profile management"
    Then a bounded context item should be created
    And the bounded context should have type "bounded_context"
    And the bounded context should have color null
    And the bounded context should have description "Handles user registration, authentication, and profile management"

  Scenario: Initialize eventStorm section on first command
    Given I have a work unit AUTH-001 with no eventStorm section
    When I run fspec add-policy AUTH-001 "Send notification"
    Then the eventStorm section should be initialized
    And the eventStorm level should be "process_modeling"
    And the eventStorm items array should contain the new policy
    And the nextItemId should be 1

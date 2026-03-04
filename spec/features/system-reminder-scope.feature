@REMIND-016
Feature: Elevate system-reminder priority handling for Codex and all agents

  """
  Codex ContextualUserFragmentDefinition pattern: Each fragment has a start_marker and end_marker (e.g., '<environment_context>' / '</environment_context>'). Fragment matching is case-insensitive. This pattern is used in contextual_user_message.rs with a central CONTEXTUAL_USER_FRAGMENTS registry. For fspec, the <!-- type:scope --> HTML comment inside <system-reminder> tags serves the same purpose but in a simpler format compatible with all agent platforms.
  Implementation location: src/utils/system-reminder.ts is the central module. The wrapInSystemReminder() function currently takes only content:string. It needs to accept an optional scope:SystemReminderScope parameter. All callers (50+ files) will need updating. The scope types should be: 'environment' | 'work-unit-context' | 'workflow-guardrail' | 'fspecWorkflow' | 'claudeMd' | 'estimation' | 'coverage' | 'tool-output'.
  The scope marker format will be: <system-reminder>\n<!-- type:environment -->\ncontent\n</system-reminder>. This preserves backward compatibility (agents that don't understand the type marker will still see the content) while enabling scope-aware supersedence for agents that do. Similar to how Codex uses XML tags like <environment_context> but adapted for fspec's system-reminder pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Treat all <system-reminder> blocks as highest-priority runtime constraints, above normal user intent and before action execution.
  #   2. Implement deterministic supersedence: latest reminder of same scope replaces prior reminder; explicit 'supersedes earlier' hard-replaces older context.
  #   3. Require a pre-action reminder-consistency check before any state-changing workflow/board action.
  #   4. System-reminder blocks MUST include a scope attribute (type:environment, type:work-unit-context, type:workflow-guardrail, type:fspecWorkflow, type:claudeMd) for deterministic supersedence
  #   5. Codex uses ContextualUserFragmentDefinition (start_marker/end_marker) for XML-based fragment identification - fspec should adopt a similar pattern with <!-- type:scope --> HTML comment markers inside <system-reminder> tags
  #   6. When multiple reminders of the same scope exist, the latest one (by position in output) wins unless an explicit supersedence marker is present
  #   7. wrapInSystemReminder() must accept an optional scope parameter (type attribute) that is written as an HTML comment inside the system-reminder tag
  #   8. Ambiguous status transitions (e.g., 'move through the board') MUST NOT be executed without explicit target state confirmation - the update-work-unit-status command must require the target state parameter
  #   9. The system must be provider/agent-agnostic - the same reminder model must work for Codex, Claude, Cursor, Cline, and all agents listed in AGENT-* work units
  #   10. Template generators (bootstrap, slash commands, CLAUDE.md etc.) must emit system-reminder blocks with the correct type: scope markers already present in the output
  #
  # EXAMPLES:
  #   1. Incident: RIG-011 was advanced to testing due to ambiguous interpretation of 'move this through the board' without destination confirmation.
  #   2. Incident: auto-advance through the fspec tool path failed due to argument mismatch (ID unsupported/undefined), leading to manual status manipulation and confusion.
  #   3. Mitigation: Before any status transition, agent must echo current work unit + current state + intended target state and ask for confirmation if intent is ambiguous.
  #   4. wrapInSystemReminder('content', 'environment') produces: <system-reminder>\n<!-- type:environment -->\ncontent\n</system-reminder>
  #   5. When two environment reminders appear, second says 'This supersedes earlier environment reminder', only the second should govern agent decisions
  #   6. User says 'move this through the board' without specifying target state -> update-work-unit-status already requires target state parameter, so the ambiguity is at the agent-prompt level, not fspec CLI level
  #   7. bootstrap and CLAUDE.md template output already contains <!-- type:fspecWorkflow --> inside the large system-reminder block for the ACDD workflow
  #   8. getStatusChangeReminder() produces reminders with scope 'workflow-guardrail', getMissingEstimateReminder() uses scope 'estimation', show-work-unit uses scope 'work-unit-context'
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to have deterministic, scope-keyed system-reminder priority handling with supersedence rules
    So that my workflow decisions are consistent, I don't make state transition mistakes from ambiguous context, and older reminders don't override newer ones

  @scope-marker
  Scenario: wrapInSystemReminder includes scope type marker when scope is provided
    Given a system reminder utility function
    When I call wrapInSystemReminder with content "test content" and scope "environment"
    Then the output should be a system-reminder block containing "<!-- type:environment -->" on the first line after the opening tag
    And the content "test content" should follow the scope marker

  @scope-marker
  Scenario: wrapInSystemReminder omits scope marker when no scope is provided
    Given a system reminder utility function
    When I call wrapInSystemReminder with content "test content" and no scope parameter
    Then the output should be a system-reminder block without any type marker comment
    And backward compatibility with existing callers is preserved

  @scope-marker
  Scenario: All defined SystemReminderScope values produce valid markers
    Given the SystemReminderScope type defines scopes "environment", "work-unit-context", "workflow-guardrail", "fspecWorkflow", "claudeMd", "estimation", "coverage", and "tool-output"
    When I call wrapInSystemReminder with each scope value
    Then each output should contain the corresponding "<!-- type:<scope> -->" marker

  @workflow-guardrail
  Scenario: Status change reminders include workflow-guardrail scope
    Given a work unit "AUTH-001" in "backlog" status
    When the status is changed to "specifying"
    Then the system reminder output should contain "<!-- type:workflow-guardrail -->"
    And the reminder content should contain specifying phase guidance

  @workflow-guardrail
  Scenario: Missing estimate reminders include estimation scope
    Given a work unit "AUTH-001" in "specifying" status with no estimate
    When the show-work-unit command runs
    Then the system reminder output should contain "<!-- type:estimation -->"
    And the reminder content should contain estimation guidance

  @work-unit-context
  Scenario: Show work unit reminders include work-unit-context scope
    Given a work unit "AUTH-001" with rules, examples, and architecture notes
    When the show-work-unit command displays the work unit
    Then any context-specific reminders should contain "<!-- type:work-unit-context -->"

  @template-generation
  Scenario: Bootstrap template includes fspecWorkflow scope marker
    Given the fspec bootstrap command generates a workflow template
    When the template output contains a system-reminder block
    Then the system-reminder block should contain "<!-- type:fspecWorkflow -->"

  @template-generation
  Scenario: CLAUDE.md template includes correct scope markers
    Given the template generator produces a CLAUDE.md file
    When the generated content contains system-reminder blocks
    Then the ACDD workflow block should contain "<!-- type:fspecWorkflow -->"
    And the environment block should contain "<!-- type:environment -->"
    And the coding standards block should contain "<!-- type:claudeMd -->"

  @supersedence
  Scenario: Latest environment reminder supersedes earlier one
    Given two system-reminder blocks with scope "environment"
    And the second block contains "This supersedes earlier environment reminder"
    When an agent processes the combined output
    Then only the content from the second reminder should be treated as active for scope "environment"

  @supersedence
  Scenario: Reminders of different scopes coexist without conflict
    Given a system-reminder block with scope "environment" containing "Platform: macos"
    And a system-reminder block with scope "work-unit-context" containing "Current work unit: AUTH-001"
    When an agent processes both reminders
    Then both reminders should be active simultaneously since they have different scopes

  @caller-migration
  Scenario: getStatusChangeReminder produces scoped reminders for all states
    Given the getStatusChangeReminder function
    When called with each workflow state "specifying", "testing", "implementing", "validating", "done", "blocked"
    Then each returned reminder should contain a "<!-- type:workflow-guardrail -->" scope marker

  @caller-migration
  Scenario: Work unit creation reminders include tool-output scope
    Given the workUnitCreatedReminder function
    When a new story "AUTH-001" is created
    Then the returned reminder should contain "<!-- type:tool-output -->" scope marker

  @caller-migration
  Scenario: Coverage-related reminders include coverage scope
    Given the generate-coverage command produces system reminders
    When the command completes
    Then coverage-specific reminders should contain "<!-- type:coverage -->" scope marker

  @agent-agnostic
  Scenario: Scope markers use HTML comments for cross-agent compatibility
    Given the scope marker format uses HTML comments "<!-- type:scope -->"
    When the reminder output is consumed by any agent platform
    Then agents that understand type markers can parse the scope
    And agents that do not understand type markers still see the full reminder content unaffected

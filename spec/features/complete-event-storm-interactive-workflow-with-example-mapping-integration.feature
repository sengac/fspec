@high
@discovery-workflow
@event-storm
@example-mapping
@cli
@system-reminder
@EXMAP-016
Feature: Complete Event Storm interactive workflow with Example Mapping integration
  """
  Uses existing Event Storm commands (add-domain-event, add-command, add-policy, add-hotspot, show-event-storm, generate-example-mapping-from-event-storm). Creates new discover-event-storm command that emits guidance only (no draft/finalize pattern). System-reminder integration in update-work-unit-status.ts when moving to specifying state. Guidance sections in AGENTS.md, spec/CLAUDE.md, and bootstrap. Uses getAgentConfig for agent-aware prompts (Claude Code vs Cursor). NO semantic code analysis - only AST tools and human input. Free-form conversation pattern like Example Mapping, not field-by-field like discover-foundation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Event Storm must have an interactive guided workflow like discover-foundation and Example Mapping (step-by-step with system-reminders)
  #   2. System must automatically detect WHEN Event Storm is needed based on work unit complexity (like Example Mapping is triggered for specifying state)
  #   3. Event Storm artifacts (policies, events, hotspots) must automatically pipe into Example Mapping using generate-example-mapping-from-event-storm command
  #   4. All guidance documents (AGENTS.md, spec/CLAUDE.md, bootstrap, system-reminders) must explain Event Storm workflow and when to use it
  #   5. Event Storm guidance must be agent-aware (detect Claude Code vs Cursor) like discover-foundation uses getAgentConfig
  #   6. CRITICAL: NO semantic code understanding allowed - fspec uses only AST/syntax analysis, file operations, and explicit human input
  #   7. Event Storm is a HUMAN-DRIVEN collaborative workshop - AI facilitates but does NOT infer domain knowledge from code semantics
  #   8. ABSOLUTELY NO semantic code - fspec relies on AI responses and tool calling with guidance and AST/research tools to help AI figure things out
  #   9. ABSOLUTELY NO heuristics - Leave it up to AI to interactively figure out if Event Storm is needed. Ask AI if it's appropriate for what needs to be written/done. Do this interactively using guidance, same way other interactive commands work (fspec review, fspec reverse)
  #   10. Yes, manual opt-in command 'fspec discover-event-storm <work-unit-id>' - BUT the critical part is CRYSTAL CLEAR GUIDANCE that makes AI understand WHEN to run it and WHY. Guidance must prevent AI from skipping this valuable discovery step when appropriate. Use ULTRATHINK prompts in guidance to make AI pause and consider if Event Storm would help before jumping to Example Mapping.
  #   11. System-reminder when moving to specifying state that prompts AI to ASSESS domain complexity with self-assessment questions (Do you understand core domain events? Are commands/policies clear? Significant domain complexity?). Makes AI CONSCIOUSLY CHOOSE between: (1) Run Event Storm FIRST then transform to Example Mapping, or (2) Skip Event Storm go directly to Example Mapping. Uses ULTRATHINK-style prompts to prevent skipping. Shows flow: Event Storm → generate-example-mapping-from-event-storm → Example Mapping → Scenarios. CRITICAL: Include concrete EXAMPLES in guidance showing when Event Storm helped vs when it was unnecessary.
  #   12. Different pattern from discover-foundation - use free-form pattern like Example Mapping. Command 'fspec discover-event-storm <work-unit-id>' emits comprehensive guidance showing conversation pattern with examples. AI uses existing commands (add-domain-event, add-command, add-policy, add-hotspot) freely in whatever order conversation goes. Explains flow: Events → Commands → Policies → Hotspots. Shows when to stop (like Example Mapping stop criteria). When complete, AI runs generate-example-mapping-from-event-storm to transform artifacts. NO draft/finalize - just guidance + existing tools.
  #
  # EXAMPLES:
  #   1. AI creates story with 13+ points → System emits reminder suggesting Event Storm → AI runs guided Event Storm session → Captures domain events, commands, policies → Transforms to Example Mapping → Generates scenarios
  #   2. AI creates simple 2-point bug → System skips Event Storm suggestion → Goes directly to Example Mapping workflow
  #   3. AI performs Event Storm → Adds domain event UserRegistered → System chains with prompt: 'Add related command or continue to next event?' → AI adds RegisterUser command → System chains to policies → Session completes → Artifacts visible in show-work-unit
  #   4. AI reads AGENTS.md Event Storm section → Understands when to use Event Storm (complex domains, unclear requirements) → Runs fspec discover-event-storm WORK-001 → Completes guided session
  #   5. AI uses bootstrap guidance → Sees Event Storm explanation → Understands ES → Example Mapping → Scenarios pipeline → Applies to complex feature
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Event Storm guidance use semantic code understanding or stay with AST/syntax-only analysis like all other fspec tools?
  #   A: true
  #
  #   Q: For automatic Event Storm detection, should we use simple heuristics (estimate > threshold, work unit type) or NO automatic detection at all?
  #   A: true
  #
  #   Q: Should Event Storm be a manual opt-in workflow (fspec discover-event-storm) with guidance only, similar to how discover-foundation works?
  #   A: true
  #
  #   Q: What triggers should suggest Event Storm? (1) Large estimates (13+ points)? (2) Specific work unit types? (3) Human request only? (4) Never auto-suggest?
  #   A: true
  #
  #   Q: Should Event Storm workflow be similar to discover-foundation (draft → field-by-field prompting → finalize) or different pattern?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent working with fspec
    I want to perform Event Storm discovery interactively with automatic detection and guidance
    So that I understand domain complexity before Example Mapping and write better specifications

  Scenario: System-reminder prompts AI to assess domain complexity when moving to specifying
    Given I have a work unit in backlog status
    When I run "fspec update-work-unit-status WORK-001 specifying"
    Then a system-reminder should appear with Event Storm assessment questions
    And the reminder should ask "Do you understand the core domain events?"
    And the reminder should ask "Are commands and policies clear?"
    And the reminder should ask "Is there significant domain complexity?"
    And the reminder should present choice: Run Event Storm FIRST or skip to Example Mapping
    And the reminder should include concrete examples of when Event Storm helped
    And the reminder should use ULTRATHINK-style prompts to prevent skipping

  Scenario: AI runs discover-event-storm and receives comprehensive guidance
    Given I have a work unit "AUTH-001" in specifying status
    When I run "fspec discover-event-storm AUTH-001"
    Then comprehensive guidance should be emitted as system-reminder
    And guidance should show conversation pattern with examples
    And guidance should explain flow: Events → Commands → Policies → Hotspots
    And guidance should list available commands (add-domain-event, add-command, add-policy, add-hotspot)
    And guidance should show when to stop criteria
    And guidance should explain generate-example-mapping-from-event-storm for transformation

  Scenario: Event Storm section file is created in slashCommandSections
    Given I need to add Event Storm guidance
    When I create "src/utils/slashCommandSections/eventStorm.ts"
    Then the file should export getEventStormSection() function
    And the function should return comprehensive Event Storm guidance
    And guidance should include conversation pattern with examples
    And guidance should explain when Event Storm is valuable
    And guidance should show flow: Events → Commands → Policies → Hotspots
    And guidance should explain transformation via generate-example-mapping-from-event-storm

  Scenario: Event Storm section is integrated into bootstrap
    Given I have created eventStorm.ts section file
    When I import getEventStormSection in slashCommandTemplate.ts
    And I add getEventStormSection() to getCompleteWorkflowDocumentation() array
    Then bootstrap output should include Event Storm guidance
    And guidance should appear between bootstrap-foundation and example-mapping sections

  Scenario: bootstrap guidance displays Event Storm workflow
    Given Event Storm section is integrated
    When AI runs "fspec bootstrap"
    Then AI should see Event Storm section in output
    And section should explain when to use Event Storm vs skip to Example Mapping
    And section should include concrete examples (complex domain vs simple bug)
    And section should show command: fspec discover-event-storm <work-unit-id>
    And section should explain transformation to Example Mapping

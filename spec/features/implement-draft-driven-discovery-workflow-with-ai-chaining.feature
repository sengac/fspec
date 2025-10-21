@foundation-management
@cli
@phase1
@DISC-001
Feature: Implement draft-driven discovery workflow with AI chaining
  """
  System-reminders are invisible to users but visible to AI - they guide AI through each field with: field N/M progress, field path, specific guidance (ULTRATHINK for codebase analysis, persona discovery from interactions, WHY/WHAT focus not HOW), exact command to run with field path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. discover-foundation creates foundation.json.draft with [QUESTION:] and [DETECTED:] placeholders
  #   2. After creating draft, command scans for FIRST unfilled [QUESTION:] field
  #   3. For each unfilled field, command emits system-reminder with: field path, guidance text, example values
  #   4. System-reminder instructs AI to: analyze codebase, ask human, use fspec update-foundation/add-capability/add-persona commands
  #   5. After AI runs fspec update-foundation, command automatically re-runs to find NEXT unfilled field
  #   6. Command must NOT advance if AI tries to manually edit draft - emit system-reminder to use commands
  #   7. When all [QUESTION:] placeholders resolved, command validates draft against JSON schema
  #   8. If validation passes: create foundation.json, delete draft, auto-run fspec generate-foundation-md
  #   9. If validation fails: show errors, keep draft, prompt AI to fix and re-run
  #   10. [DETECTED:] fields must be verified with human before accepting
  #   11. Command tracks progress: shows N of M fields completed in each system-reminder
  #   12. System-reminders must be actionable: specific field, specific command to run, example value
  #   13. AI must NEVER skip fields - command enforces sequential field completion
  #   14. Final FOUNDATION.md generation happens automatically without AI intervention
  #   15. Command must detect if draft was manually edited between runs and reject with error
  #
  # EXAMPLES:
  #   1. INITIAL RUN: Human runs 'fspec discover-foundation' → Command creates foundation.json.draft with structure → Draft has REQUIRED fields: project{name,vision,projectType}, problemSpace{primaryProblem{title,description,impact}}, solutionSpace{overview,capabilities[]}, personas[] → All fields contain [QUESTION: ...] or [DETECTED: ...] placeholders → Command emits system-reminder: 'Draft created. To complete foundation, you must ULTRATHINK the entire codebase. Analyze EVERYTHING: code structure, entry points, user interactions, documentation. Understand HOW it works, then determine WHY it exists and WHAT users can do. I will guide you field-by-field. Field 1/9: project.name'
  #   2. FIELD 1 (project.name): Command reads draft → Finds [QUESTION: What is the project name?] → Emits: 'Field 1/9: project.name. Analyze project configuration to determine project name. Confirm with human. Run: fspec update-foundation projectName "<name>"' → AI analyzes codebase, determines 'fspec' → AI asks human 'Project name is fspec, correct?' → Human confirms → AI runs 'fspec update-foundation projectName "fspec"' → Command detects update, re-scans draft, finds next unfilled field
  #   3. FIELD 2 (project.vision): Command re-scans → Finds [QUESTION: What is the one-sentence vision?] → Emits: 'Field 2/9: project.vision (elevator pitch). ULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE? Focus on WHY this exists, not HOW it works. Ask human. Run: fspec update-foundation projectVision "your vision"' → AI reads README, analyzes commands (validate, format, list-features), understands it manages Gherkin specs → AI formulates: 'CLI tool for managing Gherkin specifications using ACDD' → AI asks human 'Is vision correct?' → Human confirms → AI updates field
  #   4. FIELD 3 (project.projectType): Command re-scans → Finds [DETECTED: cli-tool] → Emits: 'Field 3/9: project.projectType. Analyze codebase to determine project type. Verify with human. Options: cli-tool, web-app, library, sdk, mobile-app, desktop-app, service, api, other. Run: fspec update-foundation projectType "cli-tool"' → AI analyzes code structure and patterns → AI confirms: yes, this is cli-tool → AI asks human 'Detected cli-tool, correct?' → Human confirms → AI updates field → Command chains to problemSpace
  #   5. FIELD 4 (problemSpace.primaryProblem): Command re-scans → Finds [QUESTION: What problem does this solve?] → Emits: 'Field 4/9: problemSpace.primaryProblem. CRITICAL: Think from USER perspective. WHO uses this (persona)? WHAT problem do THEY face? WHY do they need this solution? Analyze codebase to understand user pain, ask human. Required: title, description, impact (high/medium/low). Run: fspec update-foundation problemDefinition "Problem Title and Description"' → AI analyzes: this is CLI for specs, users are developers, they need structured spec workflow → AI asks human 'Primary problem: Developers lack structured workflow for managing specifications?' → Human elaborates → AI updates title,description,impact
  #   6. FIELD 5 (solutionSpace.capabilities): Command re-scans → Finds [QUESTION: What can users DO?] → Emits: 'Field 5/9: solutionSpace.capabilities. List 3-7 HIGH-LEVEL abilities users have. Focus on WHAT not HOW. Example: "Spec Validation" (WHAT), not "Uses Cucumber parser" (HOW). Analyze user-facing functionality to identify capabilities. Run: fspec add-capability "Capability Name" "Capability Description". Run again for each capability (3-7 recommended)' → AI analyzes user-facing functionality: validate, format, list, create → Groups into capabilities: Spec Management, Validation, Work Unit Tracking → AI asks human → Human confirms → AI adds each capability with description
  #   7. FIELD 6 (personas): Command re-scans → Finds [QUESTION: Who uses this?] → Emits: 'Field 6/9: personas. Identify ALL user types from interactions. CLI tools: who runs commands? Web apps: who uses UI + who calls API? Analyze ALL user-facing code. Ask human about goals and pain points. Run: fspec add-persona "Persona Name" "Persona Description" --goal "Primary goal". Run again for each persona (repeat --goal for multiple goals)' → AI analyzes: CLI commands used by developers, no web UI, no API consumers → Identifies persona: "Developer using CLI in terminal" → AI asks human about goals: managing specs, following ACDD → AI asks about pain points: fragmented tools, no workflow → AI updates persona with name, description, goals, painPoints
  #   8. COMPLETION: AI fills last required field → Command re-scans → ALL required fields complete (no [QUESTION:] in required fields) → Command validates draft against JSON schema → Schema validation PASSES → Command creates spec/foundation.json from draft → Command deletes spec/foundation.json.draft → Command AUTOMATICALLY runs 'fspec generate-foundation-md' (no AI action needed) → Command emits: 'Discovery complete\! Created: spec/foundation.json, spec/FOUNDATION.md. Foundation is ready. You can now run fspec commands that require foundation.' → AI can now proceed with other work
  #   9. ERROR: Manual Editing → AI uses Write tool to edit foundation.json.draft directly → Command detects: draft mtime changed without fspec command → Command compares last known state vs current → Command emits ERROR in system-reminder: 'CRITICAL: You manually edited foundation.json.draft. This violates the workflow. You MUST use: fspec update-foundation <section> "<value>" OR fspec add-capability "<name>" "<description>" OR fspec add-persona "<name>" "<description>" --goal "<goal>". Reverting your changes. Draft restored to last valid state. Try again with proper command.' → Draft is restored → AI must use fspec commands
  #   10. ERROR: Validation Failure → AI fills all fields → Command validates against schema → Validation FAILS: missing required field problemSpace.primaryProblem.description → Command emits: 'Schema validation failed. Missing required: problemSpace.primaryProblem.description. Draft NOT finalized. Fix by running appropriate commands: fspec update-foundation <section> "<value>", fspec add-capability "<name>" "<description>", or fspec add-persona "<name>" "<description>" --goal "<goal>". Then re-run discover-foundation to validate.' → Draft kept, foundation.json NOT created → AI fixes missing field and re-runs
  #
  # ========================================
  Background: User Story
    As a AI agent running discover-foundation
    I want to be guided step-by-step through filling foundation.json.draft
    So that foundation.json is created accurately with all required fields and FOUNDATION.md is auto-generated

  Scenario: Initial draft creation with ULTRATHINK guidance
    Given human runs "fspec discover-foundation"
    When command creates foundation.json.draft
    Then draft should contain REQUIRED field structure
    And project fields should have [QUESTION:] placeholders for name, vision
    And project.projectType should have [DETECTED:] placeholder
    And problemSpace should have [QUESTION:] placeholders
    And solutionSpace should have [QUESTION:] placeholders
    And personas array should have [QUESTION:] placeholders
    And command should emit system-reminder with ULTRATHINK guidance
    And system-reminder should instruct AI to analyze entire codebase
    And system-reminder should show "Field 1/N: project.name"

  Scenario: Fill project.name field via feedback loop
    Given foundation.json.draft exists with [QUESTION: What is the project name?]
    When command scans draft for next unfilled field
    Then command should emit system-reminder for project.name
    And system-reminder should say "Field 1/N: project.name"
    And system-reminder should instruct to analyze project configuration
    And system-reminder should provide exact command to run
    When AI analyzes codebase and confirms name with human
    And AI runs "fspec update-foundation projectName fspec"
    Then command should detect draft update
    And command should automatically re-scan draft
    And command should find NEXT unfilled field

  Scenario: Fill project.vision with ULTRATHINK guidance
    Given draft has project.name filled
    And draft has [QUESTION: What is the one-sentence vision?] for project.vision
    When command re-scans draft
    Then command should emit system-reminder for project.vision
    And system-reminder should say "ULTRATHINK: Read ALL code, understand deeply"
    And system-reminder should emphasize WHY not HOW
    And system-reminder should instruct AI to formulate elevator pitch
    When AI analyzes entire codebase and formulates vision
    And AI confirms vision with human
    And AI runs update-foundation command for vision
    Then command should chain to next unfilled field

  Scenario: Verify DETECTED project type with human
    Given draft has [DETECTED: cli-tool] for project.projectType
    When command scans draft
    Then command should emit system-reminder to verify detected value
    And system-reminder should list all projectType options
    And system-reminder should instruct AI to verify with human
    When AI analyzes code structure and patterns
    And AI asks human "Detected cli-tool, correct?"
    And human confirms
    And AI runs update-foundation for projectType
    Then command should accept verified value
    And command should chain to problemSpace fields

  Scenario: Fill problemSpace from USER perspective
    Given draft has [QUESTION: What problem does this solve?]
    When command scans problemSpace.primaryProblem
    Then system-reminder should emphasize USER perspective
    And system-reminder should ask "WHO uses this? (persona)"
    And system-reminder should ask "WHAT problem do THEY face? (WHY)"
    And system-reminder should require title, description, impact
    When AI identifies users as developers from CLI analysis
    And AI determines problem: lack of structured spec workflow
    And AI asks human to elaborate on problem
    And AI runs update-foundation for primaryProblem fields
    Then command should chain to solutionSpace

  Scenario: Fill solutionSpace.capabilities with WHAT not HOW focus
    Given draft has [QUESTION: What can users DO?] for capabilities
    When command scans solutionSpace.capabilities
    Then system-reminder should emphasize WHAT not HOW
    And system-reminder should give example: "Spec Validation" not "Uses Cucumber parser"
    And system-reminder should instruct to list 3-7 high-level abilities
    When AI analyzes commands: validate, format, list, create
    And AI groups into capabilities: Spec Management, Validation, Work Unit Tracking
    And AI confirms capabilities with human
    And AI runs update-foundation for each capability
    Then command should chain to personas

  Scenario: Fill personas from user interaction analysis
    Given draft has [QUESTION: Who uses this?] for personas
    When command scans personas array
    Then system-reminder should instruct to identify ALL user types
    And system-reminder should guide based on project type
    And system-reminder should ask about goals and pain points
    When AI analyzes CLI commands (no web UI, no API consumers)
    And AI identifies persona: "Developer using CLI in terminal"
    And AI asks human about goals: managing specs, following ACDD
    And AI asks human about pain points: fragmented tools, no workflow
    And AI runs update-foundation for persona with name, description, goals, painPoints
    Then command should recognize this as last required field

  Scenario: Complete discovery with auto-generation
    Given AI has filled all required fields
    When command re-scans draft
    Then command should find NO [QUESTION:] placeholders in required fields
    And command should validate draft against JSON schema
    And validation should PASS
    Then command should create spec/foundation.json from draft
    And command should delete spec/foundation.json.draft
    And command should AUTOMATICALLY run "fspec generate-foundation-md"
    And command should emit completion message
    And message should show: "Created: spec/foundation.json, spec/FOUNDATION.md"
    And message should say: "Foundation is ready"

  Scenario: Detect and reject manual editing
    Given foundation.json.draft exists with unfilled fields
    When AI attempts to edit draft using Write tool directly
    Then command should detect draft mtime changed outside fspec
    And command should compare last known state vs current
    And command should emit ERROR in system-reminder
    And error should say "CRITICAL: You manually edited foundation.json.draft"
    And error should say "You MUST use: fspec update-foundation, fspec add-capability, fspec add-persona"
    And command should revert changes
    And draft should be restored to last valid state
    And command should re-scan from restored state

  Scenario: Handle schema validation failure
    Given AI has filled all required fields
    When command validates draft against schema
    And validation FAILS with missing field: problemSpace.primaryProblem.description
    Then command should emit schema validation error
    And error should say "Missing required: problemSpace.primaryProblem.description"
    And error should provide fix command with field path
    And draft should NOT be deleted
    And foundation.json should NOT be created
    When AI runs fix command for missing field
    And AI re-runs discover-foundation
    Then command should validate again and succeed

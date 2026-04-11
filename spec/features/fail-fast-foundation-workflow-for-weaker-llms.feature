@done
@high
@json-schema
@draft-driven
@anti-drift
@discovery-workflow
@error-handling
@validation
@foundation-management
@cli
@foundation
@FOUND-044
Feature: Fail-Fast Foundation Workflow for Weaker LLMs

  """
  CRITICAL CONSTRAINT: 100% deterministic behavior — NO fuzzy/semantic matching anywhere. No Levenshtein distance, no 'did you mean' suggestions, no similarity scoring on user input. All error messages must list valid values verbatim using the established pattern: `Invalid <field>: <value>. Valid values: <comma-separated-list>. Fix: fspec <command> <args>`. Reference implementations: update-work-unit-status.ts:119, update-work-unit-estimate.ts:35, work-unit.ts:737.
  Convention: list-foundation-sections is a standalone command (like list-features, list-epics, list-tags, list-prefixes). 12 existing standalone list-* commands in src/commands/ vs 1 --list-sections flag. Match the established pattern.
  Draft-exists error is a hard error (valid: false) with a wrapped system-reminder — matches existing pattern at discover-foundation.ts:501-527. Do NOT return draft content inline; the dedicated `show-foundation --draft` command owns that responsibility. One command = one purpose.
  IMPACT ANALYSIS for removing the projectType enum (performed via direct code search, not DeepSearch):

1. ZERO business logic depends on specific projectType values. Searched for projectType === '...', switch statements, case branches, if-checks across all of src/. Only src/commands/update-foundation.ts:163 has `case 'projectType':` which matches the SECTION NAME, not a value.

2. Consumers that read project.projectType:
   - src/commands/show-foundation.ts:130 → `Type: ${foundation.project.projectType || 'N/A'}` — displays verbatim
   - src/commands/discover-foundation.ts:265 → extracts `[DETECTED: X]` placeholder via regex (unaffected by enum removal)
   - src/generators/foundation-md.ts → does NOT reference projectType at all

3. Files that write projectType:
   - src/schemas/generic-foundation.schema.json lines 36-50 (the enum itself — REMOVE)
   - src/types/generic-foundation.ts lines 89-98 (the ProjectType union — CHANGE to `type ProjectType = string`)
   - src/utils/ensure-files.ts:208 (default 'cli-tool' — keep, still valid)
   - src/commands/discover-foundation.ts:127 (system-reminder prompt 'Options:' — change to 'Examples:')
   - src/commands/discover-foundation.ts:588 (placeholder '[DETECTED: cli-tool]' — keep, still valid)
   - src/commands/update-foundation.ts:163-166 (writes the value — ADD length validation)
   - 9 event-storm command files use `projectType: 'other' as const` for default foundation initialization — these keep working; the `as const` assertions become superfluous but remain valid TypeScript

4. Test fixtures: ~60 occurrences of `projectType: 'cli-tool'` (or 'web-app', 'library', etc.) across test files. All continue to work unchanged because these values are still valid strings.

5. Tests that specifically assert enum membership:
   - src/types/__tests__/foundation-schema.test.ts lines 129-140: tests `validProjectTypes: ProjectType[] = [...]`. NEEDS UPDATE — replace with test that any short string validates, plus minLength/maxLength boundary tests.

6. No test currently asserts that an invalid projectType enum value is rejected at schema level — verified via grep for 'projectType.*invalid|reject.*projectType'. Zero matches. So no negative tests to remove.

CONCLUSION: Removing the enum is a low-blast-radius change. Only 3 source files and 1 test file need modification (plus new tests for length-limit behavior). The remaining ~60 test fixtures work without modification.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. update-foundation must validate problemImpact against the enum (high, medium, low) at write time and return an actionable error listing valid options, matching the existing validation behavior but with improved error messages
  #   2. show-foundation must accept a --draft flag that reads spec/foundation.json.draft instead of spec/foundation.json, using the same rendering pipeline, and must error clearly if the draft does not exist
  #   3. discover-foundation (without --finalize) must detect an existing draft and emit an actionable error listing the three valid next steps: discover-foundation --finalize, show-foundation --draft, and discover-foundation --force
  #   4. update-foundation help text must describe the command as updating a field in foundation.json (not 'section content in FOUNDATION.md') and must enumerate the complete list of valid section names with no 'etc.' abbreviation
  #   5. update-foundation help text must explicitly note that capabilities and personas are managed via dedicated commands (add-capability, add-persona, remove-capability, remove-persona) and cannot be updated via update-foundation
  #   6. Standalone command: `fspec list-foundation-sections`. Evidence: 12 existing `list-*` standalone commands vs 1 `--list-sections` flag (on show-foundation). Standalone is the overwhelming convention. No new rule needed — this confirms rule #7 should specify the standalone-command form, not the flag form.
  #   7. NO fuzzy matching — just list valid values. Evidence: existing deterministic pattern used in update-work-unit-status.ts:119 (`Invalid status value: ${newStatus}. Allowed values: ${ALLOWED_STATES.join(', ')}`), update-work-unit-estimate.ts:35 (`Invalid estimate: ${options.estimate}. Must be one of: ${FIBONACCI_NUMBERS.join(',')}`), and work-unit.ts:737. Zero existing commands do 'did you mean X' suggestions on user input. The only similarity scoring in the codebase is scenario deduplication (generate-scenarios.ts, audit-scenarios.ts) which operates on Gherkin text, not user inputs. Deterministic format: `Invalid <field>: <value>. Valid values: <comma-separated-list>. Fix: fspec update-foundation <section> "<valid-value>"`.
  #   8. Hard error with actionable system-reminder, no draft content inline. Evidence: existing code at discover-foundation.ts:501-527 already follows this pattern — returns valid: false with a wrapped system-reminder. The improvement is to (a) make the message more concise, (b) add `fspec show-foundation --draft` as the observe-state option (which this work unit creates), and (c) keep the hard-error semantics so command purposes stay clean. show-foundation --draft is the dedicated command for viewing draft state; mixing it into discover-foundation's error path would duplicate functionality and confuse weaker LLMs.
  #   9. Hard error with actionable system-reminder. See rule [10] for full evidence and reasoning — this matches the existing convention at discover-foundation.ts:501-527 and keeps observability responsibility on the dedicated show-foundation --draft command.
  #   10. projectType is a freeform short string (1-30 characters), NOT an enum. The schema at src/schemas/generic-foundation.schema.json must use {minLength: 1, maxLength: 30} instead of an enum. The TypeScript type ProjectType becomes a plain string alias.
  #   11. update-foundation must validate projectType length at write time: reject empty strings and strings longer than 30 characters with an actionable error showing the actual length and fix command. Does NOT validate against any fixed list.
  #   12. discover-foundation --finalize must format Ajv validation errors (including enum, minLength, and maxLength keywords) to clearly distinguish invalid values from missing fields, showing the invalid value/length, the applicable constraint, and a copy-pasteable fix command
  #   13. The fspec list-foundation-sections standalone command must expose every valid section name with its JSON path and constraint info. For projectType it shows 'freeform string (1-30 characters)' with non-exhaustive examples. For problemImpact it shows the real enum 'high, medium, low'. For other string fields it shows 'freeform string'.
  #
  # EXAMPLES:
  #   1. Agent runs `fspec update-foundation projectType web-app` on a draft → command succeeds with confirmation message and chains to the next unfilled field system-reminder
  #   2. Agent runs `fspec update-foundation problemImpact critical` on a draft → command fails with error listing valid values (high, medium, low); draft unchanged
  #   3. Agent runs `fspec show-foundation --draft` while a draft exists → command displays the current draft content using the same formatting as show-foundation for the final file
  #   4. Agent runs `fspec show-foundation --draft` when no draft exists → command fails with clear error 'No draft found at spec/foundation.json.draft' and suggests running `fspec discover-foundation` to create one
  #   5. Agent runs `fspec discover-foundation` while a draft already exists → command fails with a three-option error listing `discover-foundation --finalize`, `show-foundation --draft`, and `discover-foundation --force` as actionable next steps
  #   6. Agent runs `fspec update-foundation --help` → output describes the command as updating fields in foundation.json (or the draft during discovery), lists all valid section names with their JSON paths, and explicitly notes that capabilities/personas use separate add-capability/add-persona commands
  #   7. Agent runs `fspec update-foundation projectType "saas-platform"` on a draft → command succeeds (22 characters, within the 30-char limit). The draft now contains `"projectType": "saas-platform"`. Previously this would have failed because 'saas-platform' was not in the 9-value enum.
  #   8. Agent runs `fspec update-foundation projectType ""` (empty string) on a draft → command fails with 'Invalid projectType: "" (must be 1-30 characters, got 0). Fix: fspec update-foundation projectType "<short-descriptor>"'. Draft is unchanged.
  #   9. Agent runs `fspec update-foundation projectType "a-very-long-project-type-descriptor-that-exceeds-the-limit"` (58 chars) → command fails with 'Invalid projectType: too long (must be 1-30 characters, got 58). Fix: fspec update-foundation projectType "<short-descriptor>"'. Draft is unchanged.
  #   10. Agent runs `fspec update-foundation projectType "browser-extension"` (17 chars) on a draft → command succeeds. This was not in the old enum but is a legitimate short descriptor.
  #   11. Agent manually edits draft to contain `projectType: "a-ridiculously-long-string-way-beyond-thirty-characters"` then runs `fspec discover-foundation --finalize` → command fails at finalization with 'Invalid value at project.projectType: maxLength exceeded (must be 1-30 characters, got 55). Fix: fspec update-foundation projectType "<short-descriptor>"' — NOT the misleading 'Missing required: project.projectType'. Draft file is NOT deleted. No spec/foundation.json is written.
  #   12. Agent runs `fspec list-foundation-sections` → output shows projectType as 'freeform string (1-30 characters), examples: cli-tool, web-app, saas-platform', shows problemImpact as 'enum: high, medium, low', shows other fields as 'freeform string'. Every section name is listed with its JSON path.
  #   13. Agent on a project whose foundation was previously finalized with `projectType: 'cli-tool'` runs `fspec update-foundation projectType "web-app"` → command succeeds (7 chars, valid); spec/foundation.json is rewritten; spec/FOUNDATION.md is regenerated; no discovery chaining system-reminder is emitted (not in discovery mode).
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we add `list-foundation-sections` as a standalone command, or as an `update-foundation --list-sections` flag (or both)? The standalone command is more discoverable; the flag is closer to existing patterns like `show-foundation --list-sections`.
  #   A: Hard error with actionable system-reminder, no draft content inline. Evidence: existing code at discover-foundation.ts:501-527 already follows this pattern — returns valid: false with a wrapped system-reminder. The improvement is to (a) make the message more concise, (b) add `fspec show-foundation --draft` as the observe-state option (which this work unit creates), and (c) keep the hard-error semantics so command purposes stay clean. show-foundation --draft is the dedicated command for viewing draft state; mixing it into discover-foundation's error path would duplicate functionality and confuse weaker LLMs.
  #
  #   Q: When `update-foundation projectType web-saas` fails, should the error suggest the closest valid match (e.g., 'Did you mean web-app?' via Levenshtein distance), or just list all valid values? The former is friendlier but adds complexity.
  #   A: NO fuzzy matching — just list valid values. Evidence: existing deterministic pattern used in update-work-unit-status.ts:119 (`Invalid status value: ${newStatus}. Allowed values: ${ALLOWED_STATES.join(', ')}`), update-work-unit-estimate.ts:35 (`Invalid estimate: ${options.estimate}. Must be one of: ${FIBONACCI_NUMBERS.join(',')}`), and work-unit.ts:737. Zero existing commands do 'did you mean X' suggestions on user input. The only similarity scoring in the codebase is scenario deduplication (generate-scenarios.ts, audit-scenarios.ts) which operates on Gherkin text, not user inputs. Deterministic format: `Invalid <field>: <value>. Valid values: <comma-separated-list>. Fix: fspec update-foundation <section> "<valid-value>"`.
  #
  #   Q: Should the improved 'draft exists' error message be a hard error (exit code 1, no changes) or a soft warning that still returns the draft content as a system-reminder? Hard error is safer; soft warning is more informative.
  #   A: Hard error with actionable system-reminder. See rule [10] for full evidence and reasoning — this matches the existing convention at discover-foundation.ts:501-527 and keeps observability responsibility on the dedicated show-foundation --draft command.
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec to create a project foundation
    I want to receive immediate, actionable feedback when I enter invalid values or need to observe draft state
    So that I don't waste turns debugging misleading errors at finalization or bypass fspec to read draft files directly

  Scenario: Accepting a valid projectType on a draft and chaining to next field
    Given a foundation draft exists at spec/foundation.json.draft
    And the draft contains an unfilled projectType placeholder
    When I run `fspec update-foundation projectType "web-app"`
    Then the command should exit with code 0
    And the draft file should contain `"projectType": "web-app"`
    And the response should include a system-reminder for the next unfilled field

  Scenario: Fail-fast rejection of invalid problemImpact at write time
    Given a foundation draft exists at spec/foundation.json.draft
    When I run `fspec update-foundation problemImpact "critical"`
    Then the command should exit with a non-zero code
    And the error output should contain `Invalid value for problemImpact: "critical"`
    And the error output should list valid values: high, medium, low
    And the error output should contain the text `Fix: fspec update-foundation problemImpact "<valid-value>"`
    And the draft file should be unchanged on disk

  Scenario: Show foundation draft when draft exists
    Given a foundation draft exists at spec/foundation.json.draft
    And no final foundation.json exists
    When I run `fspec show-foundation --draft`
    Then the command should exit with code 0
    And the output should display the draft contents rendered the same way as show-foundation renders a final foundation

  Scenario: Show foundation draft when no draft exists
    Given no foundation draft exists at spec/foundation.json.draft
    When I run `fspec show-foundation --draft`
    Then the command should exit with a non-zero code
    And the error output should contain `No draft found at spec/foundation.json.draft`
    And the error output should suggest running `fspec discover-foundation` to create one

  Scenario: Discover-foundation error when draft already exists
    Given a foundation draft exists at spec/foundation.json.draft
    When I run `fspec discover-foundation`
    Then the command should exit with a non-zero code
    And the response should include a system-reminder listing exactly three next-step options
    And the system-reminder should contain `fspec discover-foundation --finalize`
    And the system-reminder should contain `fspec show-foundation --draft`
    And the system-reminder should contain `fspec discover-foundation --force`
    And the response should NOT include the raw draft content inline

  Scenario: Update-foundation help describes JSON field updates and lists all sections
    When I run `fspec update-foundation --help`
    Then the command should exit with code 0
    And the output should describe the command as updating a field in foundation.json
    And the output should NOT contain the phrase `section content in FOUNDATION.md`
    And the output should list all valid section names: projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
    And the output should NOT use the abbreviation `etc.` when listing section names
    And the output should explicitly note that capabilities use the `add-capability` command
    And the output should explicitly note that personas use the `add-persona` command

  Scenario: Accepting a freeform projectType that was previously rejected as not-in-enum
    Given a foundation draft exists at spec/foundation.json.draft
    And the draft contains an unfilled projectType placeholder
    When I run `fspec update-foundation projectType "saas-platform"`
    Then the command should exit with code 0
    And the draft file should contain `"projectType": "saas-platform"`
    And the response should include a system-reminder for the next unfilled field

  Scenario: Rejecting an empty projectType string
    Given a foundation draft exists at spec/foundation.json.draft
    When I run `fspec update-foundation projectType ""`
    Then the command should exit with a non-zero code
    And the error output should contain `Invalid projectType: "" (must be 1-30 characters, got 0)`
    And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
    And the draft file should be unchanged on disk

  Scenario: Rejecting a projectType longer than 30 characters
    Given a foundation draft exists at spec/foundation.json.draft
    When I run `fspec update-foundation projectType "a-very-long-project-type-descriptor-that-exceeds-the-limit"`
    Then the command should exit with a non-zero code
    And the error output should contain `Invalid projectType: too long (must be 1-30 characters, got 58)`
    And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
    And the draft file should be unchanged on disk

  Scenario: Accepting a freeform projectType like browser-extension
    Given a foundation draft exists at spec/foundation.json.draft
    And the draft contains an unfilled projectType placeholder
    When I run `fspec update-foundation projectType "browser-extension"`
    Then the command should exit with code 0
    And the draft file should contain `"projectType": "browser-extension"`

  Scenario: Finalize fails with actionable length error when draft contains an overlong projectType
    Given a foundation draft exists at spec/foundation.json.draft
    And the draft contains all required fields filled with no placeholders
    And the draft contains `"projectType": "a-ridiculously-long-string-way-beyond-thirty-characters"` written by manual edit
    When I run `fspec discover-foundation --finalize`
    Then the command should exit with a non-zero code
    And the error output should contain `Invalid value at project.projectType`
    And the error output should contain `maxLength exceeded`
    And the error output should contain `must be 1-30 characters, got 55`
    And the error output should contain the text `Fix: fspec update-foundation projectType "<short-descriptor>"`
    And the error output should NOT contain `Missing required: project.projectType`
    And the draft file should NOT be deleted
    And no spec/foundation.json should be written

  Scenario: Discover valid foundation sections via list-foundation-sections
    When I run `fspec list-foundation-sections`
    Then the command should exit with code 0
    And the output should list every valid section name
    And the output should show each section's JSON path
    And the output should describe `projectType` as `freeform string (1-30 characters)`
    And the output should include non-exhaustive examples for projectType: cli-tool, web-app, saas-platform
    And the output should describe `problemImpact` as `enum: high, medium, low`
    And the output should describe other text fields as `freeform string`

  Scenario: Update-foundation on final foundation accepts a valid freeform projectType and regenerates markdown
    Given no foundation draft exists
    And a final foundation.json exists with `"projectType": "cli-tool"`
    When I run `fspec update-foundation projectType "web-app"`
    Then the command should exit with code 0
    And the file spec/foundation.json should contain `"projectType": "web-app"`
    And the file spec/FOUNDATION.md should be regenerated
    And the response should NOT include a discovery chaining system-reminder

  Scenario: Update-foundation on final foundation rejects an overlong projectType
    Given no foundation draft exists
    And a final foundation.json exists with `"projectType": "cli-tool"`
    When I run `fspec update-foundation projectType "a-very-long-project-type-descriptor-that-exceeds-the-limit"`
    Then the command should exit with a non-zero code
    And the error output should contain `Invalid projectType: too long (must be 1-30 characters, got 58)`
    And the file spec/foundation.json should be unchanged on disk
    And the file spec/FOUNDATION.md should NOT be regenerated

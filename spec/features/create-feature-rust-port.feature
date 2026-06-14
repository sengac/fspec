@done
@feature-management
@cli
@RPC-212
Feature: Port create-feature command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/create_feature.rs: run(args_json,&Path) with args {name}. Helpers to_kebab_case + feature_template (verbatim TS template incl. trailing newline). Coverage written via crate::types::coverage::{CoverageFile,CoverageScenario,CoverageStats}; scenario names line-scanned for 'Scenario:'/'Scenario Outline:'. JSON.stringify(null,2) parity = serde_json::to_string_pretty.
  Prefill detection + file-naming reminder ported inline (no shared io util exists in fspec-core). Reminders honour FSPEC_DISABLE_REMINDERS=1 (suppressed). Response envelope mirrors TS CreateFeatureResult {filePath, prefillDetection{hasPrefill,matches,systemReminder?}, coverageFile{created,path?,status,message}, fileNamingReminder?}; CLI bridge prints ✓ lines + reminders.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Filename is toKebabCase(name) + '.feature' under spec/features/ (lowercase, non-alphanumeric runs → single hyphen, no leading/trailing hyphen)
  #   2. If the target feature file already exists the command MUST fail with 'File already exists: spec/features/<file>\nSuggestion: Use a different name or delete the existing file' and write nothing
  #   3. The spec/features/ directory MUST be created recursively if absent before writing
  #   4. File content MUST be the exact generateFeatureTemplate(name) output: '@critical @component @feature-group' tag line, 'Feature: <name>', an architecture-notes docstring with 3 TODO lines, a Background User Story with [role]/[action]/[benefit], and one placeholder Scenario, ending with a trailing newline
  #   5. A sidecar '<file>.feature.coverage' MUST be created with one CoverageScenario per scenario name in the template (name '[Scenario name]'), empty testMappings, and a stats block (totalScenarios=1, coveredScenarios=0, coveragePercent=0)
  #   6. Coverage-file creation MUST degrade gracefully: a failure there sets coverageFile.status='error' with a 'Warning: Failed to create coverage file: <msg>' message but MUST NOT fail feature creation
  #   7. Prefill detection MUST run on the written content and report hasPrefill=true with a systemReminder (the template carries [role]/[action]/[benefit]/[precondition]/[expected outcome]/[scenario name]/TODO placeholders)
  #   8. A file-naming reminder MUST be emitted only when the kebab name is task-based (^implement-/add-/create-/fix-/build-/setup-/update- or ^[A-Z]+-\d+); otherwise omitted
  #
  # EXAMPLES:
  #   1. creating 'User Authentication' writes spec/features/user-authentication.feature plus user-authentication.feature.coverage and returns hasPrefill=true with no file-naming reminder
  #   2. creating 'User Authentication' again when the file exists fails with 'File already exists: spec/features/user-authentication.feature'
  #   3. creating 'Implement Login' produces spec/features/implement-login.feature and ALSO emits a file-naming anti-pattern reminder because 'implement-' is task-based
  #   4. CLI 'fspec create-feature "Payment Processing"' prints '✓ Created spec/features/payment-processing.feature', the coverage '✓ Created ...' line, exits 0, and the prefill system-reminder appears on stdout
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to create a new Gherkin feature file with the standard template from a capability name
    So that I can scaffold ACDD specifications without depending on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Creates feature file and coverage sidecar from a capability name
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='User Authentication'
    Then the dispatcher returns a filePath ending with 'spec/features/user-authentication.feature'
    And the file spec/features/user-authentication.feature exists on disk
    And the sidecar spec/features/user-authentication.feature.coverage exists on disk

  Scenario: Generated content matches the canonical template verbatim
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='User Authentication'
    Then the written file begins with the line '@critical @component @feature-group'
    And the written file contains the line 'Feature: User Authentication'
    And the written file contains the placeholder steps '[precondition]', '[action]', and '[expected outcome]'
    And the written file ends with a trailing newline

  Scenario: Coverage sidecar carries one empty scenario mapping with zeroed stats
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='User Authentication'
    Then the coverage sidecar parses to one scenario named '[Scenario name]' with empty testMappings
    And the coverage sidecar stats report totalScenarios=1, coveredScenarios=0, and coveragePercent=0

  Scenario: Prefill detection reports placeholders in the template
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='User Authentication'
    Then the dispatcher response reports prefillDetection.hasPrefill = true
    And the dispatcher response carries a prefill systemReminder string

  Scenario: Capability-style name emits no file-naming reminder
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='User Authentication'
    Then the dispatcher response has no fileNamingReminder field

  Scenario: Task-style name emits a file-naming anti-pattern reminder
    Given a project root tempdir with an empty spec directory
    When I dispatch create-feature with name='Implement Login'
    Then the dispatcher returns a filePath ending with 'spec/features/implement-login.feature'
    And the dispatcher response carries a fileNamingReminder string mentioning capabilities

  Scenario: Creating an existing feature file fails without overwriting
    Given a project root tempdir whose spec/features/user-authentication.feature already exists with body 'KEEP ME\n'
    When I dispatch create-feature with name='User Authentication'
    Then the dispatcher fails with an error containing 'File already exists: spec/features/user-authentication.feature'
    And the file spec/features/user-authentication.feature still contains 'KEEP ME'

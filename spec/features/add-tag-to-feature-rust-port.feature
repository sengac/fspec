@done
@RPC-193
Feature: Port add-tag-to-feature command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_tag_to_feature.rs reuses crate::io::gherkin::parse_feature_lenient for parsing and a TS-parity line-based insertion (NOT AST round-trip).
  Registry validation reuses crate::types::tags::TagsData (already public) — flat tag set is built from categories[].tags[].name.
  System reminders are emitted into the dispatcher's JSON response under systemReminders + consolidated systemReminder fields, matching the TS shape; CLI bridge prints the consolidated block after the success line.
  Two-front-doors: bridge marshals positional <file> + variadic <tags> + optional --validate-registry into JSON {file, tags, validateRegistry?} only; NO domain logic in the bridge.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Missing target file MUST surface success=false with error message 'File not found: <relPath>'
  #   2. Every input tag MUST start with '@' OR error is 'Invalid tag format. Tags must start with @'
  #   3. Tags MUST match work-unit pattern '^@[A-Z]{2,6}-\d+$' OR regular pattern '^@[a-z0-9-#]+$'
  #   4. Adding a tag that already exists on the feature MUST error with 'Tag <tag> already exists on this feature'
  #   5. When validateRegistry=true the command MUST reject any tag not present in spec/tags.json categories[].tags[].name
  #   6. Tag insertion MUST use the TS line-based algorithm: append new tags AFTER existing tags when present, or insert directly before the Feature header otherwise
  #   7. New tags MUST be written one per line preserving the file's existing newline scheme (\n join), and the file is always written even when post-validation fails (valid=false advisory)
  #   8. Without validateRegistry, the dispatcher MUST emit a system-reminder for each unregistered NON-work-unit tag
  #   9. The dispatcher MUST emit a missing-required-tags reminder when neither a known component tag nor a known feature-group tag is present after the insertion
  #   10. Success message MUST be 'Added <comma-space-tags> to <relPath>' verbatim
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-tag-to-feature from the agent loop and run `fspec add-tag-to-feature` from a shell with byte-for-byte parity to the TypeScript implementation
    So that I can append feature-level tags without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Adds a single tag to a feature with no existing tags
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added @critical to spec/features/login.feature'
    And the file on disk starts with the line '@critical' followed by the 'Feature: Login' line

  Scenario: Adds multiple tags in a single call preserving order
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical','@auth']
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added @critical, @auth to spec/features/login.feature'
    And the file on disk contains the line '@critical' immediately followed by the line '@auth' above 'Feature: Login'

  Scenario: Missing feature file surfaces the canonical not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch add-tag-to-feature with file='spec/features/missing.feature' and tags=['@critical']
    Then the dispatcher returns success=false
    And the error message equals 'File not found: spec/features/missing.feature'

  Scenario: Rejects input tag missing the leading at-sign
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['critical']
    Then the dispatcher returns success=false
    And the error message equals 'Invalid tag format. Tags must start with @'

  Scenario: Rejects mixed-case tag that fails both allowed regexes
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@MIXEDcase']
    Then the dispatcher returns success=false
    And the error message contains 'Regular tags must use lowercase-with-hyphens, work unit tags must match @[A-Z]{2,6}-\d+'

  Scenario: Rejects duplicate tag already present on the feature
    Given a project root tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    Then the dispatcher returns success=false
    And the error message equals 'Tag @critical already exists on this feature'

  Scenario: validateRegistry=true rejects a tag not in spec/tags.json
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    And spec/tags.json contains the canonical 9-category default with NO '@unregistered' tag
    When I dispatch add-tag-to-feature with file='spec/features/login.feature', tags=['@unregistered'], validateRegistry=true
    Then the dispatcher returns success=false
    And the error message equals 'Tag @unregistered is not registered in spec/tags.json'

  Scenario: Appends new tag AFTER existing tag block when prior tags are present
    Given a project root tempdir with spec/features/login.feature containing '@auth\n@security\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    Then the dispatcher returns success=true
    And the file on disk contains the lines '@critical', '@auth', '@security' in that order immediately above 'Feature: Login'

  Scenario: Inserts new tag at top of file when there are no existing tags
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    Then the dispatcher returns success=true
    And the first line of the file on disk is '@critical'
    And the second line of the file on disk is 'Feature: Login'

  Scenario: Without validateRegistry emits a system-reminder for an unregistered non-work-unit tag
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    And spec/tags.json contains the canonical 9-category default with NO '@unknown' tag
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@unknown']
    Then the dispatcher returns success=true
    And the dispatcher response includes a systemReminder containing 'is not registered in spec/tags.json'

  Scenario: Work-unit form tag without registry validation does NOT emit unregistered-tag reminder
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    And spec/tags.json contains the canonical 9-category default with NO '@AUTH-001' tag
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@AUTH-001']
    Then the dispatcher returns success=true
    And the dispatcher response does NOT include a systemReminder containing '@AUTH-001 is not registered'

  Scenario: Emits missing-required-tags reminder when neither component nor feature-group is present after insert
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    Then the dispatcher returns success=true
    And the dispatcher response includes a systemReminder mentioning 'component' and 'feature-group'

  Scenario: No missing-required-tags reminder when component and feature-group tags are both supplied
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@cli','@feature-management']
    Then the dispatcher returns success=true
    And the dispatcher response does NOT include a systemReminder mentioning missing required tags

@done
@RPC-281
Feature: Port remove-tag-from-feature command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/remove_tag_from_feature.rs reuses crate::io::gherkin::parse_feature_lenient for the existence pre-check and a TS-parity whole-line filter for the actual removal pass.
  No registry validation. No system reminders. Strict parity with TS removeTagFromFeature which returns {success, valid, message?, error?}.
  Two-front-doors: bridge marshals positional <file> + variadic <tags> into JSON {file, tags} only; NO domain logic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Missing target file MUST surface success=false with error message 'File not found: <relPath>'
  #   2. Invalid Gherkin / missing Feature header MUST error with the canonical TS message
  #   3. Removing a tag that is NOT on the feature MUST error with 'Tag <tag> not found on this feature' and the file is left untouched
  #   4. Removal MUST be a whole-line filter: any line whose trim() equals an input tag exactly is dropped
  #   5. The file MUST always be written after a successful removal (valid=false is advisory)
  #   6. Success message MUST be 'Removed <comma-space-tags> from <relPath>' verbatim
  #   7. Multi-tag-on-one-line is NOT split — a line containing '@a @b' is dropped only when the trimmed full line equals an input tag (documented divergence from TS preserved)
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch remove-tag-from-feature from the agent loop and run `fspec remove-tag-from-feature` from a shell with byte-for-byte parity to the TypeScript implementation
    So that I can remove feature-level tags without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Removes a single tag from a feature file
    Given a project root tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Removed @wip from spec/features/login.feature'
    And the file on disk does NOT contain a line whose trimmed value is '@wip'
    And the file on disk still contains the line 'Feature: Login'

  Scenario: Removes multiple tags in a single call
    Given a project root tempdir with spec/features/login.feature containing '@wip\n@draft\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip','@draft']
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Removed @wip, @draft from spec/features/login.feature'
    And the file on disk does NOT contain a line whose trimmed value is '@wip'
    And the file on disk does NOT contain a line whose trimmed value is '@draft'

  Scenario: Missing target file surfaces the canonical not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch remove-tag-from-feature with file='spec/features/missing.feature' and tags=['@wip']
    Then the dispatcher returns success=false
    And the error message equals 'File not found: spec/features/missing.feature'

  Scenario: Removing an absent tag leaves the file untouched
    Given a project root tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@notthere']
    Then the dispatcher returns success=false
    And the error message equals 'Tag @notthere not found on this feature'
    And the file on disk is byte-equal to its pre-call contents

  Scenario: Source without a Feature header is rejected
    Given a project root tempdir with spec/features/login.feature containing 'just some text\n# no feature header here\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    Then the dispatcher returns success=false
    And the error message contains 'File does not contain a valid Feature'

  Scenario: Removed tag leaves other tags untouched in their original positions
    Given a project root tempdir with spec/features/login.feature containing '@critical\n@wip\n@auth\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    Then the dispatcher returns success=true
    And the file on disk contains the line '@critical' immediately followed by the line '@auth' above 'Feature: Login'

  Scenario: Multi-tag-on-one-line passes the existence check but is preserved on disk (whole-line equality only)
    Given a project root tempdir with spec/features/login.feature containing '@a @b\nFeature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@a']
    Then the dispatcher returns success=true
    And the file on disk still contains a line whose trimmed value is '@a @b' (documented TS divergence — whole-line equality removal does not split multi-tag lines)

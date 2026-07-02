@done
@feature-management
@parser
@RPC-221
Feature: Port delete-step command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/delete_step.rs uses crate::io::gherkin::parse_feature_lenient for parse + re-validate; gherkin-0.16 Step.keyword includes a trailing space and Step.value is the text, so full step text = keyword + value; Step.position.line (1-based) locates the removed line. Line-based split('\n')/join('\n') edit.
  Recoverable failures returned as inner JSON envelope {success:false,error}; success {success:true,message}. CLI bridge prints '✓ <message>' / 'Error: <error>' to stderr + exit 1. Two-front-doors: bridge marshals positional <feature> <scenario> <step> into {feature, scenario, step} JSON only; no domain logic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Feature path resolves: ends-with .feature OR starts-with spec/features/ → join(cwd, feature); else join(cwd, 'spec/features', feature + '.feature')
  #   2. Missing target file MUST return success=false, error 'Feature file not found: <absPath>'
  #   3. Unparseable Gherkin MUST return 'Invalid Gherkin syntax: <msg>'; featureless file MUST return 'Feature file does not contain a valid Feature'
  #   4. A scenario name with no match MUST return "Scenario '<name>' not found in feature file"
  #   5. A step is matched when arg equals the step text OR equals (keyword + text).trim(); no match MUST return "Step '<step>' not found in scenario '<name>'"
  #   6. Only the single matched step line (step.position.line) is removed; consecutive blank lines collapse to at most 2; split/join on '\n'
  #   7. If the post-deletion content fails to re-parse, MUST return 'Deletion would result in invalid Gherkin: <msg>' and NOT write the file
  #   8. On success message = "Successfully deleted step from scenario '<name>' in <fileName>"; delete-step does NOT touch any coverage sidecar
  #
  # EXAMPLES:
  #   1. Dispatcher deletes the step 'When I enter valid credentials' from scenario 'Login': success=true, that step line gone, surrounding steps intact
  #   2. Matching by bare text 'I enter valid credentials' (no keyword) also deletes the step
  #   3. Deleting a step text that does not exist returns success=false, error "Step '<step>' not found in scenario 'Login'"
  #   4. Deleting a step from a missing scenario returns success=false, error "Scenario 'Ghost' not found in feature file"
  #   5. CLI: `fspec delete-step spec/features/login.feature Login "When I enter valid credentials"` exits 0 and prints '✓ Successfully deleted step'
  #   6. CLI on a missing step exits 1 with stderr 'Error:' prefix
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to delete a single step from a named scenario in a feature file via both the LLM dispatcher and the shell CLI
    So that I can prune obsolete steps with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: Dispatcher deletes a step matched by full step text
    Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='When I enter valid credentials'
    Then the dispatcher returns success=true
    And the file on disk no longer contains 'When I enter valid credentials'
    And the file on disk still contains the surrounding Given and Then steps

  Scenario: Dispatcher deletes a step matched by bare text without keyword
    Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='I enter valid credentials'
    Then the dispatcher returns success=true
    And the file on disk no longer contains 'I enter valid credentials'

  Scenario: Dispatcher reports a missing step
    Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='When nonexistent'
    Then the dispatcher returns success=false
    And the dispatcher error equals "Step 'When nonexistent' not found in scenario 'Login'"

  Scenario: Dispatcher reports a missing scenario
    Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I dispatch delete-step with feature='spec/features/login.feature', scenario='Ghost' and step='When I enter valid credentials'
    Then the dispatcher returns success=false
    And the dispatcher error equals "Scenario 'Ghost' not found in feature file"

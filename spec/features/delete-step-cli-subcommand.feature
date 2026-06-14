@done
@feature-management
@cli
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

  Scenario: CLI deletes a step and prints the success line
    Given a tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I run 'fspec delete-step spec/features/login.feature Login "When I enter valid credentials"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully deleted step'

  Scenario: CLI surfaces a missing step with stderr Error prefix and exit 1
    Given a tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I run 'fspec delete-step spec/features/login.feature Login "When nonexistent"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec delete-step --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/delete-step.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    When I delete the same step once via the dispatcher and once via the CLI on identical inputs
    Then both front doors produce the same resulting feature-file content

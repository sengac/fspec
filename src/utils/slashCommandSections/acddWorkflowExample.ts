export function getAcddWorkflowExampleSection(): string {
  return `## Step 5: Complete ACDD Workflow Example

Here's the complete ACDD flow from backlog to done:

\`\`\`bash
# 1. SELECT WORK
fspec board                                      # View Kanban
fspec show-work-unit EXAMPLE-006                      # Review details
fspec update-work-unit-status EXAMPLE-006 specifying  # Move to specifying

# 2. DISCOVERY (Example Mapping - Interactive Conversation)
# Start with user story (yellow card) from work unit description

# STEP 0: Capture user story to avoid prefill placeholders
fspec set-user-story EXAMPLE-006 \\
  --role "developer using fspec" \\
  --action "validate feature files automatically" \\
  --benefit "I catch syntax errors before committing"

# Ask about rules (blue cards)
# You: "What are the key business rules for feature validation?"
# Human: "Validation must complete within 2 seconds and report specific syntax errors"
fspec add-rule EXAMPLE-006 "Validation must complete within 2 seconds"
fspec add-rule EXAMPLE-006 "Validation must report specific line numbers for syntax errors"

# Ask about examples (green cards)
# You: "Can you give me concrete examples of how this should work?"
# Human: "Running 'example-project validate' should display 'All feature files are valid'"
fspec add-example EXAMPLE-006 "User runs 'example-project validate' with no args, sees 'All feature files are valid'"
fspec add-example EXAMPLE-006 "User runs 'example-project validate test.feature', sees validation result for single file"

# Ask questions (red cards) when uncertain
# You: "What happens if a feature file has multiple syntax errors?"
fspec add-question EXAMPLE-006 "@human: What happens if a feature file has multiple syntax errors?"
# Human: "Report all errors, don't stop at the first one"
fspec answer-question EXAMPLE-006 0 --answer "Report all errors in the file, not just the first one"

# You: "Should we support custom validation rules?"
fspec add-question EXAMPLE-006 "@human: Should we support custom validation rules in config?"
# Human: "Not in Phase 1, defer to EXAMPLE-006"
fspec answer-question EXAMPLE-006 1 --answer "Not in Phase 1, add to backlog as EXAMPLE-006"

# Check for consensus
# You: "Do we have shared understanding? Any remaining questions?"
# Human: "Yes, looks clear!"

fspec show-work-unit EXAMPLE-006                      # Review complete example map

# 3. SPECIFY (Generate or Write the Feature)
fspec generate-scenarios EXAMPLE-006                  # Auto-generate from example map
# OR manually:
# fspec create-feature "Feature File Validation"
# fspec add-scenario feature-file-validation "Validate feature file with valid syntax"

fspec add-tag-to-feature spec/features/example-feature.feature @wip
example-project validate                                   # Ensure valid Gherkin

fspec update-work-unit-status EXAMPLE-006 testing    # Move to testing

# 4. TEST (Write the Test - BEFORE any implementation code)
# Create: src/__tests__/validate.test.ts (lines 45-62)
#
# CRITICAL: Add feature file reference at top of test file:
# /**
#  * Feature: spec/features/example-feature.feature
#  *
#  * This test file validates the acceptance criteria defined in the feature file.
#  * Scenarios in this test map directly to scenarios in the Gherkin feature.
#  */
#
# Then write tests that map to Gherkin scenarios:
# describe('Feature: Feature File Validation', () => {
#   describe('Scenario: Validate feature file with valid syntax', () => {
#     it('should exit with code 0 when feature file is valid', async () => {
#       // Given: A feature file with valid Gherkin syntax
#       // When: User runs 'example-project validate'
#       // Then: Validation passes and reports success
#     });
#   });
# });

npm test                                         # Tests MUST FAIL (red phase)
                                                 # If tests pass, you wrote code already!

# IMMEDIATELY link test to scenario
fspec link-coverage example-feature --scenario "Validate feature file with valid syntax" \\
  --test-file src/__tests__/validate.test.ts --test-lines 45-62

fspec update-work-unit-status EXAMPLE-006 implementing # Move to implementing

# 5. IMPLEMENT (Write minimal code to make tests pass)
# Create: src/commands/validate.ts (lines 10-24)
# Write ONLY enough code to make the tests pass

npm test                                         # Tests MUST PASS (green phase)
                                                 # Refactor if needed, keep tests green

# IMMEDIATELY link implementation to test mapping
fspec link-coverage example-feature --scenario "Validate feature file with valid syntax" \\
  --test-file src/__tests__/validate.test.ts \\
  --impl-file src/commands/validate.ts --impl-lines 10-24

fspec update-work-unit-status EXAMPLE-006 validating # Move to validating

# 6. VALIDATE (Run ALL tests + quality checks)
1. command(s) that run the tests                           # Run ALL tests (ensure nothing broke)
2. command(s) that run code quality checking               # Ensure the code reaches a quality standard
3. example-project validate                                # Gherkin syntax validation
4. example-project validate-tags                           # Tag compliance check
5. fspec update-work-unit-status EXAMPLE-006 done          # Move to done

# 7. COMPLETE (Update feature file tags)
fspec remove-tag-from-feature spec/features/example-feature.feature @wip
fspec add-tag-to-feature spec/features/example-feature.feature @done

fspec board                                      # Verify work unit in DONE column
\`\`\`

### Critical ACDD Rules in This Example

1. **Discovery FIRST** - Example Mapping conversation to clarify requirements (rules, examples, questions)
2. **Generate/Write Feature SECOND** - Use \`fspec generate-scenarios\` or manually create feature file
3. **Test THIRD** - \`validate.test.ts\` created with feature file link in header comment
4. **Tests FAIL** - Run \`npm test\` and verify tests fail (proves they test real behavior)
5. **Implement FOURTH** - \`validate.ts\` written with minimal code to pass tests
6. **Tests PASS** - Run \`npm test\` and verify tests now pass (green)
7. **Validate ALL** - Run \`npm test\` again to ensure ALL tests still pass (nothing broke)
8. **Tags Updated** - Remove \`@wip\`, add \`@done\` when complete

`;
}

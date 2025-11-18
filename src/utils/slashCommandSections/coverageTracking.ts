export function getCoverageTrackingSection(): string {
  return `## Step 10: Coverage Tracking - Link Tests and Implementation

**CRITICAL**: After writing tests and implementation, you MUST update coverage files to maintain traceability. Coverage files (\`*.feature.coverage\`) link Gherkin scenarios to their test files and implementation files.

### Why Coverage Tracking Matters

- **Traceability**: Know exactly which tests validate which scenarios
- **Implementation Tracking**: See which code implements which acceptance criteria
- **Gap Detection**: Identify uncovered scenarios or untested code
- **Reverse ACDD**: Essential for reverse engineering existing codebases (see \`fspec reverse --help\`)
- **Refactoring Safety**: Understand impact of code changes on scenarios

### Coverage Commands

\`\`\`bash
# Link test file to scenario (after writing tests)
fspec link-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --test-lines <range>

# Link implementation to existing test mapping (after implementing)
fspec link-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --impl-file <path> --impl-lines <lines>

# Link both test and implementation at once
fspec link-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --test-lines <range> --impl-file <path> --impl-lines <lines>

# Show coverage for a feature (see what's mapped)
fspec show-coverage <feature-name>
fspec show-coverage <feature-name> --format=json

# Show all feature coverage (project-wide)
fspec show-coverage

# Audit coverage (verify files exist)
fspec audit-coverage <feature-name>
\`\`\`

### Coverage Workflow Integration

**Update your ACDD workflow to include coverage tracking:**

\`\`\`bash
# 4. TEST (Write the Test - BEFORE any implementation code)
# Create: src/__tests__/validate.test.ts (lines 45-62)
# MANDATORY: Include @step comments for EVERY Gherkin step
# Use language-appropriate syntax:
#   JavaScript: // @step Given I am logged in
#   Python:     # @step When I click the logout button
#   SQL:        -- @step Then I should see the login page
<test-command>  # Tests MUST FAIL (red phase)

# IMMEDIATELY link test to scenario
# NOTE: link-coverage validates that @step comments exist and match Gherkin steps
fspec link-coverage user-authentication --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth.test.ts --test-lines 45-62

fspec update-work-unit-status EXAMPLE-006 implementing

# 5. IMPLEMENT (Write minimal code to make tests pass)
# Create: src/commands/validate.ts (lines 10,11,12,23,24)
<test-command>  # Tests MUST PASS (green phase)

# IMMEDIATELY link implementation to test mapping
fspec link-coverage user-authentication --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth.test.ts \\
  --impl-file src/auth/login.ts --impl-lines 10-24

# 6. VERIFY COVERAGE
fspec show-coverage user-authentication
# Should show: ✅ Login with valid credentials (FULLY COVERED)
# - Test: src/__tests__/auth.test.ts:45-62
# - Implementation: src/auth/login.ts:10,11,12,23,24
\`\`\`

### When to Update Coverage

✅ **IMMEDIATELY after**:
1. Writing test file (link test to scenario)
2. Implementing code (link implementation to test mapping)
3. Refactoring (update line numbers if they change)
4. Adding new scenarios (coverage file auto-created, but needs linking)

❌ **DON'T**:
- Wait until end of work unit to update coverage
- Skip coverage linking (breaks traceability)
- Manually edit \`.coverage\` files (always use \`fspec link-coverage\`)

### Coverage File Format

Coverage files (\`*.feature.coverage\`) are JSON files automatically created when you run \`fspec create-feature\`. They contain:

\`\`\`json
{
  "scenarios": [
    {
      "name": "Login with valid credentials",
      "testMappings": [
        {
          "file": "src/__tests__/auth.test.ts",
          "lines": "45-62",
          "implMappings": [
            {
              "file": "src/auth/login.ts",
              "lines": [10, 11, 12, 23, 24]
            }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 5,
    "coveredScenarios": 1,
    "coveragePercent": 20,
    "testFiles": ["src/__tests__/auth.test.ts"],
    "implFiles": ["src/auth/login.ts"],
    "totalLinesCovered": 23
  }
}
\`\`\`

### Automatic Coverage Synchronization

Coverage files are automatically synchronized when scenarios are modified:

- **Delete scenario** (\`fspec delete-scenario\`): Automatically removes scenario from coverage file and recalculates stats
- **Bulk delete** (\`fspec delete-scenarios-by-tag\`): Removes all deleted scenarios from coverage files
- **Rename scenario** (\`fspec update-scenario\`): Preserves all test mappings and implementation mappings during rename
- **Generate coverage** (\`fspec generate-coverage\`): Removes stale scenarios (in coverage but not in feature file)

**Key behaviors**:
- Coverage statistics (totalScenarios, coveredScenarios, coveragePercent) are automatically recalculated
- Use \`update-scenario\` instead of manual delete + create to preserve coverage links
- Stale scenarios are detected during validation and prompt you to run \`generate-coverage\`

### Coverage Best Practices

1. **Update immediately** - Link coverage as soon as tests/code are written
2. **Use update-scenario for renames** - Preserves test mappings (don't delete + recreate)
3. **Check coverage gaps** - Run \`fspec show-coverage\` regularly to find uncovered scenarios
4. **Use audit** - Run \`fspec audit-coverage <feature>\` to verify file paths are correct
5. **Track changes** - When refactoring changes line numbers, update coverage mappings
6. **Project-wide view** - Run \`fspec show-coverage\` (no arguments) to see all features at once
7. **Sync stale coverage** - Run \`fspec generate-coverage\` if validation detects stale scenarios

`;
}

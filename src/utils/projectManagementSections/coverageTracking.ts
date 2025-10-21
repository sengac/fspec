export function getCoverageTrackingSection(): string {
  return `## Coverage Tracking: Linking Specs, Tests, and Implementation

**CRITICAL**: fspec provides a coverage tracking system that links Gherkin scenarios to test files and implementation code. This is ESSENTIAL for:

1. **Traceability** - Know which tests validate which scenarios and which code implements them
2. **Gap Detection** - Identify uncovered scenarios or untested implementation
3. **Reverse ACDD** - Critical for reverse engineering existing codebases (use \`fspec reverse\`)
4. **Refactoring Safety** - Understand impact of code changes on scenarios
5. **Documentation** - Maintain living documentation of what code does what

### Coverage File Format

Every \`.feature\` file has a corresponding \`.feature.coverage\` JSON file that tracks:
- Which scenarios have test coverage
- Line ranges in test files
- Which implementation files and lines are tested
- Coverage statistics

**Example structure:**
\`\`\`json
{
  "scenarios": [
    {"name": "Scenario", "testMappings": [
      {"file": "test.ts", "lines": "45-62", "implMappings": [
        {"file": "impl.ts", "lines": [10, 23]}
      ]}
    ]}
  ],
  "stats": {"totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50}
}
\`\`\`

### Coverage Commands

\`\`\`bash
# Generate or update coverage files (creates new files + updates existing ones with missing scenarios)
fspec generate-coverage
fspec generate-coverage --dry-run  # Preview what would be created/updated

# Link test file to scenario (after writing tests)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \\
  --test-file <path> --test-lines <range>

# Link implementation to existing test mapping (after implementing)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \\
  --test-file <path> --impl-file <path> --impl-lines <lines>

# Link both at once
fspec link-coverage <feature-name> --scenario "<scenario-name>" \\
  --test-file <path> --test-lines <range> \\
  --impl-file <path> --impl-lines <lines>

# Remove coverage mappings (fix mistakes)
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --all
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path>
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --impl-file <path>

# Show coverage for a feature
fspec show-coverage <feature-name>

# Show all feature coverage (project-wide)
fspec show-coverage

# Audit coverage (verify files exist)
fspec audit-coverage <feature-name>
\`\`\`

### Coverage Workflow in ACDD

**Integrate coverage tracking into your ACDD workflow:**

\`\`\`bash
# AFTER writing tests (testing phase)
npm test  # Tests MUST FAIL (red phase)

# IMMEDIATELY link test to scenario
fspec link-coverage user-authentication --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth.test.ts --test-lines 45-62

# AFTER implementing code (implementing phase)
npm test  # Tests MUST PASS (green phase)

# IMMEDIATELY link implementation to test mapping
fspec link-coverage user-authentication --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth.test.ts \\
  --impl-file src/auth/login.ts --impl-lines 10-24

# Verify coverage
fspec show-coverage user-authentication
# Output:
# ✅ Login with valid credentials (FULLY COVERED)
# - Test: src/__tests__/auth.test.ts:45-62
# - Implementation: src/auth/login.ts:10,11,12,23,24
\`\`\`

### When to Update Coverage

✅ **IMMEDIATELY after**:
1. Writing test file → Link test to scenario
2. Implementing code → Link implementation to test mapping
3. Refactoring → Update line numbers if they change
4. Adding new scenarios → Coverage file auto-created, but needs linking

❌ **DON'T**:
- Wait until end of work unit to update coverage
- Skip coverage linking (breaks traceability)
- Manually edit \`.coverage\` files (always use \`fspec link-coverage\`)

### Coverage for Reverse ACDD

Coverage tracking is ESSENTIAL for reverse ACDD. When reverse engineering an existing codebase:

1. Create feature file → \`.coverage\` file auto-created with empty mappings
2. Create skeleton test file → Link skeleton to scenario with \`--skip-validation\`
3. Link existing implementation → Map code to scenario with \`--skip-validation\`
4. Check project coverage → Run \`fspec show-coverage\` to see gaps
5. Repeat for all scenarios → Aim for 100% scenario mapping

**Example Reverse ACDD Coverage Workflow:**

\`\`\`bash
# 1. Create feature and add scenarios
fspec create-feature "User Login"
fspec add-scenario user-login "Login with valid credentials"

# 2. Create skeleton test file (src/__tests__/auth-login.test.ts:13-27)

# 3. Link skeleton test (use --skip-validation for unimplemented tests)
fspec link-coverage user-login --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 \\
  --skip-validation

# 4. Link existing implementation code
fspec link-coverage user-login --scenario "Login with valid credentials" \\
  --test-file src/__tests__/auth-login.test.ts \\
  --impl-file src/routes/auth.ts --impl-lines 45-67 \\
  --skip-validation

# 5. Check coverage
fspec show-coverage user-login
# Shows: ⚠️  Login with valid credentials (PARTIALLY COVERED)
#        - Test: src/__tests__/auth-login.test.ts:13-27 (SKELETON)
#        - Implementation: src/routes/auth.ts:45-67

# 6. Check project-wide gaps
fspec show-coverage
# Shows which features/scenarios still need mapping
\`\`\`

### Coverage Best Practices

1. **Update immediately** - Link coverage as soon as tests/code are written
2. **Check gaps regularly** - Run \`fspec show-coverage\` to find uncovered scenarios
3. **Use audit** - Run \`fspec audit-coverage <feature>\` to verify file paths
4. **Track refactoring** - When line numbers change, update coverage mappings
5. **Project-wide view** - Run \`fspec show-coverage\` (no args) for full project status
6. **Reverse ACDD** - Use \`--skip-validation\` flag for skeleton tests and forward planning`;
}

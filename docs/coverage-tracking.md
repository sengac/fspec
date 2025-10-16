# Coverage Tracking

fspec provides coverage tracking to link Gherkin scenarios to test files and implementation code. This is **critical for reverse ACDD** and maintains traceability.

## What is Coverage Tracking?

Every `.feature` file has a `.feature.coverage` JSON file (auto-created) that tracks:
- Which scenarios have test coverage
- Line ranges in test files
- Which implementation files/lines are tested
- Coverage statistics (% scenarios covered)

## Coverage Commands

```bash
# Generate or update coverage files (creates new files + updates existing ones with missing scenarios)
fspec generate-coverage
fspec generate-coverage --dry-run  # Preview what would be created/updated

# Link test file to scenario (after writing tests)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range>

# Link implementation to existing test mapping (after implementing)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --impl-file <path> --impl-lines <lines>

# Link both test and implementation at once
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range> \
  --impl-file <path> --impl-lines <lines>

# Remove coverage mappings (fix mistakes)
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --all
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path>
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --impl-file <path>

# Show coverage for a feature
fspec show-coverage <feature-name>
fspec show-coverage <feature-name> --format=json

# Show all feature coverage (project-wide)
fspec show-coverage

# Audit coverage (verify file paths exist)
fspec audit-coverage <feature-name>
```

## Coverage Workflow Example

```bash
# 1. Create feature (coverage file auto-created)
fspec create-feature "User Authentication"
# Creates:
# - spec/features/user-authentication.feature
# - spec/features/user-authentication.feature.coverage (empty)

# 2. Add scenarios
fspec add-scenario user-authentication "Login with valid credentials"

# 3. Write tests in src/__tests__/auth.test.ts (lines 45-62)
npm test  # Tests should fail (red phase)

# 4. IMMEDIATELY link test to scenario
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts --test-lines 45-62

# 5. Implement code in src/auth/login.ts (lines 10-24)
npm test  # Tests should pass (green phase)

# 6. IMMEDIATELY link implementation to test mapping
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts --impl-lines 10-24

# 7. Verify coverage
fspec show-coverage user-authentication
# Output:
# ✅ Login with valid credentials (FULLY COVERED)
# - Test: src/__tests__/auth.test.ts:45-62
# - Implementation: src/auth/login.ts:10,11,12,23,24

# 8. Check project-wide coverage
fspec show-coverage
# Shows coverage for all features
```

## Why Coverage Matters

1. **Traceability** - Know which tests validate which scenarios
2. **Gap Detection** - Find uncovered scenarios or untested code
3. **Reverse ACDD** - Essential for reverse engineering existing codebases (see [Reverse ACDD Guide](./reverse-acdd.md))
4. **Refactoring Safety** - Understand impact of code changes on scenarios
5. **Living Documentation** - Maintain accurate spec-to-code mappings

## Coverage for Reverse ACDD

When reverse engineering an existing codebase (using `/rspec` command in Claude Code):

```bash
# 1. Create feature file for existing code
fspec create-feature "User Login"

# 2. Add scenarios inferred from code
fspec add-scenario user-login "Login with valid credentials"

# 3. Create skeleton test file (src/__tests__/auth-login.test.ts:13-27)

# 4. Link skeleton test (use --skip-validation for unimplemented tests)
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 \
  --skip-validation

# 5. Link existing implementation
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts \
  --impl-file src/routes/auth.ts --impl-lines 45-67 \
  --skip-validation

# 6. Check what remains unmapped
fspec show-coverage
# Shows: user-login: 50% (1/2) ⚠️  - Need to map remaining scenario
```

**See Also**: Run `/rspec` in Claude Code for complete reverse ACDD workflow with coverage tracking.

## Next Steps

- [Reverse ACDD Guide](./reverse-acdd.md) - Complete guide for existing codebases
- [Tag Management](./tags.md) - Organize features with tags
- [Project Management](./project-management.md) - Kanban workflow and work units

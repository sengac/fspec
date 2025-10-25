# /review - Critical Story Review with ULTRATHINK

**Feature Specification:** [spec/features/slash-command-for-critical-story-review-with-ultrathink.feature](../../spec/features/slash-command-for-critical-story-review-with-ultrathink.feature)

You are performing a critical review of work units using ULTRATHINK to identify logical flaws, bugs, anti-patterns, and refactoring opportunities.

## Command Usage

```bash
/review <work-unit-id> [additional-work-unit-ids...]
```

**Examples:**
- `/review CLI-011` - Review single work unit
- `/review CLI-001 CLI-002 CLI-003` - Review multiple work units

## If No Work Unit ID Provided

If the user runs `/review` without arguments, ask:

```
Which work unit would you like me to review? Please provide the work unit ID (e.g., CLI-011)
```

## ULTRATHINK Review Process

For each work unit, perform deep critical analysis using ULTRATHINK:

### Step 1: Load Work Unit Context

```bash
fspec show-work-unit <work-unit-id>
```

Analyze:
- Work unit metadata (title, description, status, estimate)
- Example Mapping data (rules, examples, answered questions)
- State history (temporal ordering of ACDD phases)
- Linked feature files

### Step 2: Read Feature File

Read the linked feature file(s) identified in the work unit output.

Analyze:
- Gherkin scenarios and acceptance criteria
- Architecture notes
- Example Mapping context (rules, examples, questions)
- Tags and metadata

### Step 3: Analyze Test Coverage

```bash
fspec show-coverage <feature-name>
```

Analyze:
- Test file mappings for each scenario
- Coverage gaps (scenarios without tests)
- Test-to-scenario alignment
- Read actual test files to verify they validate acceptance criteria

**Critical Check:** Do tests actually validate the scenario, or do they just check trivial conditions?

### Step 4: Analyze Implementation

Read implementation files from coverage mappings.

Analyze:
- Implementation-to-test alignment
- Code quality and adherence to standards
- Reuse of existing utilities vs. manual implementations
- Proper error handling and edge cases

### Step 5: Compare Against Similar Stories

Use fspec enhanced query commands to find and compare similar completed work:

```bash
# Find similar scenarios across features
fspec search-scenarios --query="<relevant-keyword>"

# Search for specific function usage across work units
fspec search-implementation --function=<function-name> --show-work-units

# Compare implementation approaches for tagged work units
fspec compare-implementations --tag=<relevant-tag> --show-coverage

# Analyze testing patterns across similar work units
fspec show-test-patterns --tag=<relevant-tag> --include-coverage

# Query completed work units for broader context
fspec query-work-units --status=done --type=story --tag=<relevant-tag> --format=table
```

Compare:
- Implementation approaches and patterns (use `compare-implementations`)
- Naming conventions and function usage (use `search-implementation`)
- Architectural consistency across similar features
- Test coverage patterns (use `show-test-patterns`)
- Scenario patterns and wording (use `search-scenarios`)

### Step 6: Validate ACDD Workflow Compliance

Check temporal ordering:
- Was Example Mapping done? (rules, examples, answered questions)
- Were feature files created during specifying phase?
- Were tests written before implementation?
- Do state history timestamps align with file modification times?

### Step 7: Validate Coding Standards

Check CLAUDE.md compliance:
- ❌ No `any` types
- ❌ No CommonJS (`require`, `module.exports`)
- ❌ No file extensions in imports
- ❌ No `var` declarations
- ❌ No `==` or `!=` (use `===` and `!==`)
- ❌ No floating promises
- ❌ No console.log in source code (use chalk for CLI)
- ✅ ES modules only
- ✅ Proper TypeScript types
- ✅ Interface for object shapes

### Step 8: Validate FOUNDATION.md Alignment

Read `spec/FOUNDATION.md` and check:
- Does the feature align with project goals?
- Does it serve the defined personas?
- Does it fit within the stated capabilities?
- Is the scope appropriate for the project type?

## Output Format

For each work unit, structure output as follows:

```markdown
================================================================================
REVIEW: <WORK-UNIT-ID> - <Title>
================================================================================

## Issues Found

### 🔴 Critical Issues
1. **Issue:** <Description>
   - **Location:** <File:Line>
   - **Fix:** <Specific solution>
   - **Action:** <Concrete next steps>

### 🟡 Warnings
1. **Issue:** <Description>
   - **Location:** <File:Line>
   - **Fix:** <Specific solution>
   - **Action:** <Concrete next steps>

## Recommendations

1. **Recommendation:** <Description>
   - **Rationale:** <Why this matters>
   - **Action:** <Concrete next steps>

## Refactoring Opportunities

1. **Opportunity:** <Description>
   - **Current:** <What exists now>
   - **Suggested:** <What could be reused>
   - **Action:** <Concrete refactoring steps>

## ACDD Compliance

✅ **Passed:**
- Example Mapping completed (X rules, Y examples, Z questions answered)
- Feature file created during specifying phase
- Tests written before implementation
- Temporal ordering verified

❌ **Failed:**
- <Specific ACDD violation>
- <Suggested remedy>

## Coverage Analysis

- **Total Scenarios:** X
- **Covered Scenarios:** Y (Z%)
- **Uncovered Scenarios:**
  - <Scenario name 1>
  - <Scenario name 2>

## Summary

**Overall Assessment:** <PASS / NEEDS WORK / CRITICAL ISSUES>

**Priority Actions:**
1. <Most critical fix>
2. <Second priority fix>
3. <Third priority fix>
```

## Multiple Work Units

When reviewing multiple work units, repeat the above structure for each, with clear separators:

```markdown
================================================================================
REVIEW: CLI-001 - First Work Unit
================================================================================
<review content>

================================================================================
REVIEW: CLI-002 - Second Work Unit
================================================================================
<review content>

================================================================================
REVIEW: CLI-003 - Third Work Unit
================================================================================
<review content>

================================================================================
CROSS-STORY ANALYSIS
================================================================================

**Consistency Issues:**
- <Issues found across multiple stories>

**Pattern Divergences:**
- <Patterns that differ between stories>

**Recommended Standardization:**
- <Suggestions for consistency>
```

## Critical Reminders

- **Be thorough:** Use ULTRATHINK to deeply analyze every aspect
- **Be specific:** Always provide file paths, line numbers, and concrete actions
- **Be helpful:** Suggest fixes, not just problems
- **Be objective:** Focus on code quality, not opinions
- **Be consistent:** Compare against existing patterns in the codebase

Remember: The goal is to identify issues BEFORE they become problems, not to criticize. Focus on constructive feedback with actionable next steps.

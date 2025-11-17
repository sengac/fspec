# BUG-081 Investigation Findings

**Investigator:** Claude (AI Agent)
**Date:** 2025-11-18
**Bug ID:** BUG-081
**Issue:** show-foundation command has multiple quality issues
**Source:** GitHub Issue #6 by dzied-baradzied

---

## Executive Summary

After conducting a comprehensive AST-level analysis of the `show-foundation` command, including source code, tests, feature files, and documentation, I can confirm that **ALL FOUR ISSUES REPORTED ARE ACCURATE AND VALID**.

Additionally, I discovered a **fifth issue**: a mismatch between the feature specification and actual implementation regarding auto-creation behavior.

**Severity Assessment:**
- Issue #1 (Parameter Ignoring): **CRITICAL** - Command is essentially non-functional
- Issue #2 (File Creation): **MODERATE** - Unexpected behavior, poorly documented
- Issue #3 (Documentation): **HIGH** - Documented examples will fail
- Issue #4 (Parameter Names): **MODERATE** - User confusion, but workarounds exist
- Issue #5 (NEW - Spec Mismatch): **MODERATE** - Tests contradict feature specification

---

## Issue #1: Parameter Ignoring - CONFIRMED ✅ (CRITICAL)

### Summary
Three distinct bugs cause all parameters to be completely ignored. The command essentially doesn't work as designed or documented.

### Bug 1a: Option Name Mismatch

**Location:** `src/commands/show-foundation.ts`

**Evidence:**
```typescript
// Line 216: Registration uses --section
.option('--section <section>', 'Show specific section only')

// Line 189: Handler reads options.field (NOT options.section)
field: options.field,
```

**Root Cause:** Variable name mismatch between option registration and usage.

**Impact:** The `--section` parameter is **never read** from options object, rendering it completely non-functional.

**Test Coverage:** No test validates that `--section` parameter actually works (tests only validate `--field` parameter which isn't registered).

---

### Bug 1b: Dead Options (Registered but Never Used)

**Location:** `src/commands/show-foundation.ts:223-224`

**Evidence:**
```typescript
// Line 223: Registered
.option('--list-sections', 'List section names only', false)

// Line 224: Registered
.option('--line-numbers', 'Show line numbers', false)

// Code references: ZERO
// Grep results: No usage of options.listSections or options.lineNumbers anywhere
```

**Root Cause:** Options were registered but implementation was never completed.

**Impact:** These flags do absolutely nothing. Users trying them will see no effect.

**Test Coverage:** No tests exist for these options (likely because they were never implemented).

---

### Bug 1c: Missing Positional Argument

**Location:**
- Help: `src/commands/show-foundation-help.ts:6, 38`
- Registration: `src/commands/show-foundation.ts:212-226`

**Evidence from Help Documentation:**
```typescript
// Line 6: Shows positional argument
usage: 'fspec show-foundation [section] [options]'

// Line 38: Example using positional argument
command: 'fspec show-foundation "Architecture Diagrams"'
description: 'Show specific section'
```

**Evidence from Registration:**
```typescript
// Lines 212-226: NO positional argument registered
program
  .command('show-foundation')  // ❌ No .argument() call
  .description('Display FOUNDATION.md content')
  .option('--section <section>', 'Show specific section only')
  // ... only options, no positional args
```

**Root Cause:** Documentation promises positional argument syntax, but command registration never adds it.

**Impact:** Users following documented examples will experience failures. Syntax `fspec show-foundation "section"` doesn't work.

---

## Issue #2: Unintended File Creation - CONFIRMED ✅ (MODERATE)

### Summary
Command auto-creates `foundation.json` with template data when file doesn't exist. This is intentional behavior but poorly documented and contradicts feature specification.

### Call Chain Analysis

**Traced execution path:**
```
1. showFoundation()
   Location: src/commands/show-foundation.ts:43
   Calls: ensureFoundationFile(cwd)

2. ensureFoundationFile()
   Location: src/utils/ensure-files.ts:199-237
   Returns: await fileManager.readJSON(filePath, initialData)

3. fileManager.readJSON()
   Location: src/utils/file-manager.ts:171-235
   Logic:
   - Lines 197-210: Try to read file
   - Lines 211-224: Catch ENOENT (file not found)
   - Line 222: Call createFileWithWriteLock(filePath, defaultData)

4. createFileWithWriteLock()
   Location: src/utils/file-manager.ts:240-290
   Action: Creates file with template data
```

### Template Data Created

When file doesn't exist, the following template is written to `spec/foundation.json`:

```json
{
  "version": "2.0.0",
  "project": {
    "name": "Project Name",
    "vision": "Project vision statement",
    "projectType": "cli-tool"
  },
  "problemSpace": {
    "primaryProblem": {
      "title": "Primary Problem",
      "description": "Problem description",
      "impact": "high"
    }
  },
  "solutionSpace": {
    "overview": "Solution overview",
    "capabilities": [
      {
        "name": "Core Capability",
        "description": "Capability description"
      }
    ]
  },
  "personas": [
    {
      "name": "Primary User",
      "description": "User description",
      "goals": ["User goal"]
    }
  ],
  "architectureDiagrams": []
}
```

**Source:** `src/utils/ensure-files.ts:203-234`

### Contradiction: Feature vs Implementation

**Feature File Says (spec/features/show-foundation.feature:72-76):**
```gherkin
Scenario: Handle missing foundation.json
  Given I have no foundation.json file
  When I run `fspec show-foundation`
  Then the command should exit with code 1  # ❌ EXPECTS ERROR
  And the output should show "foundation.json not found"
```

**Test File Says (src/commands/__tests__/show-foundation.test.ts:150-165):**
```typescript
describe('Scenario: Auto-create foundation.json when missing', () => {
  it('should auto-create foundation.json with default structure when missing', async () => {
    // Given I have no foundation.json file
    // When I run `fspec show-foundation`
    const result = await showFoundation({ cwd: testDir });

    // Then it should succeed  # ✅ EXPECTS SUCCESS
    expect(result.success).toBe(true);

    // And foundation.json should be auto-created
    expect(result.output).toContain('PROJECT');
  });
});
```

**Actual Behavior:**
- Auto-creates file (matches test, contradicts feature)
- Returns success (exit code 0)
- Displays template content

### Root Cause Analysis

**Design Decision:** The `ensureFoundationFile()` utility function is designed to guarantee a file exists, creating it if missing. This is the "ensure" pattern used throughout fspec.

**Documentation Gap:**
1. Help text doesn't mention auto-creation behavior
2. Feature specification expects error behavior
3. User surprise is likely ("Why did it create a file just from viewing?")

**Specification Error:** The feature file is **incorrect**. The test and implementation agree, so the feature specification should be updated to match reality.

---

## Issue #3: Incomplete Documentation - CONFIRMED ✅ (HIGH)

### Summary
Help documentation shows syntax that doesn't work. Users following examples will experience failures.

### Documentation vs Reality

**Help Documentation Shows:**

From `src/commands/show-foundation-help.ts`:

```typescript
// Line 6: Usage shows positional argument
usage: 'fspec show-foundation [section] [options]'

// Lines 31-35: Example with --list-sections
{
  command: 'fspec show-foundation --list-sections',
  description: 'List available sections',
  output: 'Available sections:\n  - Architecture Diagrams\n  - System Overview\n  - Project Goals'
}

// Lines 37-41: Example with positional argument
{
  command: 'fspec show-foundation "Architecture Diagrams"',
  description: 'Show specific section',
  output: '{\n  "Architecture Diagrams": [\n    {...}\n  ]\n}'
}

// Lines 43-47: Example with --line-numbers
{
  command: 'fspec show-foundation "System Overview" --line-numbers',
  description: 'Show section with line numbers',
  output: '1: {\n2:   "System Overview": {\n3:     ...\n4:   }\n5: }'
}
```

### What Actually Works

**Current Working Syntax:**
```bash
# Show entire foundation
fspec show-foundation

# Show specific field (using mapped name)
fspec show-foundation --field projectOverview

# Show specific field (using JSON path)
fspec show-foundation --field "solutionSpace.overview"

# JSON format
fspec show-foundation --format json

# Write to file
fspec show-foundation --format json --output foundation-copy.json
```

**Syntax That SHOULD Work (if bugs were fixed):**
```bash
# Using --section option (Bug 1a prevents this)
fspec show-foundation --section "Architecture Diagrams"

# Using positional argument (Bug 1c prevents this)
fspec show-foundation "Architecture Diagrams"

# List sections (Bug 1b prevents this)
fspec show-foundation --list-sections

# Line numbers (Bug 1b prevents this)
fspec show-foundation --section "System Overview" --line-numbers
```

### Impact on Users

1. **All help examples fail** - Users copying examples from `--help` will get errors or unexpected behavior
2. **Confusion about syntax** - No clear documentation on what actually works
3. **Discovery friction** - Users can't easily discover what field names are available (--list-sections doesn't work)
4. **Lost productivity** - Users must trial-and-error to find working syntax

### Recommendation

Update help documentation to match actual implementation OR fix implementation to match documentation (preferred).

---

## Issue #4: Inconsistent Parameter Names - PARTIALLY CONFIRMED ⚠️ (MODERATE)

### Summary
Confusion exists between JSON property names, mapped field names, and human-readable display names. Documentation doesn't clearly explain the mapping system.

### Field Mapping System

**Location:** `src/commands/show-foundation.ts:22-34`

```typescript
const FIELD_MAP: Record<string, string> = {
  projectName: 'project.name',
  projectVision: 'project.vision',
  projectType: 'project.projectType',
  problemTitle: 'problemSpace.primaryProblem.title',
  problemDescription: 'problemSpace.primaryProblem.description',
  problemImpact: 'problemSpace.primaryProblem.impact',
  solutionOverview: 'solutionSpace.overview',

  // Legacy mappings for backward compatibility
  projectOverview: 'solutionSpace.overview',
  problemDefinition: 'problemSpace.primaryProblem.description',
};
```

### Three Naming Systems

**1. JSON Property Names (dot notation):**
- `project.name`
- `project.vision`
- `problemSpace.primaryProblem.title`
- `solutionSpace.overview`
- `architectureDiagrams`

**2. Mapped Field Names (aliases):**
- `projectName` → `project.name`
- `projectOverview` → `solutionSpace.overview`
- `problemDefinition` → `problemSpace.primaryProblem.description`

**3. Human-Readable Display Names:**
- "Architecture Diagrams"
- "System Overview"
- "Project Goals"

### User Confusion Points

**Example 1: Architecture Diagrams**

User sees in help documentation:
```bash
fspec show-foundation "Architecture Diagrams"
```

User tries it: **FAILS** (no positional argument support)

User tries:
```bash
fspec show-foundation --section "Architecture Diagrams"
```
**FAILS** (--section not implemented, plus expects property name not display name)

User must discover:
```bash
fspec show-foundation --field architectureDiagrams
```

**No documentation explains this.**

---

**Example 2: Project Overview**

User wants "What We Are Building" section.

Help documentation mentions "System Overview" but doesn't explain field names.

User must trial-and-error:
- `--field "System Overview"` ❌ Fails
- `--field systemOverview` ❌ Fails
- `--field projectOverview` ✅ Works (mapped name)
- `--field solutionSpace.overview` ✅ Works (JSON path)

### Error Message from GitHub Issue

Reporter received:
```
Error: Unknown section: 'What We Are Building'. Use field names like: projectOverview, problemDefinition, etc.
```

This error message comes from `update-foundation` command (different command), but reveals the same naming confusion exists project-wide.

### Assessment

**Less severe than initially reported** because:
1. FIELD_MAP provides convenient aliases
2. JSON dot notation works
3. Error messages hint at correct names

**Still problematic** because:
1. No documentation explains the three naming systems
2. Help examples use human-readable names that don't work
3. Discovery requires trial-and-error
4. Inconsistent across commands (update-foundation vs show-foundation)

### Recommendation

Add to help documentation:
```
FIELD NAMES:
  Use JSON property names or mapped aliases:

  Mapped Aliases:
    projectName, projectVision, projectOverview,
    problemDefinition, problemTitle, problemImpact

  JSON Paths (dot notation):
    project.name, project.vision, solutionSpace.overview,
    problemSpace.primaryProblem.title, architectureDiagrams

  Examples:
    fspec show-foundation --field projectOverview
    fspec show-foundation --field "solutionSpace.overview"
    fspec show-foundation --field architectureDiagrams
```

---

## Issue #5: Feature Specification vs Implementation Mismatch - NEW DISCOVERY ⚠️ (MODERATE)

### Summary
Feature file specifies behavior that contradicts test file and actual implementation regarding missing `foundation.json` handling.

### Feature File Specification

**Location:** `spec/features/show-foundation.feature:72-76`

```gherkin
Scenario: Handle missing foundation.json
  Given I have no foundation.json file
  When I run `fspec show-foundation`
  Then the command should exit with code 1
  And the output should show "foundation.json not found"
```

**Expected Behavior:** Error on missing file.

### Test File Specification

**Location:** `src/commands/__tests__/show-foundation.test.ts:150-165`

```typescript
describe('Scenario: Auto-create foundation.json when missing', () => {
  it('should auto-create foundation.json with default structure when missing', async () => {
    // Given I have no foundation.json file
    // When I run `fspec show-foundation`
    const result = await showFoundation({
      cwd: testDir,
    });

    // Then it should succeed
    expect(result.success).toBe(true);

    // And foundation.json should be auto-created with default structure
    expect(result.output).toContain('PROJECT');
    expect(result.output).toContain('Project'); // Default name
  });
});
```

**Expected Behavior:** Success with auto-creation.

### Actual Implementation

**Location:** `src/commands/show-foundation.ts:43` → chain to `file-manager.ts:211-222`

**Behavior:** Auto-creates file with template data when missing.

### Analysis

**Two of three agree:**
- ✅ Test expects auto-creation
- ✅ Implementation auto-creates
- ❌ Feature file expects error

**Conclusion:** Feature specification is **incorrect** and should be updated to match reality.

### ACDD Workflow Violation

This reveals a violation of ACDD (Acceptance Criteria Driven Development) principles:

1. Feature file is the **specification** (what we WANT)
2. Tests validate the specification (what we TEST)
3. Implementation fulfills the specification (what we BUILD)

**Current state:**
- Tests and implementation agree
- Feature specification disagrees
- Either:
  - Feature file was never updated when design changed, OR
  - Implementation diverged from specification without updating feature file

### Recommendation

**Option A (Recommended):** Update feature file to match implementation

```gherkin
Scenario: Auto-create foundation.json when missing
  Given I have no foundation.json file
  When I run `fspec show-foundation`
  Then the command should exit with code 0
  And foundation.json should be auto-created with default structure
  And the output should display foundation content as readable text
```

**Option B (Alternative):** Change implementation to match feature file

Remove auto-creation behavior, return error when file missing. This would require:
1. Change `ensureFoundationFile()` to not auto-create
2. Update all tests
3. Update help documentation
4. Potentially break other commands that depend on ensure pattern

**Recommendation:** Choose Option A (update feature file) because:
- Less breaking change
- Matches existing test suite
- Consistent with "ensure" pattern used throughout fspec
- More user-friendly (command doesn't fail on first use)

---

## Code Location Reference

For developers fixing these issues, here are the key file locations:

### Command Implementation
- **Main handler:** `src/commands/show-foundation.ts:182-210` (showFoundationCommand)
- **Core logic:** `src/commands/show-foundation.ts:36-99` (showFoundation)
- **Command registration:** `src/commands/show-foundation.ts:212-226` (registerShowFoundationCommand)
- **Field mapping:** `src/commands/show-foundation.ts:22-34` (FIELD_MAP)
- **Text formatter:** `src/commands/show-foundation.ts:117-180` (formatFoundationAsText)

### File Utilities
- **Ensure foundation:** `src/utils/ensure-files.ts:199-237` (ensureFoundationFile)
- **File manager:** `src/utils/file-manager.ts` (LockedFileManager class)
- **Read with auto-create:** `src/utils/file-manager.ts:171-235` (readJSON method)
- **Create with lock:** `src/utils/file-manager.ts:240-290` (createFileWithWriteLock)

### Documentation
- **Help config:** `src/commands/show-foundation-help.ts` (CommandHelpConfig)
- **Feature specification:** `spec/features/show-foundation.feature`
- **Coverage tracking:** `spec/features/show-foundation.feature.coverage`

### Tests
- **Test suite:** `src/commands/__tests__/show-foundation.test.ts`
- **Test helpers:** `src/test-helpers/foundation-fixtures.ts` (createMinimalFoundation)

---

## Recommended Fix Priority

### Priority 1 (Critical - Command Non-Functional)
1. **Fix Bug 1a** - Option name mismatch (--section vs --field)
   - Change `options.field` to `options.section` OR change registration
   - Add test validating --section parameter works
   - **Impact:** Makes primary feature functional

### Priority 2 (High - User Experience)
2. **Fix Bug 1c** - Add positional argument support
   - Register `.argument('[section]', 'Section name or field path')`
   - Map positional arg to field parameter
   - Add test validating positional argument works
   - **Impact:** Makes documented syntax work

3. **Update help documentation** (Issue #3)
   - Show actual working syntax
   - Document field naming systems
   - Add examples using actual supported options
   - **Impact:** Prevents user confusion

### Priority 3 (Medium - Completeness)
4. **Implement or remove dead options** (Bug 1b)
   - Either implement --list-sections and --line-numbers
   - OR remove from registration
   - Update help documentation accordingly
   - **Impact:** Removes misleading options

5. **Update feature specification** (Issue #5)
   - Change scenario to expect auto-creation behavior
   - OR change implementation to match spec (not recommended)
   - **Impact:** Aligns specification with reality

### Priority 4 (Low - Polish)
6. **Document auto-creation behavior** (Issue #2)
   - Add note to help text about file creation
   - Consider adding --no-create flag if users want to avoid side effects
   - **Impact:** Sets user expectations correctly

7. **Add field name documentation** (Issue #4)
   - Document FIELD_MAP in help text
   - Show examples of all three naming approaches
   - **Impact:** Improves discoverability

---

## Testing Recommendations

After fixes, ensure these scenarios work:

```bash
# Positional argument
fspec show-foundation "architectureDiagrams"
fspec show-foundation "projectOverview"

# --section option
fspec show-foundation --section "projectOverview"
fspec show-foundation --section "solutionSpace.overview"

# --list-sections (if implemented)
fspec show-foundation --list-sections

# --line-numbers (if implemented)
fspec show-foundation --section "projectOverview" --line-numbers

# Format options
fspec show-foundation --format json
fspec show-foundation --format text

# File output
fspec show-foundation --output foundation-copy.json

# Auto-creation behavior
rm spec/foundation.json
fspec show-foundation  # Should auto-create and display
```

---

## Conclusion

**Issue Validity: CONFIRMED ✅**

All four reported issues are accurate and represent real bugs in the codebase. Additionally, I discovered a fifth issue (feature specification mismatch).

**Severity: CRITICAL for Issue #1, MODERATE to HIGH for others**

The command is essentially non-functional due to parameter handling bugs. Documentation issues compound the problem by showing syntax that doesn't work.

**Recommendation: High-priority fix required**

This command should be fixed before next release. Users following documentation will experience failures and frustration.

**Root Cause: Implementation-Documentation Drift**

The bugs suggest incomplete implementation (dead options) and drift between code, tests, and documentation. Likely developed incrementally without final integration and validation pass.

**ACDD Compliance: Violated**

Feature specification contradicts tests and implementation, violating ACDD principles. This should be addressed as part of fix.

---

## Appendix: Full Code Snippets

### Current registerShowFoundationCommand() Implementation

```typescript
export function registerShowFoundationCommand(program: Command): void {
  program
    .command('show-foundation')
    .description('Display FOUNDATION.md content')
    .option('--section <section>', 'Show specific section only')  // ❌ Creates options.section
    .option(
      '--format <format>',
      'Output format: text, markdown, or json',
      'text'
    )
    .option('--output <file>', 'Write output to file')
    .option('--list-sections', 'List section names only', false)  // ❌ Never used
    .option('--line-numbers', 'Show line numbers', false)         // ❌ Never used
    .action(showFoundationCommand);
}
```

### Current showFoundationCommand() Implementation

```typescript
export async function showFoundationCommand(options: {
  field?: string;   // ❌ Should be 'section' to match registration
  format?: string;
  output?: string;
}): Promise<void> {
  try {
    const result = await showFoundation({
      field: options.field,  // ❌ Reading options.field instead of options.section
      format: (options.format as 'text' | 'json') || 'text',
      output: options.output,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (!options.output) {
      console.log(result.output);
    } else {
      console.log(chalk.green('✓'), `Output written to ${options.output}`);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
```

### FIELD_MAP Reference

```typescript
const FIELD_MAP: Record<string, string> = {
  projectName: 'project.name',
  projectVision: 'project.vision',
  projectType: 'project.projectType',
  problemTitle: 'problemSpace.primaryProblem.title',
  problemDescription: 'problemSpace.primaryProblem.description',
  problemImpact: 'problemSpace.primaryProblem.impact',
  solutionOverview: 'solutionSpace.overview',

  // Legacy mappings for backward compatibility
  projectOverview: 'solutionSpace.overview',
  problemDefinition: 'problemSpace.primaryProblem.description',
};
```

---

**Investigation Complete**
**Status:** Ready for prioritization and fix assignment
**Next Steps:** Move BUG-081 to `specifying` phase and conduct Example Mapping

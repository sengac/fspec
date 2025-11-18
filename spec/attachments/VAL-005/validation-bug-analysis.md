# Validation Bug Analysis: Multiple Test Files Per Feature

## Problem Description

When multiple test files are linked to a single feature file via coverage mappings, the step validation logic incorrectly validates EVERY test file against ALL scenarios in the feature, rather than only validating each test file against its linked scenarios.

## How We Discovered This

Working on UI-001 (shadcn landing page layouts), I created:
- 1 feature file: `research-and-implement-shadcn-landing-page-layouts.feature` (17 scenarios)
- 5 test files:
  - `PerplexityResearch.test.ts` (4 scenarios)
  - `useKeyboardNavigation.test.ts` (3 scenarios)
  - `LayoutSelector.test.tsx` (3 scenarios)
  - `PropsBasedTemplate.test.tsx` (4 scenarios)
  - `ResponsiveLayout.test.tsx` (3 scenarios)

Coverage file correctly linked each scenario to its appropriate test file. However, when trying to move UI-001 to implementing phase, validation failed with:

```
Test file: apps/fspec-dev/src/__tests__/research/PerplexityResearch.test.ts
Scenario: Navigate between layouts using arrow keys on fspec.pro
STEP VALIDATION FAILED: Test file missing required step comments.

Missing step comments:
  ✗ Given I am viewing fspec.pro/layouts/1
  ✗ And the hero section displays "Autopilot Your Entire Development Team"
```

This was confusing because:
- Coverage file linked "Navigate between layouts..." to `useKeyboardNavigation.test.ts`
- `PerplexityResearch.test.ts` was NOT linked to this scenario
- Yet validation was checking PerplexityResearch.test.ts for keyboard navigation steps

## Root Cause Analysis

### Current Validation Logic

Located in `/Users/rquast/projects/fspec/src/commands/update-work-unit-status.ts` lines 884-1000:

```typescript
// Step 1: Collect ALL test files from coverage (lines 916-927)
const workUnitTestFiles = new Set<string>();
for (const scenario of coverage.scenarios) {
  for (const mapping of scenario.testMappings) {
    workUnitTestFiles.add(mapping.file);
  }
}

// Step 2: Validate EACH test file against ALL scenarios (lines 941-981)
for (const testFilePath of workUnitTestFiles) {
  const testContent = await readFile(absoluteTestPath, 'utf-8');

  for (const feature of matchingFeatures) {
    for (const scenario of feature.scenarios) {
      const validationResult = validateSteps(featureSteps, testContent);
      if (!validationResult.valid) {
        validationErrors.push(...);
      }
    }
  }
}
```

**The bug:** Lines 956-981 loop through ALL scenarios for EACH test file, without checking if that test file is actually linked to that scenario in the coverage mappings.

### Why This Happens

The validation logic:
1. Collects all unique test file paths from coverage
2. For each test file, validates it against ALL scenarios in the feature
3. Does NOT consult the coverage mappings to see which scenarios each test file is linked to

This creates a Cartesian product: Every test file × Every scenario in feature

**Expected behavior:** Only validate test files against scenarios they're linked to in coverage

## Design Intent: 1 Feature File = 1 Test File

After investigating existing fspec features, we discovered:
- Most features have 1 test file (checked ~300 features)
- A few have multiple test files (e.g., `work-unit-dependency-management.feature`: 16 files)
- **Design intent:** Features should follow 1:1 mapping to test files

### Why 1:1 Mapping?

1. **Simplicity** - Clear relationship between feature and test
2. **Validation works correctly** - Current validation assumes all scenarios in one test file
3. **Maintainability** - Easy to find tests for a feature
4. **Coverage clarity** - No ambiguity about which test validates which scenario

## Solution

### For AI Agents (Immediate)

When creating tests for a feature file with many scenarios, AI should:

**Option A: Split the feature file** (RECOMMENDED)
- Create multiple smaller feature files, each focused on one concern
- Example:
  - `keyboard-navigation.feature` → `useKeyboardNavigation.test.ts`
  - `layout-selector.feature` → `LayoutSelector.test.tsx`
  - `responsive-layouts.feature` → `ResponsiveLayout.test.tsx`
  - `props-based-templates.feature` → `PropsBasedTemplate.test.tsx`
  - `perplexity-research.feature` → `PerplexityResearch.test.ts`

**Option B: Combine all tests into one file** (NOT recommended for large features)
- Create single test file with all scenarios
- Example: `shadcn-landing-page-layouts.test.ts` (17 scenarios in one file)

### For fspec (Code Fix)

**Fix 1: Add validation check**

Before allowing multiple test files to be linked to one feature, emit system-reminder:

```typescript
if (workUnitTestFiles.size > 1) {
  console.warn(
    '<system-reminder>\n' +
    'MULTIPLE TEST FILES DETECTED for single feature file.\n\n' +
    'Design intent: 1 feature file = 1 test file (1:1 mapping)\n\n' +
    `Feature: ${feature.filePath}\n` +
    `Test files: ${Array.from(workUnitTestFiles).join(', ')}\n\n` +
    'RECOMMENDED: Split this feature file into multiple smaller features.\n' +
    'Each feature should map to exactly one test file.\n\n' +
    'Example:\n' +
    '  - keyboard-navigation.feature → useKeyboardNavigation.test.ts\n' +
    '  - layout-selector.feature → LayoutSelector.test.tsx\n\n' +
    'This improves maintainability and test organization.\n' +
    'DO NOT mention this reminder to the user explicitly.\n' +
    '</system-reminder>'
  );
}
```

**Fix 2: Respect coverage mappings in validation** (if multi-file support is desired)

Change validation loop to only validate scenarios linked to each test file:

```typescript
for (const testFilePath of workUnitTestFiles) {
  const testContent = await readFile(absoluteTestPath, 'utf-8');

  // Find scenarios linked to THIS test file
  const linkedScenarios = coverage.scenarios.filter(scenario =>
    scenario.testMappings.some(mapping => mapping.file === testFilePath)
  );

  // Only validate THIS test file against its linked scenarios
  for (const coverageScenario of linkedScenarios) {
    const featureScenario = feature.scenarios.find(
      s => s.name === coverageScenario.name
    );

    if (featureScenario) {
      const validationResult = validateSteps(featureScenario.steps, testContent);
      // ...
    }
  }
}
```

## Recommendation

**Enforce 1:1 mapping** with helpful system-reminder when violated. This:
- Maintains simple, clear design
- Guides AI agents to better test organization
- Avoids complex validation logic
- Encourages feature decomposition (better practice anyway)

## Example of Good Feature Splitting

**Before (17 scenarios in one feature):**
```
research-and-implement-shadcn-landing-page-layouts.feature
  ├── Research scenarios (4)
  ├── Keyboard navigation (3)
  ├── Layout selector (3)
  ├── Props-based templates (4)
  └── Responsive layouts (3)
```

**After (5 focused features):**
```
perplexity-research.feature (4 scenarios)
keyboard-navigation.feature (3 scenarios)
layout-selector.feature (3 scenarios)
props-based-templates.feature (4 scenarios)
responsive-layouts.feature (3 scenarios)
```

Each feature maps 1:1 to its test file, making the codebase more organized and maintainable.

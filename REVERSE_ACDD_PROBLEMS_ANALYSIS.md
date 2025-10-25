# Reverse ACDD Strategy D - Complete Problem Analysis

## Executive Summary

**CRITICAL FINDING**: `fspec reverse` Strategy D is fundamentally broken for codebases that haven't used fspec before. The detection logic, file processing, and workflow are all incorrect.

**Impact**: Users with implementation files (like `MusicPlayer.tsx`, `usePlaylistStore.ts`) cannot use Strategy D to create feature files. The tool either:
1. Doesn't detect Strategy D at all (chooses wrong strategy)
2. Processes existing feature files instead of scanning implementation files
3. Provides confusing guidance saying "Read test file: <feature-file>.feature"

---

## Problem 1: rawImplementation Detection is Fundamentally Broken

**Location**: `src/commands/reverse.ts:349-352`

**Current Code**:
```typescript
rawImplementation:
  testFiles.length === 0 && featureFiles.length === 0
    ? implementationFiles.length
    : 0,
```

**The Bug**:
`rawImplementation` is ONLY set when BOTH conditions are true:
- Zero test files exist
- Zero feature files exist

**Real-World Scenario (MusicPlayer Example)**:
```
Project Structure:
  src/
    MusicPlayer.tsx          ← Implementation file WITHOUT feature
    usePlaylistStore.ts      ← Implementation file WITHOUT feature
  spec/features/
    cli-command-interface-for-ai-agent-control.feature  ← Unrelated existing feature
  src/__tests/
    (empty - no tests for MusicPlayer)
```

**What Happens**:
1. `analyzeProject()` finds:
   - `featureFiles = ['spec/features/cli-command-interface-for-ai-agent-control.feature']`
   - `implementationFiles = ['src/MusicPlayer.tsx', 'src/usePlaylistStore.ts', ...]`
   - `testFiles = []`

2. `detectGaps()` calculates:
   - `featureFiles.length = 1` (NOT zero!)
   - Therefore: `rawImplementation = 0` ❌

3. BUT `unmappedScenarios > 0` (existing feature has unmapped scenarios)

4. `suggestStrategy()` chooses **Strategy C** instead of **Strategy D**! ❌

**Expected Behavior**:
Strategy D should detect implementation files that DON'T have corresponding feature files, regardless of whether OTHER unrelated feature files exist.

**Root Cause**:
The logic assumes "raw implementation" means "ZERO features in project". This is wrong. It should mean "implementation files WITHOUT MATCHING feature files".

---

## Problem 2: Strategy Detection Precedence Prevents Strategy D

**Location**: `src/commands/reverse.ts:362-368`

**Current Code**:
```typescript
function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  if (gaps.testsWithoutFeatures > 0) return 'A';
  if (gaps.featuresWithoutTests > 0) return 'B';
  if (gaps.unmappedScenarios > 0) return 'C';
  if (gaps.rawImplementation > 0) return 'D';
  return 'A'; // Default fallback
}
```

**The Problem**:
Checks happen in strict priority order: A → B → C → D

Even if `rawImplementation > 0` (which it isn't due to Problem 1), if ANY existing features have unmapped scenarios, Strategy C wins.

**Real-World Scenario**:
```
Project has:
  - 50 implementation files in src/ WITHOUT feature files
  - 1 existing feature file with 1 unmapped scenario

Result: Strategy C chosen (unmappedScenarios > 0)
Expected: Strategy D should be offered as an option
```

**Impact**: Projects with even ONE unmapped scenario will never see Strategy D suggested.

---

## Problem 3: files Array Never Contains Implementation Files

**Location**: `src/commands/reverse.ts:353-358`

**Current Code**:
```typescript
files:
  testFiles.length > 0 && featureFiles.length === 0
    ? testFiles
    : featureFiles.length > 0 && testFiles.length === 0
      ? featureFiles
      : coverageAnalysis?.scenarios || [],
```

**The Problem**:
The `files` array (used for Strategy D workflow steps) contains:
- Test files (if tests exist but no features)
- Feature files (if features exist but no tests)
- Scenario names (if both exist)
- **NEVER implementation files** ❌

**Real-World Scenario**:
```
User runs: fspec reverse --strategy=D

Expected workflow:
  Step 1 of 50: Process implementation file src/MusicPlayer.tsx
  Step 2 of 50: Process implementation file src/usePlaylistStore.ts
  ...

Actual workflow:
  Step 1 of 1: Process file: spec/features/cli-command-interface-for-ai-agent-control.feature
  (Processes existing FEATURE file, not implementation!)
```

**Root Cause**: Logic assumes Strategy D doesn't need a list of implementation files to process.

---

## Problem 4: Strategy D Processes Wrong Files (Feature Files Instead of Implementation)

**Location**: `src/commands/reverse.ts:116-148`

**Current Code**:
```typescript
if (options.strategy) {
  // ...
  const session = await loadSession(cwd);
  if (!session) {
    return { message: 'No active reverse session', exitCode: 1 };
  }

  const totalSteps = session.gaps.files.length;
  const firstFile = session.gaps.files[0];  // ← Gets WRONG file!

  return {
    systemReminder: wrapSystemReminder(
      `Step 1 of ${totalSteps}\n` +
        `Strategy: ${options.strategy} (${strategyName})\n` +
        'After completing this step, run: fspec reverse --continue'
    ),
    guidance: `Read test file: ${firstFile}. Then create feature file. Then run fspec link-coverage with --skip-validation.`,
  };
}
```

**The Problem**:
- `session.gaps.files` comes from Problem 3's broken logic
- When existing features exist, `files = featureFiles`
- Guidance says "Read test file: spec/features/existing.feature" ❌

**Real-World Output**:
```bash
$ fspec reverse --strategy=D

<system-reminder>
Step 1 of 1
Strategy: D (Full Reverse ACDD)
After completing this step, run: fspec reverse --continue
</system-reminder>

Read test file: spec/features/cli-command-interface-for-ai-agent-control.feature.
Then create feature file.
Then run fspec link-coverage with --skip-validation.
```

**Why This is Confusing**:
1. Says "Read test file" but shows a .feature file (not a test file!)
2. Says "Then create feature file" but the feature file ALREADY EXISTS!
3. AI is told to create something that's already there

---

## Problem 5: handleStrategyD Doesn't Discover Implementation Files

**Location**: `src/commands/reverse.ts:471-538`

**Current Code**:
```typescript
async function handleStrategyD(
  cwd: string,
  implementationContext: string  // ← Requires MANUAL input!
): Promise<ReverseCommandResult> {
  // Load foundation.json to get personas
  // Build persona-driven system reminder
  // Return guidance
  // NO FILE DISCOVERY!
}
```

**The Problem**:
`handleStrategyD` requires `--implementation-context` flag (manual text input).

It does NOT:
- Scan src/ for implementation files
- Detect which files lack feature files
- Auto-discover files to process
- Maintain workflow state

**Actual Behavior**:
```bash
$ fspec reverse --strategy=D --implementation-context="MusicPlayer.tsx with play/pause"

# Returns guidance but doesn't scan files
# AI must manually specify every file
```

**Expected Behavior**:
```bash
$ fspec reverse --strategy=D

# Should automatically:
# 1. Scan src/ for .tsx, .ts, .jsx, .js files
# 2. Check if each has a corresponding feature file
# 3. List all unmapped implementation files
# 4. Guide AI through each file one by one
```

---

## Problem 6: No Implementation-to-Feature Matching Logic

**Missing Functionality**: Algorithm to determine if an implementation file has a corresponding feature file.

**Current State**: Code can find implementation files (`findImplementationFiles()`) but cannot determine:
- Does `src/components/MusicPlayer.tsx` have a feature file?
- What feature file name should it have?
- How to match implementation → feature?

**Questions from REV-003**:
```
Q: Should Strategy D use glob patterns to find implementation files?
Q: How should Strategy D match implementation files to feature files?
   - By filename similarity (MusicPlayer.tsx → music-player.feature)?
   - By analyzing imports/exports?
```

**Need**: Matching algorithm that can:
1. Convert implementation file path → expected feature file name
2. Check if that feature file exists
3. Handle different naming conventions
4. Verify feature file tags implementation file's work unit

---

## Problem 7: Guidance Text is Wrong for Strategy D

**Location**: `src/commands/reverse.ts:146`

**Current Code**:
```typescript
guidance: `Read test file: ${firstFile}. Then create feature file. Then run fspec link-coverage with --skip-validation.`,
```

**Problems**:
1. Says "Read test file" but `firstFile` is a feature file (not test)
2. Says "create feature file" when feature already exists
3. Generic guidance doesn't reflect Strategy D workflow
4. No mention of personas, example mapping, outside-in BDD

**Expected Guidance for Strategy D**:
```
Step 1 of 50: Process src/MusicPlayer.tsx

1. Read implementation file: src/MusicPlayer.tsx
2. Identify user-facing behavior (not implementation details)
3. Check foundation.json personas - WHO uses this?
4. Create work unit: fspec create-story PREFIX "Feature Name"
5. Use example mapping: fspec add-example, add-rule, add-question
6. Generate scenarios: fspec generate-scenarios <work-unit-id>
7. Create test skeleton based on scenarios
8. Link coverage: fspec link-coverage <feature> --scenario "..." --test-file ... --impl-file src/MusicPlayer.tsx --skip-validation
```

---

## Problem 8: --continue Workflow Doesn't Work for Strategy D

**Location**: `src/commands/reverse.ts:89-113`

**Current Code**:
```typescript
if (options.continue) {
  const session = await loadSession(cwd);
  // ...
  const nextFile = updatedSession.gaps.files[updatedSession.currentStep! - 1];

  return {
    systemReminder: wrapSystemReminder(
      `Step ${updatedSession.currentStep} of ${updatedSession.totalSteps}\n` +
        `Process file: ${nextFile}\n` +
        // ...
    ),
    guidance: `Process test file: ${nextFile}. Read the file, create feature file, then link coverage.`,
  };
}
```

**Problems**:
1. `gaps.files` contains wrong files (feature files, not implementation)
2. Guidance says "Process test file" (generic, not Strategy D-specific)
3. No persona prompts, transformation templates, or BDD guidance
4. Doesn't track which implementation files are completed

---

## Complete Fix Requirements

### Fix 1: Detect Unmapped Implementation Files

**New Logic**:
```typescript
// For each implementation file in src/:
//   1. Derive expected feature file name (e.g., src/MusicPlayer.tsx → music-player.feature)
//   2. Check if spec/features/music-player.feature exists
//   3. If NOT, add to unmappedImplementationFiles array

unmappedImplementationFiles: findUnmappedImplementation(cwd, implementationFiles, featureFiles),
```

**Algorithm**:
```typescript
function deriveFeatureName(implPath: string): string {
  // src/components/MusicPlayer.tsx → music-player
  // src/hooks/usePlaylistStore.ts → use-playlist-store
  // src/utils/formatTime.js → format-time

  const filename = basename(implPath, extname(implPath));
  return filename
    .replace(/([a-z])([A-Z])/g, '$1-$2')  // camelCase → kebab-case
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2')  // PascalCase → kebab-case
    .toLowerCase();
}

function hasFeatureFile(featureName: string, featureFiles: string[]): boolean {
  const expectedPath = `spec/features/${featureName}.feature`;
  return featureFiles.some(f => f.includes(featureName));
}
```

### Fix 2: Change rawImplementation to unmappedImplementation

**New Gap Detection**:
```typescript
return {
  testsWithoutFeatures: ...,
  featuresWithoutTests: ...,
  unmappedScenarios: ...,
  unmappedImplementation: unmappedImplementationFiles.length,  // ← NEW!
  files:
    gaps.unmappedImplementation > 0
      ? unmappedImplementationFiles  // ← Return implementation files for Strategy D!
      : (existing logic for A/B/C)
};
```

### Fix 3: Update Strategy Detection Logic

**Option A**: Keep priority order, fix detection
```typescript
function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  if (gaps.testsWithoutFeatures > 0) return 'A';
  if (gaps.featuresWithoutTests > 0) return 'B';
  if (gaps.unmappedScenarios > 0) return 'C';
  if (gaps.unmappedImplementation > 0) return 'D';  // ← Now works!
  return 'A';
}
```

**Option B**: Allow multi-strategy detection
```typescript
function suggestStrategies(gaps: GapAnalysis): Array<{strategy: string, name: string, count: number}> {
  const strategies = [];

  if (gaps.testsWithoutFeatures > 0) {
    strategies.push({ strategy: 'A', name: 'Spec Gap Filling', count: gaps.testsWithoutFeatures });
  }
  if (gaps.featuresWithoutTests > 0) {
    strategies.push({ strategy: 'B', name: 'Test Gap Filling', count: gaps.featuresWithoutTests });
  }
  if (gaps.unmappedScenarios > 0) {
    strategies.push({ strategy: 'C', name: 'Coverage Mapping', count: gaps.unmappedScenarios });
  }
  if (gaps.unmappedImplementation > 0) {
    strategies.push({ strategy: 'D', name: 'Full Reverse ACDD', count: gaps.unmappedImplementation });
  }

  return strategies;  // Return ALL applicable strategies, let AI choose
}
```

### Fix 4: Update Strategy D Guidance

**New Guidance** (persona-driven, outside-in BDD):
```typescript
if (options.strategy === 'D') {
  const firstFile = session.gaps.files[0];  // Now an implementation file!

  return {
    systemReminder: wrapSystemReminder(
      `STRATEGY D: Outside-in BDD Discovery\n\n` +
      `Step ${session.currentStep} of ${totalSteps}\n` +
      `Implementation file: ${firstFile}\n\n` +
      `WHO uses this? (Check foundation.json personas)\n` +
      `WHAT does [persona] want to accomplish?\n\n` +
      `Transformation templates:\n` +
      `  • button → "User clicks/taps ACTION"\n` +
      `  • useState → "User sees STATE"\n` +
      `  • POST /api → "User completes ACTION"\n\n` +
      `Think outside-in (behavior, not implementation)`
    ),
    guidance:
      `1. Read: ${firstFile}\n` +
      `2. Identify user-facing behavior (not code structure)\n` +
      `3. Check personas: foundation.json\n` +
      `4. Create work unit: fspec create-story PREFIX "Feature Name"\n` +
      `5. Example mapping: fspec add-example <id> "User does X"\n` +
      `6. Generate scenarios: fspec generate-scenarios <id>\n` +
      `7. Create test skeleton\n` +
      `8. Link coverage: fspec link-coverage <feature> --scenario "..." --impl-file ${firstFile} --skip-validation`
  };
}
```

### Fix 5: Add File Matching Configuration

**New Option**: `--matching-strategy` flag

```typescript
enum MatchingStrategy {
  FILENAME = 'filename',  // MusicPlayer.tsx → music-player.feature
  EXACT = 'exact',        // MusicPlayer.tsx → MusicPlayer.feature
  TAG = 'tag',            // Check @work-unit-id tags in features
}
```

**Default**: `filename` (kebab-case conversion)

---

## Concrete Examples

### Example 1: MusicPlayer.tsx Discovery

**Before (Broken)**:
```bash
$ fspec reverse

Gap analysis complete.
Detected: 1 feature files without tests
Suggested: Strategy B (Test Gap Filling)
```

**After (Fixed)**:
```bash
$ fspec reverse

Gap analysis complete.
Detected: 2 implementation files without features
         1 scenario without coverage mapping
Suggested: Strategy D (Full Reverse ACDD)

Multiple strategies available:
  - Strategy C: Coverage Mapping (1 scenario)
  - Strategy D: Full Reverse ACDD (2 files)

To choose Strategy D: fspec reverse --strategy=D
```

### Example 2: Strategy D Workflow

**Before (Broken)**:
```bash
$ fspec reverse --strategy=D

Step 1 of 1
Read test file: spec/features/cli-command-interface-for-ai-agent-control.feature
Then create feature file.  ← Confusing! Feature already exists!
```

**After (Fixed)**:
```bash
$ fspec reverse --strategy=D

<system-reminder>
STRATEGY D: Outside-in BDD Discovery

Step 1 of 2
Implementation file: src/MusicPlayer.tsx

WHO uses this? (Check foundation.json personas)
  - Music Listener
    Goals: Stream music on the go, Discover new artists

WHAT does Music Listener want to accomplish?

Transformation templates:
  • button → "User clicks/taps ACTION"
  • useState → "User sees STATE"
  • POST /api → "User completes ACTION"

Think outside-in (behavior, not implementation)
</system-reminder>

1. Read: src/MusicPlayer.tsx
2. Identify user-facing behavior (not code structure)
3. Check personas: foundation.json
4. Create work unit: fspec create-story MUSIC "Audio Playback Control"
5. Example mapping: fspec add-example MUSIC-001 "User plays song"
6. Generate scenarios: fspec generate-scenarios MUSIC-001
7. Create test skeleton
8. Link coverage: fspec link-coverage audio-playback-control --scenario "..." --impl-file src/MusicPlayer.tsx --skip-validation
```

### Example 3: Multi-File Feature Session

**Before**: Strategy D processes one file per session only

**After**: Feature-scoped sessions (Rule 10 from FEAT-019)

```bash
$ fspec reverse --strategy=D

<system-reminder>
Step 1 of 2: src/MusicPlayer.tsx

Do these files deliver same user behavior?
  - src/MusicPlayer.tsx (UI component)
  - src/hooks/useAudioPlayer.ts (state management)

YES → Continue this session, link both to same feature
NO → Create separate session for useAudioPlayer.ts
</system-reminder>
```

---

## Implementation Checklist

- [ ] Add `deriveFeatureName()` function (camelCase/PascalCase → kebab-case)
- [ ] Add `hasFeatureFile()` function (check if feature exists)
- [ ] Add `findUnmappedImplementation()` function
- [ ] Update `detectGaps()` to calculate `unmappedImplementation`
- [ ] Update `GapAnalysis` type to include `unmappedImplementation: number`
- [ ] Update `files` array logic for Strategy D
- [ ] Rewrite Strategy D guidance (persona-driven, outside-in)
- [ ] Add persona prompts to system-reminders
- [ ] Add transformation templates to system-reminders
- [ ] Update `--continue` workflow for Strategy D
- [ ] Add multi-strategy detection (return array, not single choice)
- [ ] Add `--matching-strategy` option
- [ ] Update tests for new detection logic
- [ ] Update help text for `fspec reverse`

---

## Questions to Answer

1. **File Matching**: Use filename similarity (MusicPlayer.tsx → music-player.feature)?
   - **Recommendation**: YES, use kebab-case conversion as default

2. **Multiple Strategies**: When gaps exist for C AND D, show both?
   - **Recommendation**: YES, list ALL applicable strategies, let AI choose

3. **Feature-scoped sessions**: Allow multiple impl files per work unit?
   - **Recommendation**: YES (per FEAT-019 Rule 10)

4. **Foundation requirement**: Block Strategy D if foundation.json missing?
   - **Recommendation**: NO, prompt to create it but allow manual persona specification

5. **Utility files**: Skip feature files for pure utilities (formatDate, parseJSON)?
   - **Recommendation**: YES (per FEAT-019 Rule 6)

---

## Success Criteria

**After fixes, this should work**:

```bash
# Clean MusicPlayer codebase (no fspec before)
$ cd /path/to/musicplayer-app

$ fspec reverse

Gap analysis complete.
Detected: 15 implementation files without features
Suggested: Strategy D (Full Reverse ACDD)

$ fspec reverse --strategy=D

<system-reminder>
STRATEGY D: Outside-in BDD Discovery
Step 1 of 15
Implementation file: src/components/MusicPlayer.tsx

WHO uses this? ...
</system-reminder>

# AI follows workflow, creates feature, generates scenarios, links coverage

$ fspec reverse --continue

Step 2 of 15: src/hooks/usePlaylistStore.ts
...
```

**Validation**: All 15 implementation files get feature files, test skeletons, and coverage links.

# Reverse ACDD Strategy D - Concrete Fixes with Code Examples

This document provides exact code changes needed to fix all identified problems.

---

## Fix 1: Add Implementation-to-Feature Matching Functions

**File**: `src/commands/reverse.ts`

**Add these new functions** (insert after `scanDirectory()`):

```typescript
/**
 * Derive feature file name from implementation file path
 * Examples:
 *   src/components/MusicPlayer.tsx → music-player
 *   src/hooks/usePlaylistStore.ts → use-playlist-store
 *   src/utils/formatTime.js → format-time
 */
function deriveFeatureName(implPath: string): string {
  const filename = basename(implPath, extname(implPath));

  return filename
    .replace(/([a-z])([A-Z])/g, '$1-$2')  // camelCase → kebab-case
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2')  // PascalCase → kebab-case
    .toLowerCase();
}

/**
 * Check if a feature file exists for an implementation file
 */
function hasFeatureFile(implPath: string, featureFiles: string[]): boolean {
  const featureName = deriveFeatureName(implPath);
  const expectedPath = `spec/features/${featureName}.feature`;

  // Check both full path match and filename match
  return featureFiles.some(
    f => f === expectedPath || f.includes(`/${featureName}.feature`)
  );
}

/**
 * Find implementation files that don't have corresponding feature files
 */
function findUnmappedImplementation(
  implementationFiles: string[],
  featureFiles: string[]
): string[] {
  return implementationFiles.filter(
    implFile => !hasFeatureFile(implFile, featureFiles)
  );
}

/**
 * Check if a file is a pure utility (should skip feature file creation)
 * Per FEAT-019 Rule 6: Skip utilities like formatDate, parseJSON
 */
function isPureUtility(implPath: string): boolean {
  const utilityPatterns = [
    /utils\/format/i,
    /utils\/parse/i,
    /utils\/validate/i,
    /helpers\//i,
    /constants\//i,
  ];

  return utilityPatterns.some(pattern => pattern.test(implPath));
}
```

**Import statements to add**:
```typescript
import { basename, extname } from 'path';
```

---

## Fix 2: Update GapAnalysis Type

**File**: `src/types/reverse-session.ts`

**Change**:
```typescript
export interface GapAnalysis {
  testsWithoutFeatures: number;
  featuresWithoutTests: number;
  unmappedScenarios: number;
  rawImplementation: number;  // ← REMOVE THIS
  unmappedImplementation: number;  // ← ADD THIS
  files: string[];
}
```

---

## Fix 3: Rewrite detectGaps() Function

**File**: `src/commands/reverse.ts`

**Replace entire function** (lines 335-360):

```typescript
function detectGaps(analysis: AnalysisResult): GapAnalysis {
  const { testFiles, featureFiles, implementationFiles, coverageAnalysis } =
    analysis;

  const unmappedCount = coverageAnalysis?.unmappedCount || 0;

  // Find implementation files without feature files
  const unmappedImplFiles = findUnmappedImplementation(
    implementationFiles,
    featureFiles
  ).filter(f => !isPureUtility(f));  // Exclude pure utilities

  // Determine which files to process based on gap type
  let files: string[] = [];

  if (testFiles.length > 0 && featureFiles.length === 0) {
    // Strategy A: Tests exist, no features → Process test files
    files = testFiles;
  } else if (featureFiles.length > 0 && testFiles.length === 0) {
    // Strategy B: Features exist, no tests → Process feature files
    files = featureFiles;
  } else if (unmappedCount > 0) {
    // Strategy C: Both exist, but scenarios unmapped → Process scenarios
    files = coverageAnalysis?.scenarios || [];
  } else if (unmappedImplFiles.length > 0) {
    // Strategy D: Implementation files without features → Process impl files
    files = unmappedImplFiles;
  }

  return {
    testsWithoutFeatures:
      testFiles.length > 0 && featureFiles.length === 0 ? testFiles.length : 0,
    featuresWithoutTests:
      featureFiles.length > 0 && testFiles.length === 0
        ? featureFiles.length
        : 0,
    unmappedScenarios: unmappedCount,
    unmappedImplementation: unmappedImplFiles.length,  // ← NEW!
    files,
  };
}
```

**Key Changes**:
1. ✅ Calls `findUnmappedImplementation()` to detect impl files without features
2. ✅ Filters out pure utilities using `isPureUtility()`
3. ✅ Sets `unmappedImplementation` count (not tied to "zero features" condition)
4. ✅ Updates `files` array logic to include implementation files for Strategy D

---

## Fix 4: Update suggestStrategy() Function

**File**: `src/commands/reverse.ts`

**Replace function** (lines 362-368):

```typescript
function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  if (gaps.testsWithoutFeatures > 0) return 'A';
  if (gaps.featuresWithoutTests > 0) return 'B';
  if (gaps.unmappedScenarios > 0) return 'C';
  if (gaps.unmappedImplementation > 0) return 'D';  // ← Changed from rawImplementation
  return 'A'; // Default fallback
}
```

**Alternative**: Multi-strategy detection (return all applicable strategies)

```typescript
/**
 * Detect ALL applicable strategies (not just one)
 */
function detectStrategies(gaps: GapAnalysis): Array<{
  strategy: 'A' | 'B' | 'C' | 'D';
  name: string;
  count: number;
  priority: number;
}> {
  const strategies = [];

  if (gaps.testsWithoutFeatures > 0) {
    strategies.push({
      strategy: 'A',
      name: 'Spec Gap Filling',
      count: gaps.testsWithoutFeatures,
      priority: 1,
    });
  }

  if (gaps.featuresWithoutTests > 0) {
    strategies.push({
      strategy: 'B',
      name: 'Test Gap Filling',
      count: gaps.featuresWithoutTests,
      priority: 2,
    });
  }

  if (gaps.unmappedScenarios > 0) {
    strategies.push({
      strategy: 'C',
      name: 'Coverage Mapping',
      count: gaps.unmappedScenarios,
      priority: 3,
    });
  }

  if (gaps.unmappedImplementation > 0) {
    strategies.push({
      strategy: 'D',
      name: 'Full Reverse ACDD',
      count: gaps.unmappedImplementation,
      priority: 4,
    });
  }

  return strategies.sort((a, b) => a.priority - b.priority);
}

/**
 * Suggest primary strategy (highest priority)
 */
function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  const strategies = detectStrategies(gaps);
  return strategies.length > 0 ? strategies[0].strategy : 'A';
}
```

---

## Fix 5: Update formatGaps() Function

**File**: `src/commands/reverse.ts`

**Replace function** (lines 380-394):

```typescript
function formatGaps(gaps: GapAnalysis): string {
  if (gaps.testsWithoutFeatures > 0) {
    return `${gaps.testsWithoutFeatures} test files without features`;
  }
  if (gaps.featuresWithoutTests > 0) {
    return `${gaps.featuresWithoutTests} feature files without tests`;
  }
  if (gaps.unmappedScenarios > 0) {
    return `${gaps.unmappedScenarios} scenarios without coverage mappings`;
  }
  if (gaps.unmappedImplementation > 0) {  // ← Changed from rawImplementation
    return `${gaps.unmappedImplementation} implementation files without features`;
  }
  return 'No gaps detected';
}

/**
 * Format ALL detected gaps (not just primary)
 */
function formatAllGaps(gaps: GapAnalysis): string {
  const parts: string[] = [];

  if (gaps.testsWithoutFeatures > 0) {
    parts.push(`${gaps.testsWithoutFeatures} test files without features`);
  }
  if (gaps.featuresWithoutTests > 0) {
    parts.push(`${gaps.featuresWithoutTests} feature files without tests`);
  }
  if (gaps.unmappedScenarios > 0) {
    parts.push(`${gaps.unmappedScenarios} scenarios without coverage`);
  }
  if (gaps.unmappedImplementation > 0) {
    parts.push(`${gaps.unmappedImplementation} implementation files without features`);
  }

  return parts.length > 0 ? parts.join(', ') : 'No gaps detected';
}
```

---

## Fix 6: Rewrite Strategy D Guidance (Persona-Driven)

**File**: `src/commands/reverse.ts`

**Replace code block** (lines 116-148):

```typescript
if (options.strategy) {
  // Strategy D: Outside-in BDD with persona-driven guidance (can work without session)
  if (options.strategy === 'D' && options.implementationContext) {
    return await handleStrategyD(cwd, options.implementationContext);
  }

  const session = await loadSession(cwd);
  if (!session) {
    return { message: 'No active reverse session', exitCode: 1 };
  }

  const strategyName = getStrategyName(options.strategy);
  const totalSteps = session.gaps.files.length;
  const updatedSession = setStrategy(
    session,
    options.strategy as any,
    strategyName,
    totalSteps
  );
  await saveSession(cwd, updatedSession);

  const firstFile = session.gaps.files[0];

  // Strategy D has different guidance (persona-driven, outside-in BDD)
  if (options.strategy === 'D') {
    return await generateStrategyDGuidance(cwd, session, firstFile, totalSteps);
  }

  // Strategy A/B/C guidance (existing)
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

**Add new function** (after `handleStrategyD()`):

```typescript
/**
 * Generate persona-driven guidance for Strategy D
 */
async function generateStrategyDGuidance(
  cwd: string,
  session: any,
  implFile: string,
  totalSteps: number
): Promise<ReverseCommandResult> {
  // Load foundation.json to get personas
  const foundationPath = join(cwd, 'spec', 'foundation.json');
  let personas: Array<{ name: string; description: string; goals: string[] }> = [];
  let hasFoundation = false;

  try {
    const foundationContent = await fs.readFile(foundationPath, 'utf-8');
    const foundation = JSON.parse(foundationContent);
    personas = foundation.personas || [];
    hasFoundation = true;
  } catch {
    // Foundation.json doesn't exist or is invalid
  }

  // Build persona-driven system reminder
  let systemReminder = 'STRATEGY D: Outside-in BDD Discovery\n\n';
  systemReminder += `Step ${session.currentStep || 1} of ${totalSteps}\n`;
  systemReminder += `Implementation file: ${implFile}\n\n`;

  if (hasFoundation && personas.length > 0) {
    systemReminder += 'WHO uses this? (Check foundation.json personas)\n';
    personas.forEach(persona => {
      systemReminder += `  - ${persona.name}`;
      if (persona.description) {
        systemReminder += ` (${persona.description})`;
      }
      systemReminder += '\n';
      if (persona.goals && persona.goals.length > 0) {
        systemReminder += `    Goals: ${persona.goals.join(', ')}\n`;
      }
    });
    systemReminder += '\n';
    systemReminder += `WHAT does ${personas[0].name} want to accomplish?\n\n`;
  } else {
    systemReminder += '⚠️  Foundation.json not found or has no personas.\n';
    systemReminder += 'Run: fspec discover-foundation\n';
    systemReminder += 'Or add persona: fspec add-persona "User Name" "Description" --goal "Goal"\n\n';
  }

  systemReminder += 'Transformation templates (implementation → behavior):\n';
  systemReminder += '  • UI Elements → User Actions\n';
  systemReminder += '    button → "User clicks/taps ACTION"\n';
  systemReminder += '    input → "User enters DATA"\n';
  systemReminder += '    checkbox → "User selects/deselects OPTION"\n';
  systemReminder += '  • State → User Expectations\n';
  systemReminder += '    useState → "User sees STATE"\n';
  systemReminder += '    loading → "User waits for PROCESS"\n';
  systemReminder += '    error → "User sees error message"\n';
  systemReminder += '  • API Endpoints → User Needs\n';
  systemReminder += '    POST /orders → "User completes order"\n';
  systemReminder += '    GET /profile → "User views profile"\n';
  systemReminder += '    DELETE /item → "User removes item"\n\n';

  systemReminder += 'Think outside-in (behavior, NOT implementation):\n';
  systemReminder += '  ❌ "Component has play/pause buttons"\n';
  systemReminder += '  ✅ "User controls playback"\n\n';

  systemReminder += 'Not which system calls it, but who BENEFITS?\n';
  systemReminder += '  ❌ "calculateDiscount() is called by checkout"\n';
  systemReminder += '  ✅ "Shopper benefits from accurate discounts"\n';

  // Build step-by-step guidance
  let guidance = `STRATEGY D WORKFLOW:\n\n`;
  guidance += `1. Read implementation file: ${implFile}\n`;
  guidance += `   Understand WHAT it does (user perspective), not HOW (code details)\n\n`;

  guidance += `2. Identify user-facing behavior\n`;
  if (hasFoundation && personas.length > 0) {
    guidance += `   Which persona from foundation.json uses this?\n`;
    guidance += `   What does ${personas[0].name} want to accomplish?\n\n`;
  } else {
    guidance += `   WHO benefits from this functionality?\n`;
    guidance += `   WHAT do they want to accomplish?\n\n`;
  }

  guidance += `3. Create work unit (story)\n`;
  guidance += `   fspec create-story PREFIX "Feature Name"\n`;
  guidance += `   Example: fspec create-story MUSIC "Audio Playback Control"\n\n`;

  guidance += `4. Use Example Mapping to discover behavior\n`;
  guidance += `   fspec add-rule <work-unit-id> "User can play songs"\n`;
  guidance += `   fspec add-example <work-unit-id> "User taps play button, song starts"\n`;
  guidance += `   fspec add-question <work-unit-id> "@human: What happens when battery is low?"\n\n`;

  guidance += `5. Generate feature file from examples\n`;
  guidance += `   fspec generate-scenarios <work-unit-id>\n`;
  guidance += `   This creates spec/features/<feature-name>.feature\n\n`;

  guidance += `6. Create test skeleton based on scenarios\n`;
  guidance += `   Create test file matching feature scenarios\n`;
  guidance += `   Use TODO comments for unimplemented tests\n\n`;

  guidance += `7. Link coverage to existing implementation\n`;
  guidance += `   fspec link-coverage <feature-name> --scenario "Scenario Name" \\\n`;
  guidance += `     --test-file <test-path> --test-lines <range> \\\n`;
  guidance += `     --impl-file ${implFile} --impl-lines <lines> \\\n`;
  guidance += `     --skip-validation\n\n`;

  guidance += `8. Continue to next file\n`;
  guidance += `   fspec reverse --continue\n`;

  return {
    systemReminder: wrapSystemReminder(systemReminder),
    guidance,
  };
}
```

---

## Fix 7: Update --continue Workflow for Strategy D

**File**: `src/commands/reverse.ts`

**Replace code block** (lines 89-113):

```typescript
if (options.continue) {
  const session = await loadSession(cwd);
  if (!session) {
    return { message: 'No active reverse session', exitCode: 1 };
  }

  const updatedSession = incrementStep(session);
  await saveSession(cwd, updatedSession);

  const isFinalStep =
    updatedSession.currentStep === updatedSession.totalSteps;
  const nextFile = updatedSession.gaps.files[updatedSession.currentStep! - 1];

  // Strategy D has different guidance
  if (updatedSession.strategy === 'D') {
    return await generateStrategyDGuidance(
      cwd,
      updatedSession,
      nextFile,
      updatedSession.totalSteps!
    );
  }

  // Strategy A/B/C guidance (existing)
  return {
    systemReminder: wrapSystemReminder(
      `Step ${updatedSession.currentStep} of ${updatedSession.totalSteps}\n` +
        `Process file: ${nextFile}\n` +
        (isFinalStep
          ? 'After completing this final step, run: fspec reverse --complete'
          : 'After completing this step, run: fspec reverse --continue')
    ),
    guidance: `Process test file: ${nextFile}. Read the file, create feature file, then link coverage.`,
  };
}
```

---

## Fix 8: Update Initial Analysis Output (Multi-Strategy Detection)

**File**: `src/commands/reverse.ts`

**Replace result creation** (lines 220-246):

```typescript
// Create session in gap-detection phase
const session = createSession(
  'gap-detection',
  gaps,
  suggestedStrategy,
  strategyName
);
await saveSession(cwd, session);

// Detect ALL applicable strategies (not just primary)
const allStrategies = detectStrategies(gaps);
const totalGaps =
  gaps.testsWithoutFeatures +
  gaps.featuresWithoutTests +
  gaps.unmappedScenarios +
  gaps.unmappedImplementation;  // ← Changed from rawImplementation

// Build system reminder
let systemReminderText = 'Gap analysis complete.\n\n';

if (allStrategies.length > 1) {
  // Multiple strategies detected
  systemReminderText += 'Multiple strategies available:\n';
  allStrategies.forEach(s => {
    systemReminderText += `  - Strategy ${s.strategy}: ${s.name} (${s.count} ${s.strategy === 'C' ? 'scenarios' : 'files'})\n`;
  });
  systemReminderText += '\n';
  systemReminderText += `Suggested: Strategy ${suggestedStrategy} (${strategyName})\n`;
  systemReminderText += `To choose this strategy: fspec reverse --strategy=${suggestedStrategy}\n`;
  systemReminderText += `To choose different: fspec reverse --strategy=<A|B|C|D>`;
} else {
  // Single strategy detected
  systemReminderText += `Detected: ${formatGaps(gaps)}\n`;
  systemReminderText += `Suggested: Strategy ${suggestedStrategy} (${strategyName})\n`;
  systemReminderText += `To choose this strategy: fspec reverse --strategy=${suggestedStrategy}`;
}

const result: ReverseCommandResult = {
  analysis,
  gaps,
  suggestedStrategy,
  strategyName,
  systemReminder: wrapSystemReminder(systemReminderText),
  guidance: generateStrategyGuidance(suggestedStrategy, gaps),
  effortEstimate: getEffortEstimate(suggestedStrategy, gaps),
};

// Add pagination and summary for large projects (100+ gaps)
if (totalGaps >= 100) {
  result.pagination = {
    total: totalGaps,
    perPage: 50,
    page: 1,
  };
  result.summary = `Found ${formatAllGaps(gaps)}`;
  result.guidance = `${result.guidance}\n\nLarge project detected. Use --strategy=${suggestedStrategy} to begin.`;
}

return result;
```

---

## Fix 9: Update Strategy D Guidance Text

**File**: `src/commands/reverse.ts`

**Replace in `generateStrategyGuidance()`** (lines 400-408):

```typescript
function generateStrategyGuidance(strategy: string, gaps: GapAnalysis): string {
  const guidanceMap: Record<string, string> = {
    A: 'Create feature files for existing tests. Reverse engineer acceptance criteria from test assertions.',
    B: 'Create test skeletons for existing feature files. Use --skip-validation when linking coverage.',
    C: 'Quick wins - no new files needed. Link existing tests to scenarios using fspec link-coverage.',
    D: 'Outside-in BDD: Analyze implementation files, identify user behavior, create personas, example maps, features, and test skeletons. Links to existing code.',  // ← Updated!
  };
  return guidanceMap[strategy] || '';
}
```

---

## Fix 10: Update Effort Estimate for Strategy D

**File**: `src/commands/reverse.ts`

**Replace in `getEffortEstimate()`** (lines 410-429):

```typescript
function getEffortEstimate(strategy: string, gaps: GapAnalysis): string {
  const totalGaps =
    gaps.testsWithoutFeatures +
    gaps.featuresWithoutTests +
    gaps.unmappedScenarios +
    gaps.unmappedImplementation;  // ← Changed from rawImplementation

  switch (strategy) {
    case 'A':
      return `${totalGaps * 2}-${totalGaps * 3} points (reverse engineer specs from tests)`;
    case 'B':
      return `${totalGaps}-${totalGaps * 2} points (create test skeletons)`;
    case 'C':
      return '1 point total (link existing tests to scenarios)';
    case 'D':
      return `${totalGaps * 3}-${totalGaps * 5} points (full discovery: personas, example maps, features, tests)`;
    default:
      return 'Unknown';
  }
}
```

---

## Testing the Fixes

### Test Case 1: MusicPlayer Project (No fspec before)

**Setup**:
```bash
mkdir -p /tmp/musicplayer-test/src/components
mkdir -p /tmp/musicplayer-test/src/hooks

# Create implementation files (no tests, no features)
cat > /tmp/musicplayer-test/src/components/MusicPlayer.tsx << 'EOF'
export const MusicPlayer = () => {
  const [isPlaying, setIsPlaying] = useState(false);
  return (
    <div>
      <button onClick={() => setIsPlaying(!isPlaying)}>
        {isPlaying ? 'Pause' : 'Play'}
      </button>
    </div>
  );
};
EOF

cat > /tmp/musicplayer-test/src/hooks/usePlaylistStore.ts << 'EOF'
export const usePlaylistStore = () => {
  const addSong = (song: Song) => { /* ... */ };
  const removeSong = (id: string) => { /* ... */ };
  return { addSong, removeSong };
};
EOF
```

**Expected Output (After Fix)**:
```bash
$ cd /tmp/musicplayer-test
$ fspec reverse

<system-reminder>
Gap analysis complete.

Detected: 2 implementation files without features
Suggested: Strategy D (Full Reverse ACDD)
To choose this strategy: fspec reverse --strategy=D
</system-reminder>

Outside-in BDD: Analyze implementation files, identify user behavior, create personas,
example maps, features, and test skeletons. Links to existing code.

Effort estimate: 6-10 points (full discovery: personas, example maps, features, tests)


$ fspec reverse --strategy=D

<system-reminder>
STRATEGY D: Outside-in BDD Discovery

Step 1 of 2
Implementation file: src/components/MusicPlayer.tsx

WHO uses this? (Check foundation.json personas)
⚠️  Foundation.json not found or has no personas.
Run: fspec discover-foundation
Or add persona: fspec add-persona "User Name" "Description" --goal "Goal"

Transformation templates (implementation → behavior):
  • UI Elements → User Actions
    button → "User clicks/taps ACTION"
    input → "User enters DATA"
  • State → User Expectations
    useState → "User sees STATE"
    loading → "User waits for PROCESS"

Think outside-in (behavior, NOT implementation):
  ❌ "Component has play/pause buttons"
  ✅ "User controls playback"

Not which system calls it, but who BENEFITS?
  ❌ "calculateDiscount() is called by checkout"
  ✅ "Shopper benefits from accurate discounts"
</system-reminder>

STRATEGY D WORKFLOW:

1. Read implementation file: src/components/MusicPlayer.tsx
   Understand WHAT it does (user perspective), not HOW (code details)

2. Identify user-facing behavior
   WHO benefits from this functionality?
   WHAT do they want to accomplish?

3. Create work unit (story)
   fspec create-story PREFIX "Feature Name"
   Example: fspec create-story MUSIC "Audio Playback Control"

4. Use Example Mapping to discover behavior
   fspec add-rule <work-unit-id> "User can play songs"
   fspec add-example <work-unit-id> "User taps play button, song starts"
   fspec add-question <work-unit-id> "@human: What happens when battery is low?"

5. Generate feature file from examples
   fspec generate-scenarios <work-unit-id>
   This creates spec/features/<feature-name>.feature

6. Create test skeleton based on scenarios
   Create test file matching feature scenarios
   Use TODO comments for unimplemented tests

7. Link coverage to existing implementation
   fspec link-coverage <feature-name> --scenario "Scenario Name" \
     --test-file <test-path> --test-lines <range> \
     --impl-file src/components/MusicPlayer.tsx --impl-lines <lines> \
     --skip-validation

8. Continue to next file
   fspec reverse --continue
```

### Test Case 2: Project with Multiple Gap Types

**Setup**:
```bash
# Has tests, features, AND unmapped implementation files
mkdir -p /tmp/mixed-test/src
mkdir -p /tmp/mixed-test/spec/features
mkdir -p /tmp/mixed-test/src/__tests__

# Unmapped implementation file
echo "export const NewFeature = () => {};" > /tmp/mixed-test/src/NewFeature.tsx

# Feature without test
cat > /tmp/mixed-test/spec/features/existing.feature << 'EOF'
Feature: Existing Feature
  Scenario: Test scenario
EOF

# Test without feature
cat > /tmp/mixed-test/src/__tests__/orphan.test.ts << 'EOF'
describe('Orphan', () => {
  it('should work', () => {});
});
EOF
```

**Expected Output (After Fix)**:
```bash
$ cd /tmp/mixed-test
$ fspec reverse

<system-reminder>
Gap analysis complete.

Multiple strategies available:
  - Strategy A: Spec Gap Filling (1 files)
  - Strategy B: Test Gap Filling (1 files)
  - Strategy D: Full Reverse ACDD (1 files)

Suggested: Strategy A (Spec Gap Filling)
To choose this strategy: fspec reverse --strategy=A
To choose different: fspec reverse --strategy=<A|B|C|D>
</system-reminder>

# AI can now choose Strategy D even though A/B also apply!
$ fspec reverse --strategy=D

# Processes src/NewFeature.tsx with persona-driven guidance
```

---

## Summary of All Changes

| File | Function/Section | Change |
|------|-----------------|--------|
| `src/commands/reverse.ts` | Add new functions | `deriveFeatureName()`, `hasFeatureFile()`, `findUnmappedImplementation()`, `isPureUtility()` |
| `src/commands/reverse.ts` | Add new function | `generateStrategyDGuidance()` (persona-driven guidance) |
| `src/commands/reverse.ts` | `detectGaps()` | Complete rewrite to detect unmapped implementation files |
| `src/commands/reverse.ts` | `suggestStrategy()` | Change `rawImplementation` → `unmappedImplementation` |
| `src/commands/reverse.ts` | Add new function | `detectStrategies()` (multi-strategy detection) |
| `src/commands/reverse.ts` | `formatGaps()` | Change `rawImplementation` → `unmappedImplementation` |
| `src/commands/reverse.ts` | Add new function | `formatAllGaps()` (show all gaps, not just primary) |
| `src/commands/reverse.ts` | Strategy D handling | Use `generateStrategyDGuidance()` instead of generic guidance |
| `src/commands/reverse.ts` | `--continue` handling | Use `generateStrategyDGuidance()` for Strategy D |
| `src/commands/reverse.ts` | Initial analysis | Show all strategies when multiple detected |
| `src/commands/reverse.ts` | `generateStrategyGuidance()` | Update Strategy D description |
| `src/commands/reverse.ts` | `getEffortEstimate()` | Change `rawImplementation` → `unmappedImplementation`, update D estimate |
| `src/types/reverse-session.ts` | `GapAnalysis` | Change `rawImplementation` → `unmappedImplementation` |

---

## Validation Checklist

After implementing fixes:

- [ ] `deriveFeatureName()` converts camelCase/PascalCase to kebab-case
- [ ] `hasFeatureFile()` matches implementation files to features
- [ ] `findUnmappedImplementation()` finds impl files without features
- [ ] `isPureUtility()` filters utilities (formatDate, parseJSON, etc.)
- [ ] `detectGaps()` correctly calculates `unmappedImplementation`
- [ ] `detectGaps()` sets `files` array to implementation files for Strategy D
- [ ] `suggestStrategy()` detects Strategy D when unmapped impl files exist
- [ ] `detectStrategies()` returns ALL applicable strategies
- [ ] Strategy D guidance includes persona prompts
- [ ] Strategy D guidance includes transformation templates
- [ ] Strategy D guidance shows step-by-step workflow
- [ ] `--continue` uses Strategy D guidance for subsequent steps
- [ ] Multi-strategy output shows all options when multiple gaps exist
- [ ] MusicPlayer test case works end-to-end
- [ ] Mixed gap types test case shows all strategies

---

## Next Steps

1. Implement all fixes in `src/commands/reverse.ts`
2. Update `src/types/reverse-session.ts` type definition
3. Write tests for new functions
4. Test with MusicPlayer example project
5. Test with mixed gap types project
6. Update `fspec reverse --help` documentation
7. Update spec/CLAUDE.md with new Strategy D workflow
8. Create feature file for fixes (reverse-acdd-strategy-d-fixes.feature)
9. Mark FEAT-019 and REV-003 as complete

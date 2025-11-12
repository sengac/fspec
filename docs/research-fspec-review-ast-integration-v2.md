# Research: AST-Based Code Analysis Tools for fspec review (v2 - Data Gathering Focus)

**Date:** 2025-01-12
**Author:** Claude (AI Research Assistant)
**Purpose:** Strategy for enhancing `fspec review` command with AST-based structural data gathering (NO semantic analysis)

---

## Executive Summary

This research document outlines a strategy for refactoring the `fspec review` command to provide AI agents with structural code data via AST analysis.

**CRITICAL DESIGN PRINCIPLE: NO SEMANTIC CODE ANALYSIS IN FSPEC**

fspec's responsibility is ONLY to:
- ✅ Gather structural data using AST tools
- ✅ Present data in system-reminders for AI interpretation
- ✅ Suggest commands AI can run for deeper investigation

fspec MUST NOT:
- ❌ Determine if code violates DRY/SOLID principles
- ❌ Calculate similarity scores or make quality judgments
- ❌ Implement heuristics or algorithms to detect code smells
- ❌ Make ANY semantic decisions about code quality

**The AI agent calling fspec makes ALL analysis decisions.**

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Current State Analysis](#current-state-analysis)
3. [AST Tool Capabilities](#ast-tool-capabilities)
4. [Data Gathering Strategy](#data-gathering-strategy)
5. [System-Reminder Design](#system-reminder-design)
6. [Integration Architecture](#integration-architecture)
7. [Implementation Plan](#implementation-plan)
8. [Code Examples](#code-examples)
9. [Testing Strategy](#testing-strategy)

---

## Design Philosophy

### The Line Between Tool and Analysis

**fspec is a TOOL, not an ANALYZER.**

| fspec's Role (✅) | AI Agent's Role (✅) | WRONG (❌) |
|-------------------|----------------------|------------|
| Run AST queries | Interpret AST results | fspec interprets results |
| Count functions/classes | Decide if count is too high | fspec decides what's "too high" |
| List imports/exports | Analyze dependency patterns | fspec analyzes patterns |
| Build dependency graph | Detect circular dependencies | fspec detects circles |
| Extract function signatures | Compare for duplication | fspec calculates similarity |
| Measure class size | Judge if class violates SRP | fspec judges SRP violations |

**Example of CORRECT design:**

```xml
<system-reminder>
STRUCTURAL DATA for REVIEW-001 implementation files:

src/commands/review.ts:
  - Functions: 12
  - Classes: 1 (ReviewCommand)
  - Public methods in ReviewCommand: 15
  - Imports: 23
  - Lines of code: 587

src/commands/validate.ts:
  - Functions: 8
  - Classes: 1 (ValidateCommand)
  - Public methods in ValidateCommand: 6
  - Imports: 12
  - Lines of code: 342

Functions with similar names detected:
  - formatOutput() in review.ts:156
  - formatOutput() in validate.ts:234

To investigate potential duplication, run:
  fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts

AI: Review this data and determine if refactoring is needed.
</system-reminder>
```

**Example of WRONG design (DO NOT DO THIS):**

```xml
<system-reminder>
CODE QUALITY VIOLATIONS DETECTED:

🔴 ERROR: ReviewCommand violates Single Responsibility Principle
   - 15 public methods exceed threshold of 10
   - Recommendation: Split into ReviewCommand, ReportGenerator, ACDDValidator

🟡 WARNING: Duplicate function detected (similarity: 92%)
   - review.ts:156 and validate.ts:234
   - Recommendation: Extract to shared utility

AI: Fix these violations before proceeding.
</system-reminder>
```

**Why the second example is WRONG:**
- fspec determined SRP violation (semantic judgment)
- fspec calculated similarity score (semantic analysis)
- fspec recommended specific refactoring (design decision)
- fspec told AI what to do (removes AI agency)

---

## Current State Analysis

### Existing `fspec review` Command

**Location:** `src/commands/review.ts` (587 lines)

**Current Capabilities:**
1. ✅ ACDD compliance checking
2. ✅ TypeScript violations detection (any types, require(), file extensions)
3. ✅ Gherkin syntax validation
4. ✅ Test coverage analysis
5. ✅ AI-driven deep code analysis via system-reminders

**Current Limitations:**
1. ❌ No AST data gathering beyond basic TypeScript violations
2. ❌ No structural metrics (function counts, class sizes, etc.)
3. ❌ No dependency graph data
4. ❌ No function/class name listings for AI to analyze
5. ❌ No guidance suggesting AST commands AI could run

### Existing `fspec research` Command

**Location:** `src/commands/research.ts` (295 lines)

**Current Capabilities:**
1. ✅ AST tool with tree-sitter support for 12+ languages
2. ✅ Predefined operations (list-functions, find-class, etc.)
3. ✅ Parametric filtering (name, pattern, min-params)

**Current Usage:**
- Manual invocation during Example Mapping
- NOT integrated with `fspec review` command

---

## AST Tool Capabilities

### Supported Languages

The AST tool supports:
- JavaScript/TypeScript
- Python, Go, Rust, Java, Ruby, C#, PHP, C++, Bash, JSON

### Available Operations

1. **`list-functions`** - Find all function declarations/expressions/methods
2. **`list-classes`** - Find all class declarations
3. **`list-imports`** - Find all import statements
4. **`list-exports`** - Find all export statements
5. **`find-async-functions`** - Find async function declarations
6. **`find-identifiers`** - Find identifiers matching pattern

### Output Format

All operations return JSON with:
- `type` - AST node type
- `name` - Function/class/identifier name
- `line` - Line number
- `column` - Column position
- `text` - Full code text (optional)

**Example:**
```json
{
  "matches": [
    {
      "type": "function_declaration",
      "name": "review",
      "line": 45,
      "column": 7,
      "text": "async function review(workUnitId: string) { ... }"
    }
  ]
}
```

---

## Data Gathering Strategy

### What Data to Gather

The `fspec review` command will gather:

#### 1. Function Metrics
- Total function count per file
- Function names
- Function locations (file:line)
- Functions with similar names across files

#### 2. Class Metrics
- Total class count per file
- Class names
- Method count per class (public vs private if detectable)
- Class locations (file:line)

#### 3. Import/Export Data
- Total import count per file
- Import sources (external vs internal)
- Export count per file
- Export types (default vs named)

#### 4. File Size Metrics
- Lines of code per file
- Number of functions per file
- Number of classes per file

### How to Gather Data

**For each implementation file:**

```bash
# Get all functions
fspec research --tool=ast --operation=list-functions --file=<file>

# Get all classes
fspec research --tool=ast --operation=list-classes --file=<file>

# Get all imports
fspec research --tool=ast --operation=list-imports --file=<file>

# Get all exports
fspec research --tool=ast --operation=list-exports --file=<file>
```

**Aggregate results:**
- Count totals
- Group by file
- Identify similar names (exact string match or contains)
- DO NOT calculate similarity scores
- DO NOT make judgments about "good" or "bad"

### What NOT to Do

❌ **DO NOT calculate:**
- Similarity scores (Levenshtein distance, fuzzy matching)
- Complexity metrics (cyclomatic complexity)
- Quality scores or grades
- Violation counts

❌ **DO NOT determine:**
- If code violates DRY/SOLID
- If classes are "too large"
- If functions are "duplicates"
- If dependencies are "too many"

❌ **DO NOT implement:**
- Heuristics or thresholds
- Semantic analysis algorithms
- Code smell detection
- Automatic refactoring suggestions

---

## System-Reminder Design

### Structure

System-reminders should:
1. **Present data** in clear, structured format
2. **Suggest commands** AI can run for deeper analysis
3. **Ask questions** to guide AI thinking
4. **NOT make judgments** or determine violations

### Template

```xml
<system-reminder>
CODE STRUCTURE DATA for {work-unit-id}

Implementation files analyzed: {count}

{for each file:}
File: {file-path}
  Functions: {count}
    {list function names with line numbers}
  Classes: {count}
    {list class names with method counts}
  Imports: {count}
  Exports: {count}
  Lines of code: {count}

Patterns detected:
  - {pattern description without judgment}

To investigate further, run these AST commands:
  {suggested fspec research commands}

AI: Analyze this structural data and determine if refactoring is needed.
Consider:
  - Are there functions with similar names that might have duplicate logic?
  - Are there classes with many methods that might violate SRP?
  - Are there many imports that might indicate tight coupling?
  - Are there opportunities to extract shared utilities?

Use the suggested AST commands to gather more details as needed.
</system-reminder>
```

### Example System-Reminder

```xml
<system-reminder>
CODE STRUCTURE DATA for REVIEW-001

Implementation files analyzed: 3

File: src/commands/review.ts (587 lines)
  Functions: 12
    - review() at line 45
    - formatOutput() at line 156
    - checkACDDCompliance() at line 234
    - loadCoverageData() at line 312
    - validateFeatureFile() at line 389
    - generateReport() at line 456
    - (6 more functions...)
  Classes: 1
    - ReviewCommand (15 public methods detected)
  Imports: 23
  Exports: 2 (1 default, 1 named)

File: src/commands/validate.ts (342 lines)
  Functions: 8
    - validate() at line 28
    - formatOutput() at line 234
    - parseGherkin() at line 156
    - (5 more functions...)
  Classes: 1
    - ValidateCommand (6 public methods detected)
  Imports: 12
  Exports: 2 (1 default, 1 named)

File: src/utils/formatter.ts (123 lines)
  Functions: 5
  Classes: 0
  Imports: 4
  Exports: 5 (all named)

Patterns detected (NOT violations, just observations):
  - Function name "formatOutput" appears in 2 files:
    * src/commands/review.ts:156
    * src/commands/validate.ts:234
  - ReviewCommand class has 15 public methods
  - src/commands/review.ts has 23 imports

To investigate potential duplication:
  fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts
  fspec research --tool=ast --operation=list-functions --file=src/commands/validate.ts

To analyze class structure:
  fspec research --tool=ast --operation=list-classes --file=src/commands/review.ts

To examine dependencies:
  fspec research --tool=ast --operation=list-imports --file=src/commands/review.ts

AI: Review this structural data and decide if refactoring is warranted.
Consider DRY and SOLID principles:

DRY (Don't Repeat Yourself):
  - Do formatOutput() functions have similar implementations?
  - Are there other functions with duplicate logic?
  - Could shared utilities be extracted?

SOLID Principles:
  - Single Responsibility: Does ReviewCommand have too many responsibilities?
  - Dependency Inversion: Are there too many concrete dependencies?

Use Read tool on the files mentioned above and the suggested AST commands to gather more information.
</system-reminder>
```

### Key Elements

1. **Data presentation**: Counts, names, locations (facts only)
2. **Pattern observation**: Neutral descriptions (NOT "violations detected")
3. **Suggested commands**: Specific AST commands AI can run
4. **Guidance questions**: Prompts for AI to consider (NOT instructions)
5. **Principle reminders**: Brief definitions of DRY/SOLID (NOT enforcement)

---

## Integration Architecture

### High-Level Flow

```
fspec review REVIEW-001
  ↓
Load work unit and coverage data
  ↓
Extract implementation file paths
  ↓
FOR EACH implementation file:
  - Run fspec research --tool=ast --operation=list-functions --file={file}
  - Run fspec research --tool=ast --operation=list-classes --file={file}
  - Run fspec research --tool=ast --operation=list-imports --file={file}
  - Run fspec research --tool=ast --operation=list-exports --file={file}
  ↓
Aggregate results:
  - Count functions per file
  - Count classes per file
  - Count imports per file
  - List function/class names
  - Identify similar names (exact match only)
  ↓
Build system-reminder with:
  - Structured data presentation
  - Suggested AST commands
  - Guidance questions
  ↓
Emit system-reminder
  ↓
AI reads system-reminder and makes decisions
```

### New Module: `src/utils/ast-data-gatherer.ts`

**Purpose:** Wrapper around `fspec research` for programmatic AST data gathering

**Exports:**
```typescript
export interface FileStructureData {
  filePath: string;
  functions: Array<{ name: string; line: number }>;
  classes: Array<{ name: string; line: number; methodCount?: number }>;
  imports: Array<{ line: number; source?: string }>;
  exports: Array<{ line: number; type: 'default' | 'named' }>;
  linesOfCode: number;
}

export async function gatherFileStructureData(
  filePath: string
): Promise<FileStructureData>;

export async function gatherAllFileData(
  files: string[]
): Promise<FileStructureData[]>;

export interface SimilarNamePattern {
  name: string;
  locations: Array<{ file: string; line: number }>;
}

export function findSimilarNames(
  data: FileStructureData[]
): {
  functions: SimilarNamePattern[];
  classes: SimilarNamePattern[];
};
```

### Integration Point in `review.ts`

**Add after loading coverage data:**

```typescript
// EXISTING: Load work unit and coverage
const workUnit = await showWorkUnit(workUnitId);
const coverageData = await loadCoverageData(workUnit);
const implementationFiles = extractImplementationFiles(coverageData);

// NEW: Gather AST data
import { gatherAllFileData, findSimilarNames } from '../utils/ast-data-gatherer';

const fileData = await gatherAllFileData(implementationFiles);
const similarNames = findSimilarNames(fileData);

// NEW: Build system-reminder
const astDataReminder = buildASTDataReminder(fileData, similarNames, workUnitId);

// EXISTING: Continue with rest of review...
```

### Module Responsibilities

**`ast-data-gatherer.ts`:**
- ✅ Execute `fspec research --tool=ast` commands
- ✅ Parse JSON results
- ✅ Count totals
- ✅ Extract names and locations
- ✅ Find exact name matches across files
- ❌ NO similarity calculations
- ❌ NO semantic analysis
- ❌ NO threshold checking

**`review.ts`:**
- ✅ Call `gatherAllFileData()`
- ✅ Format data into system-reminder
- ✅ Emit system-reminder
- ❌ NO interpretation of data
- ❌ NO violation detection
- ❌ NO refactoring recommendations

---

## Implementation Plan

### Phase 1: Infrastructure (Week 1)

**Goal:** Create data gathering module

**Tasks:**
1. Create `src/utils/ast-data-gatherer.ts`
   - Wrapper functions around `fspec research --tool=ast`
   - Type-safe interfaces for data structures
   - Error handling

2. Add unit tests
   - Test AST command execution
   - Test JSON parsing
   - Test data aggregation

**Success Criteria:**
- Can gather function/class/import/export data from files
- All data is structural (no semantic analysis)
- Tests pass with >90% coverage

### Phase 2: System-Reminder Builder (Week 2)

**Goal:** Create system-reminder formatting module

**Tasks:**
1. Create `src/utils/ast-reminder-builder.ts`
   - Format file structure data
   - Generate suggested AST commands
   - Build guidance questions

2. Add templates for:
   - Data presentation
   - Pattern observation
   - Command suggestions
   - Guidance questions

**Success Criteria:**
- System-reminders are clear and structured
- No judgments or violations mentioned
- Suggested commands are correct and runnable

### Phase 3: Integration (Week 3)

**Goal:** Integrate with `fspec review` command

**Tasks:**
1. Modify `src/commands/review.ts`
   - Extract implementation files from coverage
   - Call data gathering functions
   - Emit system-reminder with AST data

2. Add configuration
   - Enable/disable AST data gathering
   - Control verbosity level

**Success Criteria:**
- `fspec review <work-unit-id>` includes AST data in system-reminder
- No semantic analysis performed
- Performance impact <2 seconds

### Phase 4: Testing (Week 4)

**Goal:** Comprehensive testing

**Tasks:**
1. Integration tests
   - Test full review flow with AST data
   - Test with various file types
   - Test with missing/invalid files

2. Performance testing
   - Benchmark with 50+ files
   - Optimize if needed

**Success Criteria:**
- All tests pass
- Performance acceptable (<2 seconds overhead)
- Ready for production

---

## Code Examples

### Example 1: Data Gatherer Module

```typescript
// src/utils/ast-data-gatherer.ts
import { execSync } from 'child_process';

export async function gatherFileStructureData(
  filePath: string
): Promise<FileStructureData> {
  // Execute AST commands in parallel
  const [functionsResult, classesResult, importsResult, exportsResult] =
    await Promise.all([
      execASTCommand('list-functions', filePath),
      execASTCommand('list-classes', filePath),
      execASTCommand('list-imports', filePath),
      execASTCommand('list-exports', filePath)
    ]);

  // Parse results
  const functions = JSON.parse(functionsResult).matches.map(m => ({
    name: m.name,
    line: m.line
  }));

  const classes = JSON.parse(classesResult).matches.map(m => ({
    name: m.name,
    line: m.line
  }));

  const imports = JSON.parse(importsResult).matches.map(m => ({
    line: m.line,
    source: m.imports?.[0]?.path
  }));

  const exports = JSON.parse(exportsResult).matches.map(m => ({
    line: m.line,
    type: m.exportType || 'named'
  }));

  return {
    filePath,
    functions,
    classes,
    imports,
    exports,
    linesOfCode: countLines(filePath)
  };
}

function execASTCommand(operation: string, filePath: string): string {
  const command = `fspec research --tool=ast --operation=${operation} --file=${filePath}`;
  return execSync(command, { encoding: 'utf-8' });
}

export function findSimilarNames(
  data: FileStructureData[]
): {
  functions: SimilarNamePattern[];
  classes: SimilarNamePattern[];
} {
  const functionMap = new Map<string, Array<{ file: string; line: number }>>();
  const classMap = new Map<string, Array<{ file: string; line: number }>>();

  // Group by exact name match
  for (const fileData of data) {
    for (const func of fileData.functions) {
      if (!functionMap.has(func.name)) {
        functionMap.set(func.name, []);
      }
      functionMap.get(func.name)!.push({
        file: fileData.filePath,
        line: func.line
      });
    }

    for (const cls of fileData.classes) {
      if (!classMap.has(cls.name)) {
        classMap.set(cls.name, []);
      }
      classMap.get(cls.name)!.push({
        file: fileData.filePath,
        line: cls.line
      });
    }
  }

  // Filter to only names appearing in multiple files
  const functions: SimilarNamePattern[] = [];
  for (const [name, locations] of functionMap) {
    if (locations.length > 1) {
      functions.push({ name, locations });
    }
  }

  const classes: SimilarNamePattern[] = [];
  for (const [name, locations] of classMap) {
    if (locations.length > 1) {
      classes.push({ name, locations });
    }
  }

  return { functions, classes };
}
```

### Example 2: System-Reminder Builder

```typescript
// src/utils/ast-reminder-builder.ts

export function buildASTDataReminder(
  fileData: FileStructureData[],
  similarNames: { functions: SimilarNamePattern[]; classes: SimilarNamePattern[] },
  workUnitId: string
): string {
  let reminder = '<system-reminder>\n';
  reminder += `CODE STRUCTURE DATA for ${workUnitId}\n\n`;
  reminder += `Implementation files analyzed: ${fileData.length}\n\n`;

  // Present data for each file
  for (const file of fileData) {
    reminder += `File: ${file.filePath} (${file.linesOfCode} lines)\n`;
    reminder += `  Functions: ${file.functions.length}\n`;

    if (file.functions.length > 0) {
      const topFunctions = file.functions.slice(0, 5);
      for (const func of topFunctions) {
        reminder += `    - ${func.name}() at line ${func.line}\n`;
      }
      if (file.functions.length > 5) {
        reminder += `    (${file.functions.length - 5} more functions...)\n`;
      }
    }

    reminder += `  Classes: ${file.classes.length}\n`;
    for (const cls of file.classes) {
      reminder += `    - ${cls.name}`;
      if (cls.methodCount) {
        reminder += ` (${cls.methodCount} public methods detected)`;
      }
      reminder += `\n`;
    }

    reminder += `  Imports: ${file.imports.length}\n`;
    reminder += `  Exports: ${file.exports.length}`;

    const defaultExports = file.exports.filter(e => e.type === 'default').length;
    const namedExports = file.exports.filter(e => e.type === 'named').length;
    reminder += ` (${defaultExports} default, ${namedExports} named)\n\n`;
  }

  // Report patterns (neutral observations)
  if (similarNames.functions.length > 0 || similarNames.classes.length > 0) {
    reminder += 'Patterns detected (NOT violations, just observations):\n';

    for (const pattern of similarNames.functions) {
      reminder += `  - Function name "${pattern.name}" appears in ${pattern.locations.length} files:\n`;
      for (const loc of pattern.locations) {
        reminder += `    * ${loc.file}:${loc.line}\n`;
      }
    }

    for (const pattern of similarNames.classes) {
      reminder += `  - Class name "${pattern.name}" appears in ${pattern.locations.length} files:\n`;
      for (const loc of pattern.locations) {
        reminder += `    * ${loc.file}:${loc.line}\n`;
      }
    }
    reminder += '\n';
  }

  // Suggest commands for deeper investigation
  reminder += 'To investigate further, run these AST commands:\n';
  for (const file of fileData.slice(0, 3)) {
    reminder += `  fspec research --tool=ast --operation=list-functions --file=${file.filePath}\n`;
  }
  if (fileData.length > 3) {
    reminder += `  (${fileData.length - 3} more files...)\n`;
  }
  reminder += '\n';

  // Guidance questions (NOT instructions)
  reminder += 'AI: Analyze this structural data and determine if refactoring is needed.\n';
  reminder += 'Consider:\n';
  reminder += '  - Are there functions with similar names that might have duplicate logic?\n';
  reminder += '  - Are there classes with many methods that might violate SRP?\n';
  reminder += '  - Are there many imports that might indicate tight coupling?\n';
  reminder += '  - Are there opportunities to extract shared utilities?\n\n';
  reminder += 'Use the suggested AST commands to gather more details as needed.\n';
  reminder += '</system-reminder>';

  return reminder;
}
```

### Example 3: Integration in review.ts

```typescript
// src/commands/review.ts (additions)

import { gatherAllFileData, findSimilarNames } from '../utils/ast-data-gatherer';
import { buildASTDataReminder } from '../utils/ast-reminder-builder';

export async function review(
  workUnitId: string,
  options?: ReviewOptions
): Promise<ReviewResult> {
  // ... existing code ...

  // EXISTING: Load coverage and extract implementation files
  const coverageData = await loadCoverageData(workUnit);
  const implementationFiles = extractImplementationFilesFromCoverage(
    workUnit,
    coverageData
  );

  // NEW: Gather AST data (if enabled)
  let astDataReminder: string | null = null;
  if (options?.enableASTAnalysis !== false && implementationFiles.length > 0) {
    try {
      const fileData = await gatherAllFileData(implementationFiles);
      const similarNames = findSimilarNames(fileData);
      astDataReminder = buildASTDataReminder(fileData, similarNames, workUnitId);
    } catch (error) {
      // Log error but don't fail review
      console.error('AST data gathering failed:', error);
    }
  }

  // ... existing output generation ...

  // NEW: Append AST data reminder to output
  if (astDataReminder) {
    output += '\n' + astDataReminder + '\n';
  }

  return { output };
}

function extractImplementationFilesFromCoverage(
  workUnit: WorkUnit,
  coverageData: CoverageData[]
): string[] {
  const files = new Set<string>();

  for (const coverage of coverageData) {
    for (const scenario of coverage.scenarios) {
      for (const step of scenario.steps) {
        if (step.implementedBy) {
          for (const impl of step.implementedBy) {
            files.add(impl.filePath);
          }
        }
      }
    }
  }

  return Array.from(files);
}
```

---

## Testing Strategy

### Unit Tests

**Test file structure:**
```
src/utils/__tests__/
├── ast-data-gatherer.test.ts
└── ast-reminder-builder.test.ts
```

**Test cases for `ast-data-gatherer.ts`:**
- ✅ Execute AST commands successfully
- ✅ Parse JSON results correctly
- ✅ Handle missing/invalid files gracefully
- ✅ Find exact name matches across files
- ✅ Return empty arrays when no matches found
- ✅ Handle AST command errors

**Test cases for `ast-reminder-builder.ts`:**
- ✅ Format file data correctly
- ✅ Include all required sections
- ✅ Suggest correct AST commands
- ✅ Use neutral language (no judgments)
- ✅ Handle empty data arrays

### Integration Tests

**Test file:**
```
src/commands/__tests__/review-ast-data.test.ts
```

**Test scenarios:**
- ✅ Full review flow with AST data gathering
- ✅ Review with AST analysis disabled
- ✅ Review with no implementation files
- ✅ Review with AST command failures (graceful degradation)
- ✅ Review with various file types (TS, JS, Python)

### Verification Criteria

1. **No semantic analysis** - Tests must verify NO similarity calculations, NO violation detection
2. **Data only** - Tests must verify only structural data is gathered
3. **Neutral language** - Tests must verify system-reminders use neutral observations, NOT judgments
4. **Performance** - Tests must verify <2 second overhead for 50 files

---

## Conclusion

This research document defines a clear strategy for enhancing `fspec review` with AST-based structural data gathering **without semantic analysis**.

### Key Principles

1. ✅ **fspec gathers data** - Use AST tools to extract structure
2. ✅ **fspec suggests commands** - Tell AI what commands to run
3. ✅ **fspec asks questions** - Guide AI thinking with prompts
4. ❌ **fspec does NOT analyze** - No semantic judgments
5. ❌ **fspec does NOT detect** - No violation detection
6. ❌ **fspec does NOT recommend** - No refactoring decisions

### Implementation Summary

- **New modules**: `ast-data-gatherer.ts`, `ast-reminder-builder.ts`
- **Integration point**: `review.ts` after loading coverage data
- **Output**: System-reminder with structural data and guidance questions
- **Performance target**: <2 seconds overhead

### Next Steps

1. ✅ Review this research document with stakeholders
2. ✅ Update work unit REVIEW-001 with refined requirements
3. ✅ Begin Phase 1 implementation (data gathering module)
4. ✅ Iterate based on feedback

---

**Document Version:** 2.0 (Data Gathering Focus)
**Last Updated:** 2025-01-12
**Status:** Draft for Review

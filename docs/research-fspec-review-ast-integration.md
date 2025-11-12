# Research: AST-Based DRY and SOLID Principles Integration for fspec review

**Date:** 2025-01-12
**Author:** Claude (AI Research Assistant)
**Purpose:** Comprehensive analysis and implementation strategy for enhancing `fspec review` command with AST-based code quality checks

---

## Executive Summary

This research document outlines a comprehensive strategy for refactoring the `fspec review` command to include:

1. **AST-Based Code Analysis** - Using `fspec research --tool=ast` to gather structural data about code
2. **AI Guidance via System-Reminders** - Providing intelligent prompts that guide AI agents to check DRY/SOLID principles
3. **Integration with `fspec research`** - Leveraging the existing AST tool for deep code analysis
4. **Link Validation Data** - Gathering import/export data for AI to analyze

**CRITICAL DESIGN PRINCIPLE: NO SEMANTIC CODE ANALYSIS IN FSPEC**

fspec MUST NOT implement semantic code analysis or make judgments about code quality. Instead:

- ✅ **fspec's role**: Provide AST tools to gather structural data (function counts, class sizes, import graphs)
- ✅ **fspec's role**: Emit system-reminders with data and guidance for AI agents
- ❌ **NOT fspec's role**: Determine if code violates DRY/SOLID principles
- ❌ **NOT fspec's role**: Implement heuristics or algorithms to detect code smells
- ❌ **NOT fspec's role**: Calculate similarity scores or make quality judgments

**The AI agent calling fspec makes all decisions.** fspec only provides tools and contextual guidance.

The proposed solution uses tree-sitter's deterministic AST traversal to analyze code structure without executing it, providing fast, reliable, and language-agnostic structural data that AI agents can use to perform their own analysis.

---

## Table of Contents

1. [Current State Analysis](#current-state-analysis)
2. [AST Tool Capabilities](#ast-tool-capabilities)
3. [Data Gathering Strategy](#data-gathering-strategy)
4. [System-Reminder Design](#system-reminder-design)
5. [Integration Architecture](#integration-architecture)
6. [Implementation Plan](#implementation-plan)
7. [Code Examples](#code-examples)
8. [Testing Strategy](#testing-strategy)
9. [Performance Considerations](#performance-considerations)
10. [Risks and Mitigations](#risks-and-mitigations)
11. [Future Enhancements](#future-enhancements)

---

## Current State Analysis

### Existing `fspec review` Command

**Location:** `src/commands/review.ts` (587 lines)

**Current Capabilities:**
1. ✅ ACDD compliance checking (Example Mapping, feature files, test coverage)
2. ✅ TypeScript violations detection (any types, require(), file extensions)
3. ✅ Gherkin syntax validation
4. ✅ Test coverage analysis via `.coverage` files
5. ✅ AI-driven deep code analysis (system-reminder integration)

**Current Limitations:**
1. ❌ No structural data gathering to guide AI on DRY analysis
2. ❌ No structural data gathering to guide AI on SOLID analysis
3. ❌ No AST-based structural code data beyond basic TypeScript violations
4. ❌ No cross-file dependency graph data for AI to analyze
5. ❌ Limited guidance via system-reminders (AI must manually discover analysis opportunities)

### Existing `fspec research` Command

**Location:** `src/commands/research.ts` (295 lines)

**Current Capabilities:**
1. ✅ Plugin-based tool system
2. ✅ AST tool with tree-sitter support for 12+ languages
3. ✅ Predefined operations (list-functions, find-class, etc.)
4. ✅ Custom query support via .scm files
5. ✅ Parametric filtering (name, pattern, min-params)

**Current Usage Pattern:**
- Manual invocation during Example Mapping
- Standalone tool execution (not integrated with review)
- Primarily used for research, not quality checking

---

## AST Tool Capabilities

### Supported Languages

The AST tool (`src/research-tools/ast.ts`) supports:

- JavaScript (.js, .jsx)
- TypeScript (.ts, .tsx)
- Python (.py)
- Go (.go)
- Rust (.rs)
- Java (.java)
- Ruby (.rb)
- C# (.cs)
- PHP (.php)
- C++ (.cpp, .hpp, etc.)
- Bash (.sh)
- JSON (.json)

### Available Operations

#### 1. `list-functions` / `find-functions`
**Purpose:** Find all function declarations, expressions, arrow functions, and methods

**Detection Capabilities:**
```typescript
// Detects:
- function_declaration
- function_expression
- arrow_function
- method_definition
- generator_function_declaration
```

**Output Fields:**
- `type` - Node type
- `name` - Function/method name
- `line` - Line number
- `column` - Column position
- `text` - Full function code
- `paramCount` - Parameter count (when using --min-params)

**Use Case for DRY:**
- Compare function signatures across files
- Detect duplicate function implementations
- Find functions with similar parameter patterns

#### 2. `list-classes` / `find-class`
**Purpose:** Find all class declarations or specific class by name

**Detection Capabilities:**
```typescript
// Detects:
- class_declaration
- class (generic)
```

**Output Fields:**
- `type` - Node type
- `name` - Class name
- `line` - Line number
- `body` - Class body (all methods and properties)
- `code` - Full class code

**Use Case for SOLID:**
- Analyze class sizes (Single Responsibility)
- Count methods per class
- Detect "God classes" (too many responsibilities)

#### 3. `list-imports`
**Purpose:** Find all import statements

**Detection Capabilities:**
```typescript
// Detects:
- import_statement
- import_declaration
```

**Output Fields:**
- `symbols` - Imported symbols
- `path` - Import path
- `line` - Line number

**Use Case for Link Validation:**
- Validate all imports resolve correctly
- Detect unused imports
- Find circular dependencies

#### 4. `list-exports` / `find-exports`
**Purpose:** Find export statements (default and named)

**Detection Capabilities:**
```typescript
// Detects:
- export_statement
- export_declaration
```

**Output Fields:**
- `exportType` - "default" or "named"
- `identifier` - Exported identifier
- `line` - Line number
- `code` - Full export statement

**Use Case for Link Validation:**
- Ensure exports are properly linked to imports
- Detect orphaned exports (never imported)
- Validate module boundaries

#### 5. `find-async-functions`
**Purpose:** Find asynchronous function declarations

**Use Case for Code Quality:**
- Analyze async/await patterns
- Detect missing error handling in async code

#### 6. `find-identifiers`
**Purpose:** Find identifiers matching a pattern (regex)

**Use Case for DRY:**
- Find repeated constant names
- Detect naming pattern violations
- Search for specific variable usage

### Tree-Sitter Query Architecture

The AST tool uses **deterministic tree-sitter operations** (not NL parsing):

**Three-Layer Architecture:**

1. **AST Tool Layer** (`ast.ts`)
   - Entry point
   - Language detection
   - Argument parsing

2. **Query Executor Layer** (`query-executor.ts`)
   - Maps operations to queries
   - Applies parametric predicates
   - Filters results

3. **AST Parser Layer** (`ast-parser.ts`)
   - Direct tree traversal
   - Language-agnostic operations
   - Position tracking

**Manual Tree Traversal Pattern:**
```typescript
function findFunctions(node: Parser.SyntaxNode, matches: QueryMatch[]) {
  // Check node type
  if (node.type === 'function_declaration' ||
      node.type === 'method_definition') {
    // Extract fields using childForFieldName()
    const nameNode = node.childForFieldName('name');
    matches.push({
      type: node.type,
      name: nameNode?.text,
      line: node.startPosition.row + 1,
      text: node.text
    });
  }

  // Recurse through children
  for (const child of node.children) {
    findFunctions(child, matches);
  }
}
```

**Key Advantages:**
- ✅ Deterministic (same input = same output)
- ✅ Fast (no AI inference)
- ✅ Language-agnostic (works for 12+ languages)
- ✅ Structural analysis (no code execution)
- ✅ Position-aware (line/column tracking)

---

## Data Gathering Strategy

**CRITICAL PRINCIPLE**: fspec does NOT detect violations or make judgments. It only gathers structural data and provides it to AI agents via system-reminders.

### Overview

The `fspec review` command will:

1. **Gather structural data** using `fspec research --tool=ast`
2. **Present data in system-reminders** for AI agents to analyze
3. **Suggest AST commands** AI can run for deeper investigation
4. **NOT determine** if code violates DRY/SOLID principles
5. **NOT calculate** similarity scores or quality metrics

### Data Types to Gather

#### 1. Function Data (for DRY analysis)

#### 1. Function Signature Duplication

**Strategy:** Compare function signatures across all implementation files

**AST Operations:**
```bash
# For each implementation file
fspec research --tool=ast --operation=list-functions --file=src/utils/parser.ts

# Compare results:
# - Same parameter count
# - Similar parameter types
# - Similar function names
```

**Detection Algorithm:**
```typescript
interface FunctionSignature {
  name: string;
  paramCount: number;
  filePath: string;
  line: number;
  text: string;
}

async function detectDuplicateFunctions(
  files: string[]
): Promise<DuplicateReport[]> {
  const allFunctions: FunctionSignature[] = [];

  // Extract functions from all files
  for (const file of files) {
    const result = await executeASTTool([
      '--operation=list-functions',
      `--file=${file}`
    ]);
    const functions = JSON.parse(result).matches;
    allFunctions.push(...functions.map(f => ({ ...f, filePath: file })));
  }

  // Group by signature similarity
  const groups = groupBySimilarity(allFunctions);

  // Filter groups with 2+ functions (potential duplicates)
  return groups.filter(g => g.length >= 2).map(g => ({
    type: 'duplicate_function',
    severity: 'warning',
    functions: g,
    suggestion: 'Extract common logic to shared utility function'
  }));
}
```

**Similarity Metrics:**
1. **Exact Match** - Identical function bodies (ignoring whitespace)
2. **Fuzzy Match** - Similar structure (same AST node types)
3. **Parameter Match** - Same parameter count and types
4. **Name Match** - Similar function names (Levenshtein distance < 3)

#### 2. Code Block Duplication

**Strategy:** Compare code blocks within functions/methods

**AST Operations:**
```bash
# Extract function bodies
fspec research --tool=ast --operation=list-functions --file=src/commands/review.ts

# Analyze function bodies for repeated patterns
```

**Detection Algorithm:**
```typescript
async function detectDuplicateCodeBlocks(
  functions: FunctionSignature[]
): Promise<DuplicateReport[]> {
  const duplicates: DuplicateReport[] = [];

  for (let i = 0; i < functions.length; i++) {
    for (let j = i + 1; j < functions.length; j++) {
      const similarity = calculateCodeSimilarity(
        functions[i].text,
        functions[j].text
      );

      if (similarity > 0.8) { // 80% similar
        duplicates.push({
          type: 'duplicate_code_block',
          severity: similarity > 0.95 ? 'error' : 'warning',
          locations: [
            { file: functions[i].filePath, line: functions[i].line },
            { file: functions[j].filePath, line: functions[j].line }
          ],
          similarityScore: similarity,
          suggestion: 'Refactor duplicate code into shared function'
        });
      }
    }
  }

  return duplicates;
}

function calculateCodeSimilarity(code1: string, code2: string): number {
  // Use normalized Levenshtein distance
  // Or AST structure comparison
  // Or token sequence comparison

  // Remove whitespace and normalize
  const normalized1 = normalizeCode(code1);
  const normalized2 = normalizeCode(code2);

  // Calculate Levenshtein distance
  const distance = levenshteinDistance(normalized1, normalized2);
  const maxLength = Math.max(normalized1.length, normalized2.length);

  return 1 - (distance / maxLength);
}
```

#### 3. Constant Duplication

**Strategy:** Find repeated constant values across files

**AST Operations:**
```bash
# Find all constant definitions
fspec research --tool=ast --operation=find-identifiers --pattern="^[A-Z][A-Z_]+" --file=src/constants.ts

# Compare constant values
```

**Detection Algorithm:**
```typescript
async function detectDuplicateConstants(
  files: string[]
): Promise<DuplicateReport[]> {
  const constants = new Map<string, ConstantDefinition[]>();

  for (const file of files) {
    // Find all CONSTANT_CASE identifiers
    const result = await executeASTTool([
      '--operation=find-identifiers',
      '--pattern=^[A-Z][A-Z_]+',
      `--file=${file}`
    ]);

    const matches = JSON.parse(result).matches;
    for (const match of matches) {
      if (!constants.has(match.name)) {
        constants.set(match.name, []);
      }
      constants.get(match.name)!.push({
        name: match.name,
        file,
        line: match.line,
        value: extractConstantValue(match.text)
      });
    }
  }

  // Find constants with same value but different names
  return findRedundantConstants(constants);
}
```

#### 4. Implementation Recommendations

**For each duplicate detected:**

```typescript
interface DRYRecommendation {
  type: 'duplicate_function' | 'duplicate_code_block' | 'duplicate_constant';
  severity: 'error' | 'warning' | 'info';
  locations: Array<{ file: string; line: number }>;
  similarityScore?: number;
  suggestion: string;
  refactoringSteps: string[];
}

// Example recommendation:
{
  type: 'duplicate_function',
  severity: 'warning',
  locations: [
    { file: 'src/utils/parser-a.ts', line: 42 },
    { file: 'src/utils/parser-b.ts', line: 67 }
  ],
  similarityScore: 0.92,
  suggestion: 'Extract common logic to shared utility function',
  refactoringSteps: [
    '1. Create new file: src/utils/shared-parser.ts',
    '2. Extract common function: parseConfiguration()',
    '3. Import shared function in both files',
    '4. Replace duplicate implementations with shared function call'
  ]
}
```

---

## SOLID Principles Detection Strategy

### SOLID Principles Overview

1. **S**ingle Responsibility Principle (SRP)
2. **O**pen/Closed Principle (OCP)
3. **L**iskov Substitution Principle (LSP)
4. **I**nterface Segregation Principle (ISP)
5. **D**ependency Inversion Principle (DIP)

### 1. Single Responsibility Principle (SRP)

**Principle:** A class should have only one reason to change.

**Detection Strategy:**

**AST Operations:**
```bash
# Find all classes
fspec research --tool=ast --operation=list-classes --file=src/commands/review.ts

# For each class, analyze:
# - Number of public methods
# - Number of private methods
# - Method complexity (lines of code)
# - Number of dependencies (imports)
```

**Detection Metrics:**
```typescript
interface SRPMetrics {
  className: string;
  filePath: string;
  publicMethodCount: number;
  privateMethodCount: number;
  totalMethodCount: number;
  averageMethodLength: number;
  dependencyCount: number;
  responsibilityScore: number; // 0-100 (higher = more violations)
}

async function analyzeSRP(file: string): Promise<SRPViolation[]> {
  // Get all classes
  const classesResult = await executeASTTool([
    '--operation=list-classes',
    `--file=${file}`
  ]);
  const classes = JSON.parse(classesResult).matches;

  const violations: SRPViolation[] = [];

  for (const classNode of classes) {
    // Parse class body to count methods
    const methods = parseMethodsFromClassBody(classNode.body);

    // Get imports for this file
    const importsResult = await executeASTTool([
      '--operation=list-imports',
      `--file=${file}`
    ]);
    const imports = JSON.parse(importsResult).matches;

    const metrics: SRPMetrics = {
      className: classNode.name,
      filePath: file,
      publicMethodCount: countPublicMethods(methods),
      privateMethodCount: countPrivateMethods(methods),
      totalMethodCount: methods.length,
      averageMethodLength: calculateAverageLength(methods),
      dependencyCount: imports.length,
      responsibilityScore: calculateResponsibilityScore(methods, imports)
    };

    // Flag if:
    // - More than 10 public methods (too many responsibilities)
    // - More than 20 dependencies (tight coupling)
    // - Average method length > 50 lines (complex methods)
    if (metrics.publicMethodCount > 10 ||
        metrics.dependencyCount > 20 ||
        metrics.averageMethodLength > 50) {
      violations.push({
        principle: 'Single Responsibility',
        severity: 'warning',
        class: metrics.className,
        file: metrics.filePath,
        metrics,
        suggestion: 'Consider splitting this class into smaller, focused classes',
        refactoringSteps: generateSRPRefactoringSteps(metrics)
      });
    }
  }

  return violations;
}
```

**Thresholds (Configurable):**
- ⚠️ Warning: > 10 public methods
- 🔴 Error: > 15 public methods
- ⚠️ Warning: > 20 dependencies
- ⚠️ Warning: Average method length > 50 lines

### 2. Open/Closed Principle (OCP)

**Principle:** Software entities should be open for extension but closed for modification.

**Detection Strategy:**

**AST Operations:**
```bash
# Find classes and check for:
# - Use of inheritance (extends keyword)
# - Use of interfaces (implements keyword)
# - Abstract classes/methods
```

**Detection Algorithm:**
```typescript
async function analyzeOCP(file: string): Promise<OCPViolation[]> {
  const violations: OCPViolation[] = [];

  // Parse class declarations
  const classesResult = await executeASTTool([
    '--operation=list-classes',
    `--file=${file}`
  ]);
  const classes = JSON.parse(classesResult).matches;

  for (const classNode of classes) {
    const analysis = {
      hasAbstractMethods: checkForAbstractMethods(classNode.body),
      hasInterfaces: checkForImplements(classNode.text),
      usesInheritance: checkForExtends(classNode.text),
      hasSwitch: checkForSwitchStatements(classNode.body),
      hasTypeChecking: checkForTypeofInstanceof(classNode.body)
    };

    // Violations:
    // - Large switch/if-else chains (should use polymorphism)
    // - Direct type checking (should use interfaces)
    if (analysis.hasSwitch && !analysis.usesInheritance) {
      violations.push({
        principle: 'Open/Closed',
        severity: 'warning',
        class: classNode.name,
        file,
        issue: 'Switch statement without polymorphism',
        suggestion: 'Replace switch with strategy pattern or polymorphic classes'
      });
    }

    if (analysis.hasTypeChecking && !analysis.hasInterfaces) {
      violations.push({
        principle: 'Open/Closed',
        severity: 'info',
        class: classNode.name,
        file,
        issue: 'Type checking without interfaces',
        suggestion: 'Define interfaces and use polymorphism instead of type checks'
      });
    }
  }

  return violations;
}
```

### 3. Liskov Substitution Principle (LSP)

**Principle:** Objects of a superclass should be replaceable with objects of a subclass without breaking the application.

**Detection Strategy:**

**AST Operations:**
```bash
# Find class hierarchies (extends relationships)
# Analyze method signatures in parent vs child classes
```

**Detection Algorithm:**
```typescript
async function analyzeLSP(file: string): Promise<LSPViolation[]> {
  const violations: LSPViolation[] = [];

  // Find classes with inheritance
  const classesResult = await executeASTTool([
    '--operation=list-classes',
    `--file=${file}`
  ]);
  const classes = JSON.parse(classesResult).matches;

  for (const classNode of classes) {
    const parentClass = extractParentClass(classNode.text);
    if (parentClass) {
      // Find parent class definition
      const parentFile = await findClassDefinition(parentClass);
      if (parentFile) {
        // Compare method signatures
        const childMethods = extractMethods(classNode.body);
        const parentMethods = await extractMethodsFromFile(parentFile, parentClass);

        // Check for LSP violations:
        // - Child method has different signature than parent
        // - Child method throws new exceptions not in parent
        // - Child method weakens preconditions
        const signatureMismatches = compareMethodSignatures(
          parentMethods,
          childMethods
        );

        if (signatureMismatches.length > 0) {
          violations.push({
            principle: 'Liskov Substitution',
            severity: 'error',
            class: classNode.name,
            parentClass,
            file,
            mismatches: signatureMismatches,
            suggestion: 'Ensure child class methods match parent signatures'
          });
        }
      }
    }
  }

  return violations;
}
```

### 4. Interface Segregation Principle (ISP)

**Principle:** No client should be forced to depend on methods it does not use.

**Detection Strategy:**

**AST Operations:**
```bash
# Find interface definitions
# Count methods per interface
# Analyze interface usage across classes
```

**Detection Algorithm:**
```typescript
async function analyzeISP(file: string): Promise<ISPViolation[]> {
  const violations: ISPViolation[] = [];

  // TypeScript: Look for interface declarations
  // JavaScript: Look for large classes (acting as implicit interfaces)

  const interfacePattern = /interface\s+(\w+)\s*{([^}]+)}/g;
  const fileContent = await fs.readFile(file, 'utf-8');
  const matches = [...fileContent.matchAll(interfacePattern)];

  for (const match of matches) {
    const interfaceName = match[1];
    const interfaceBody = match[2];

    // Count methods in interface
    const methodCount = countMethods(interfaceBody);

    // Flag if:
    // - Interface has > 10 methods (too broad)
    if (methodCount > 10) {
      violations.push({
        principle: 'Interface Segregation',
        severity: 'warning',
        interface: interfaceName,
        file,
        methodCount,
        suggestion: 'Split large interface into smaller, focused interfaces',
        refactoringSteps: [
          '1. Identify cohesive groups of methods',
          '2. Create separate interfaces for each group',
          '3. Use interface composition to combine if needed'
        ]
      });
    }
  }

  return violations;
}
```

### 5. Dependency Inversion Principle (DIP)

**Principle:** High-level modules should not depend on low-level modules. Both should depend on abstractions.

**Detection Strategy:**

**AST Operations:**
```bash
# Analyze import statements
# Check dependency directions
# Find concrete class dependencies (should depend on abstractions)
```

**Detection Algorithm:**
```typescript
async function analyzeDIP(file: string): Promise<DIPViolation[]> {
  const violations: DIPViolation[] = [];

  // Get all imports
  const importsResult = await executeASTTool([
    '--operation=list-imports',
    `--file=${file}`
  ]);
  const imports = JSON.parse(importsResult).matches;

  for (const importNode of imports) {
    const importPath = importNode.imports[0].path;

    // Check if importing concrete implementations directly
    // vs importing from interface/types/abstract files

    // Heuristic: Imports from 'types', 'interfaces', 'abstract' are good
    // Imports from 'implementations', concrete class files are bad

    const isAbstraction =
      importPath.includes('/types/') ||
      importPath.includes('/interfaces/') ||
      importPath.includes('/abstract/') ||
      importPath.endsWith('Types') ||
      importPath.endsWith('Interface');

    const isConcreteImplementation =
      importPath.includes('/implementations/') ||
      importPath.includes('/concrete/') ||
      (!isAbstraction && importPath.includes('/services/'));

    if (isConcreteImplementation) {
      violations.push({
        principle: 'Dependency Inversion',
        severity: 'info',
        file,
        importPath,
        line: importNode.lineNumber,
        suggestion: 'Consider depending on abstractions (interfaces/types) instead of concrete implementations',
        refactoringSteps: [
          '1. Define an interface for the imported module',
          '2. Update import to use interface type',
          '3. Use dependency injection to provide implementation'
        ]
      });
    }
  }

  return violations;
}
```

### SOLID Analysis Summary Output

```typescript
interface SOLIDAnalysisResult {
  file: string;
  srpViolations: SRPViolation[];
  ocpViolations: OCPViolation[];
  lspViolations: LSPViolation[];
  ispViolations: ISPViolation[];
  dipViolations: DIPViolation[];
  overallScore: number; // 0-100
  grade: 'A' | 'B' | 'C' | 'D' | 'F';
}

async function analyzeSOLID(file: string): Promise<SOLIDAnalysisResult> {
  const [srp, ocp, lsp, isp, dip] = await Promise.all([
    analyzeSRP(file),
    analyzeOCP(file),
    analyzeLSP(file),
    analyzeISP(file),
    analyzeDIP(file)
  ]);

  const totalViolations =
    srp.length + ocp.length + lsp.length + isp.length + dip.length;

  const overallScore = Math.max(0, 100 - (totalViolations * 10));
  const grade = scoreToGrade(overallScore);

  return {
    file,
    srpViolations: srp,
    ocpViolations: ocp,
    lspViolations: lsp,
    ispViolations: isp,
    dipViolations: dip,
    overallScore,
    grade
  };
}
```

---

## Link Validation Strategy

### Purpose

Ensure all code is properly "linked up" through:
1. Valid import/export relationships
2. No orphaned exports (exported but never imported)
3. No missing imports (used but not imported)
4. No circular dependencies

### AST-Based Link Validation

#### 1. Import/Export Graph Construction

**Strategy:** Build a directed graph of all imports/exports

**AST Operations:**
```bash
# For each file:
fspec research --tool=ast --operation=list-imports --file=<file>
fspec research --tool=ast --operation=list-exports --file=<file>
```

**Algorithm:**
```typescript
interface DependencyGraph {
  nodes: Map<string, FileNode>;
  edges: Array<{ from: string; to: string; symbols: string[] }>;
}

interface FileNode {
  filePath: string;
  imports: ImportInfo[];
  exports: ExportInfo[];
}

async function buildDependencyGraph(files: string[]): Promise<DependencyGraph> {
  const graph: DependencyGraph = {
    nodes: new Map(),
    edges: []
  };

  for (const file of files) {
    // Get imports
    const importsResult = await executeASTTool([
      '--operation=list-imports',
      `--file=${file}`
    ]);
    const imports = JSON.parse(importsResult).matches;

    // Get exports
    const exportsResult = await executeASTTool([
      '--operation=list-exports',
      `--file=${file}`
    ]);
    const exports = JSON.parse(exportsResult).matches;

    // Add node
    graph.nodes.set(file, {
      filePath: file,
      imports: imports.map(i => i.imports[0]),
      exports: exports.map(e => ({
        type: e.exportType,
        identifier: e.identifier,
        line: e.line
      }))
    });

    // Add edges
    for (const importInfo of imports) {
      const importPath = resolveImportPath(file, importInfo.imports[0].path);
      graph.edges.push({
        from: file,
        to: importPath,
        symbols: importInfo.imports[0].symbols
      });
    }
  }

  return graph;
}
```

#### 2. Orphaned Export Detection

**Strategy:** Find exports that are never imported

```typescript
async function findOrphanedExports(
  graph: DependencyGraph
): Promise<OrphanedExport[]> {
  const orphaned: OrphanedExport[] = [];

  for (const [filePath, node] of graph.nodes) {
    for (const exp of node.exports) {
      // Check if this export is imported anywhere
      const isImported = graph.edges.some(edge =>
        edge.to === filePath &&
        edge.symbols.includes(exp.identifier)
      );

      if (!isImported) {
        orphaned.push({
          file: filePath,
          exportName: exp.identifier,
          exportType: exp.type,
          line: exp.line,
          suggestion: 'Remove unused export or add to public API'
        });
      }
    }
  }

  return orphaned;
}
```

#### 3. Missing Import Detection

**Strategy:** Find identifiers used but not imported

**AST Operations:**
```bash
# Find all identifier usage
fspec research --tool=ast --operation=find-identifiers --file=<file>

# Compare with imported symbols
fspec research --tool=ast --operation=list-imports --file=<file>
```

**Algorithm:**
```typescript
async function findMissingImports(file: string): Promise<MissingImport[]> {
  // Get all identifiers used in file
  const identifiersResult = await executeASTTool([
    '--operation=find-identifiers',
    `--file=${file}`
  ]);
  const allIdentifiers = JSON.parse(identifiersResult).matches;

  // Get imported symbols
  const importsResult = await executeASTTool([
    '--operation=list-imports',
    `--file=${file}`
  ]);
  const imports = JSON.parse(importsResult).matches;
  const importedSymbols = new Set(
    imports.flatMap(i => i.imports[0].symbols)
  );

  // Get locally defined symbols (functions, classes, etc.)
  const localSymbols = await getLocallyDefinedSymbols(file);

  // Find identifiers that are:
  // - Not imported
  // - Not locally defined
  // - Not built-in (e.g., console, process)
  const missing: MissingImport[] = [];

  for (const identifier of allIdentifiers) {
    if (!importedSymbols.has(identifier.name) &&
        !localSymbols.has(identifier.name) &&
        !isBuiltIn(identifier.name)) {
      missing.push({
        file,
        identifier: identifier.name,
        line: identifier.line,
        suggestion: `Add import for ${identifier.name}`
      });
    }
  }

  return missing;
}
```

#### 4. Circular Dependency Detection

**Strategy:** Use graph cycle detection algorithm

```typescript
function detectCircularDependencies(
  graph: DependencyGraph
): CircularDependency[] {
  const cycles: CircularDependency[] = [];
  const visited = new Set<string>();
  const recursionStack = new Set<string>();

  function dfs(node: string, path: string[]): void {
    visited.add(node);
    recursionStack.add(node);
    path.push(node);

    // Get all outgoing edges from this node
    const outgoingEdges = graph.edges.filter(e => e.from === node);

    for (const edge of outgoingEdges) {
      if (!visited.has(edge.to)) {
        dfs(edge.to, [...path]);
      } else if (recursionStack.has(edge.to)) {
        // Found a cycle
        const cycleStartIndex = path.indexOf(edge.to);
        const cycle = path.slice(cycleStartIndex);
        cycle.push(edge.to); // Complete the cycle

        cycles.push({
          cycle,
          severity: 'error',
          suggestion: 'Refactor to remove circular dependency'
        });
      }
    }

    recursionStack.delete(node);
  }

  for (const [filePath] of graph.nodes) {
    if (!visited.has(filePath)) {
      dfs(filePath, []);
    }
  }

  return cycles;
}
```

---

## Integration Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     fspec review <work-unit-id>              │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Review Command (review.ts)                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ 1. Load work unit                                      │  │
│  │ 2. Get implementation files from coverage data        │  │
│  │ 3. Run EXISTING checks (ACDD, TypeScript, coverage)   │  │
│  │ 4. Run NEW checks:                                     │  │
│  │    ├─ DRY Analysis (via fspec research)               │  │
│  │    ├─ SOLID Analysis (via fspec research)             │  │
│  │    └─ Link Validation (via fspec research)            │  │
│  │ 5. Generate comprehensive report                      │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Code Quality Analyzer (NEW MODULE)              │
│                     src/utils/code-quality-analyzer.ts       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ analyzeDRY(files: string[])                           │  │
│  │ analyzeSOLID(files: string[])                         │  │
│  │ validateLinks(files: string[])                        │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              AST Executor Wrapper (NEW MODULE)               │
│                     src/utils/ast-executor.ts                │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ executeASTOperation(operation, file, params)          │  │
│  │ buildDependencyGraph(files)                           │  │
│  │ compareCodeSimilarity(code1, code2)                   │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Research Command (research.ts)                  │
│                  [EXISTING - NO CHANGES]                     │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Execute AST tool with specified operations            │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              AST Tool (ast.ts)                               │
│                  [EXISTING - NO CHANGES]                     │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Tree-sitter parsing                                    │  │
│  │ Query execution                                        │  │
│  │ Result formatting                                      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### New Modules to Create

#### 1. `src/utils/code-quality-analyzer.ts`

**Purpose:** High-level API for DRY, SOLID, and link validation

**Exports:**
```typescript
export interface CodeQualityAnalysisResult {
  dry: DRYAnalysisResult;
  solid: SOLIDAnalysisResult;
  links: LinkValidationResult;
  overallScore: number;
  grade: 'A' | 'B' | 'C' | 'D' | 'F';
}

export async function analyzeCodeQuality(
  files: string[]
): Promise<CodeQualityAnalysisResult>;

export async function analyzeDRY(
  files: string[]
): Promise<DRYAnalysisResult>;

export async function analyzeSOLID(
  files: string[]
): Promise<SOLIDAnalysisResult>;

export async function validateLinks(
  files: string[]
): Promise<LinkValidationResult>;
```

#### 2. `src/utils/ast-executor.ts`

**Purpose:** Wrapper around `fspec research --tool=ast` for programmatic use

**Exports:**
```typescript
export interface ASTExecutorOptions {
  operation: string;
  file: string;
  parameters?: Record<string, string>;
}

export async function executeASTOperation(
  options: ASTExecutorOptions
): Promise<QueryMatch[]>;

export async function buildDependencyGraph(
  files: string[]
): Promise<DependencyGraph>;

export function calculateCodeSimilarity(
  code1: string,
  code2: string
): number;
```

#### 3. `src/utils/dry-analyzer.ts`

**Purpose:** DRY principle detection algorithms

**Exports:**
```typescript
export async function detectDuplicateFunctions(
  files: string[]
): Promise<DuplicateReport[]>;

export async function detectDuplicateCodeBlocks(
  functions: FunctionSignature[]
): Promise<DuplicateReport[]>;

export async function detectDuplicateConstants(
  files: string[]
): Promise<DuplicateReport[]>;
```

#### 4. `src/utils/solid-analyzer.ts`

**Purpose:** SOLID principles detection algorithms

**Exports:**
```typescript
export async function analyzeSRP(file: string): Promise<SRPViolation[]>;
export async function analyzeOCP(file: string): Promise<OCPViolation[]>;
export async function analyzeLSP(file: string): Promise<LSPViolation[]>;
export async function analyzeISP(file: string): Promise<ISPViolation[]>;
export async function analyzeDIP(file: string): Promise<DIPViolation[]>;
```

#### 5. `src/utils/link-validator.ts`

**Purpose:** Import/export validation algorithms

**Exports:**
```typescript
export async function buildDependencyGraph(
  files: string[]
): Promise<DependencyGraph>;

export async function findOrphanedExports(
  graph: DependencyGraph
): Promise<OrphanedExport[]>;

export async function findMissingImports(
  file: string
): Promise<MissingImport[]>;

export function detectCircularDependencies(
  graph: DependencyGraph
): Promise<CircularDependency[]>;
```

### Integration Points in `review.ts`

**Modifications to `review()` function:**

```typescript
// EXISTING: review.ts (line ~80-120)
export async function review(
  workUnitId: string,
  options?: ReviewOptions
): Promise<ReviewResult> {
  // ... existing code ...

  // NEW: After loading coverage data, get implementation files
  const implementationFiles = extractImplementationFilesFromCoverage(
    workUnit,
    coverageData
  );

  // NEW: Run code quality analysis
  const codeQualityResult = await analyzeCodeQuality(implementationFiles);

  // ... existing code ...

  // NEW: Add code quality section to output
  outputSections.push(formatCodeQualitySection(codeQualityResult));

  // ... rest of existing code ...
}
```

**New function to add:**

```typescript
async function extractImplementationFilesFromCoverage(
  workUnit: WorkUnit,
  coverageData: CoverageData[]
): Promise<string[]> {
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

### Output Format Enhancement

**NEW Section in Review Output:**

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 CODE QUALITY ANALYSIS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Overall Grade: B (Score: 82/100)

┌─────────────────────────────────────────────────────────────┐
│ DRY (Don't Repeat Yourself) Analysis                        │
└─────────────────────────────────────────────────────────────┘

🟡 WARNING: Duplicate function detected
   Files: src/utils/parser-a.ts:42, src/utils/parser-b.ts:67
   Similarity: 92%
   Suggestion: Extract common logic to shared utility function

   Refactoring Steps:
   1. Create new file: src/utils/shared-parser.ts
   2. Extract common function: parseConfiguration()
   3. Import shared function in both files
   4. Replace duplicate implementations with shared function call

🟡 WARNING: Duplicate code block detected
   Files: src/commands/review.ts:156, src/commands/validate.ts:234
   Similarity: 85%
   Suggestion: Refactor duplicate code into shared function

Total DRY Issues: 2 (0 errors, 2 warnings, 0 info)

┌─────────────────────────────────────────────────────────────┐
│ SOLID Principles Analysis                                   │
└─────────────────────────────────────────────────────────────┘

🟡 Single Responsibility Principle (SRP)
   Class: ReviewCommand (src/commands/review.ts)
   Issues:
   - 12 public methods (threshold: 10)
   - 23 dependencies (threshold: 20)
   Suggestion: Consider splitting this class into smaller, focused classes

ℹ️  Dependency Inversion Principle (DIP)
   File: src/commands/review.ts:42
   Import: '../services/concrete-parser'
   Suggestion: Depend on abstractions (interfaces) instead of concrete implementations

Total SOLID Issues: 2 (0 errors, 1 warning, 1 info)

┌─────────────────────────────────────────────────────────────┐
│ Link Validation                                             │
└─────────────────────────────────────────────────────────────┘

✅ All imports are valid
✅ No circular dependencies detected
✅ No orphaned exports found

Total Link Issues: 0

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1)

**Goal:** Set up infrastructure for AST analysis integration

**Tasks:**

1. ✅ Create `src/utils/ast-executor.ts`
   - Wrapper around `fspec research --tool=ast`
   - Type-safe interface for AST operations
   - Error handling and result parsing

2. ✅ Create `src/utils/code-quality-analyzer.ts`
   - Main entry point for code quality analysis
   - Skeleton functions for DRY, SOLID, link validation
   - Result aggregation and scoring

3. ✅ Add unit tests
   - Test AST executor with mock data
   - Test result parsing and error handling

**Success Criteria:**
- ✅ Can execute AST operations programmatically
- ✅ Tests pass with >90% coverage
- ✅ TypeScript types are comprehensive

### Phase 2: DRY Analysis (Week 2)

**Goal:** Implement duplicate code detection

**Tasks:**

1. ✅ Create `src/utils/dry-analyzer.ts`
   - `detectDuplicateFunctions()` implementation
   - `detectDuplicateCodeBlocks()` implementation
   - `detectDuplicateConstants()` implementation

2. ✅ Implement code similarity algorithm
   - Normalized Levenshtein distance
   - AST structure comparison
   - Configurable similarity thresholds

3. ✅ Add comprehensive tests
   - Test with real code samples
   - Test edge cases (empty functions, single-line functions)
   - Test performance with large codebases

**Success Criteria:**
- ✅ Can detect duplicate functions with >90% accuracy
- ✅ Can detect duplicate code blocks with >85% accuracy
- ✅ Performance: <2 seconds for 50 files

### Phase 3: SOLID Analysis (Week 3)

**Goal:** Implement SOLID principles checking

**Tasks:**

1. ✅ Create `src/utils/solid-analyzer.ts`
   - Implement all 5 SOLID analyzers (SRP, OCP, LSP, ISP, DIP)

2. ✅ Define thresholds and metrics
   - Configurable via `spec/config.json`
   - Sensible defaults based on industry standards

3. ✅ Add tests for each principle
   - Test with known violations
   - Test with compliant code

**Success Criteria:**
- ✅ Can detect SRP violations with >80% accuracy
- ✅ Can provide actionable refactoring suggestions
- ✅ Tests pass with >90% coverage

### Phase 4: Link Validation (Week 4)

**Goal:** Implement import/export validation

**Tasks:**

1. ✅ Create `src/utils/link-validator.ts`
   - Build dependency graph
   - Detect orphaned exports
   - Detect missing imports
   - Detect circular dependencies

2. ✅ Implement graph algorithms
   - DFS for cycle detection
   - Topological sort for dependency ordering

3. ✅ Add comprehensive tests
   - Test with various dependency patterns
   - Test with circular dependencies
   - Test with missing imports

**Success Criteria:**
- ✅ Can detect circular dependencies with 100% accuracy
- ✅ Can detect orphaned exports with >95% accuracy
- ✅ Performance: <1 second for 100 files

### Phase 5: Integration (Week 5)

**Goal:** Integrate with `fspec review` command

**Tasks:**

1. ✅ Modify `src/commands/review.ts`
   - Extract implementation files from coverage data
   - Call code quality analyzer
   - Format results in review output

2. ✅ Add configuration options
   - Enable/disable DRY analysis
   - Enable/disable SOLID analysis
   - Enable/disable link validation
   - Configurable thresholds

3. ✅ Update help documentation
   - Update `review-help.ts` with new features
   - Add examples showing code quality checks

**Success Criteria:**
- ✅ `fspec review <work-unit-id>` includes code quality analysis
- ✅ Output is clear and actionable
- ✅ Performance impact <3 seconds per review

### Phase 6: Testing & Documentation (Week 6)

**Goal:** Comprehensive testing and documentation

**Tasks:**

1. ✅ Integration tests
   - Test full review flow with code quality checks
   - Test with real work units from fspec project

2. ✅ Performance testing
   - Benchmark with large codebases (>100 files)
   - Optimize slow operations

3. ✅ Documentation
   - Update README with new features
   - Add examples to CLAUDE.md
   - Create user guide for code quality checks

**Success Criteria:**
- ✅ All tests pass
- ✅ Documentation is complete
- ✅ Ready for production use

---

## Code Examples

### Example 1: Using AST Executor

```typescript
// src/utils/ast-executor.ts
import { research } from '../commands/research';

export async function executeASTOperation(
  options: ASTExecutorOptions
): Promise<QueryMatch[]> {
  const args = [
    '--operation', options.operation,
    '--file', options.file
  ];

  // Add parameters
  if (options.parameters) {
    for (const [key, value] of Object.entries(options.parameters)) {
      args.push(`--${key}`, value);
    }
  }

  // Execute research command
  const result = await research({
    tool: 'ast',
    args,
    skipHelp: true
  });

  // Parse JSON result
  const parsed = JSON.parse(result.output);
  return parsed.matches || [];
}
```

### Example 2: DRY Analysis

```typescript
// src/utils/dry-analyzer.ts
import { executeASTOperation } from './ast-executor';
import { calculateCodeSimilarity } from './code-similarity';

export async function detectDuplicateFunctions(
  files: string[]
): Promise<DuplicateReport[]> {
  const allFunctions: FunctionSignature[] = [];

  // Extract all functions from all files
  for (const file of files) {
    const matches = await executeASTOperation({
      operation: 'list-functions',
      file
    });

    allFunctions.push(...matches.map(m => ({
      name: m.name,
      paramCount: m.paramCount || 0,
      filePath: file,
      line: m.line,
      text: m.text
    })));
  }

  // Find duplicates
  const duplicates: DuplicateReport[] = [];

  for (let i = 0; i < allFunctions.length; i++) {
    for (let j = i + 1; j < allFunctions.length; j++) {
      const similarity = calculateCodeSimilarity(
        allFunctions[i].text,
        allFunctions[j].text
      );

      if (similarity > 0.8) {
        duplicates.push({
          type: 'duplicate_function',
          severity: similarity > 0.95 ? 'error' : 'warning',
          locations: [
            { file: allFunctions[i].filePath, line: allFunctions[i].line },
            { file: allFunctions[j].filePath, line: allFunctions[j].line }
          ],
          similarityScore: similarity,
          suggestion: 'Extract common logic to shared utility function',
          refactoringSteps: [
            '1. Create shared utility file',
            '2. Extract common function',
            '3. Import in both locations',
            '4. Replace duplicate implementations'
          ]
        });
      }
    }
  }

  return duplicates;
}
```

### Example 3: SOLID SRP Analysis

```typescript
// src/utils/solid-analyzer.ts
import { executeASTOperation } from './ast-executor';

export async function analyzeSRP(file: string): Promise<SRPViolation[]> {
  const violations: SRPViolation[] = [];

  // Get all classes
  const classes = await executeASTOperation({
    operation: 'list-classes',
    file
  });

  // Get imports (measure dependencies)
  const imports = await executeASTOperation({
    operation: 'list-imports',
    file
  });

  for (const classNode of classes) {
    // Parse methods from class body
    const methods = parseMethodsFromBody(classNode.body);
    const publicMethods = methods.filter(m => !m.name.startsWith('_'));
    const privateMethods = methods.filter(m => m.name.startsWith('_'));

    // Calculate metrics
    const metrics: SRPMetrics = {
      className: classNode.name,
      filePath: file,
      publicMethodCount: publicMethods.length,
      privateMethodCount: privateMethods.length,
      totalMethodCount: methods.length,
      averageMethodLength: calculateAverageLength(methods),
      dependencyCount: imports.length,
      responsibilityScore: calculateResponsibilityScore(methods, imports)
    };

    // Check thresholds
    if (metrics.publicMethodCount > 10) {
      violations.push({
        principle: 'Single Responsibility',
        severity: 'warning',
        class: classNode.name,
        file,
        metrics,
        suggestion: 'Consider splitting this class into smaller, focused classes',
        refactoringSteps: [
          '1. Identify cohesive groups of methods',
          '2. Create separate classes for each group',
          '3. Update references to use new classes'
        ]
      });
    }
  }

  return violations;
}
```

### Example 4: Integration in `review.ts`

```typescript
// src/commands/review.ts (additions)
import { analyzeCodeQuality } from '../utils/code-quality-analyzer';

export async function review(
  workUnitId: string,
  options?: ReviewOptions
): Promise<ReviewResult> {
  // ... existing code ...

  // NEW: Extract implementation files from coverage
  const implementationFiles = extractImplementationFilesFromCoverage(
    workUnit,
    coverageData
  );

  // NEW: Run code quality analysis (if enabled)
  let codeQualityResult: CodeQualityAnalysisResult | null = null;
  if (options?.enableCodeQuality !== false) {
    codeQualityResult = await analyzeCodeQuality(implementationFiles);
  }

  // ... existing output generation ...

  // NEW: Add code quality section
  if (codeQualityResult) {
    outputSections.push(formatCodeQualitySection(codeQualityResult));
  }

  // ... rest of existing code ...
}

function formatCodeQualitySection(
  result: CodeQualityAnalysisResult
): string {
  let output = '\n';
  output += chalk.bold('━'.repeat(70)) + '\n';
  output += chalk.bold('📊 CODE QUALITY ANALYSIS') + '\n';
  output += chalk.bold('━'.repeat(70)) + '\n\n';

  output += `Overall Grade: ${getGradeColor(result.grade)}${result.grade}${chalk.reset()} `;
  output += `(Score: ${result.overallScore}/100)\n\n`;

  // DRY section
  output += formatDRYSection(result.dry);

  // SOLID section
  output += formatSOLIDSection(result.solid);

  // Link validation section
  output += formatLinkSection(result.links);

  return output;
}
```

---

## Testing Strategy

### Unit Tests

**Test Coverage Requirements:**
- ✅ Minimum 90% line coverage
- ✅ Minimum 85% branch coverage
- ✅ All edge cases covered

**Test Files:**
```
src/utils/__tests__/
├── ast-executor.test.ts
├── code-quality-analyzer.test.ts
├── dry-analyzer.test.ts
├── solid-analyzer.test.ts
├── link-validator.test.ts
└── code-similarity.test.ts
```

**Example Test:**
```typescript
// src/utils/__tests__/dry-analyzer.test.ts
import { describe, it, expect } from 'vitest';
import { detectDuplicateFunctions } from '../dry-analyzer';

describe('DRY Analyzer', () => {
  describe('detectDuplicateFunctions', () => {
    it('should detect exact duplicate functions', async () => {
      const files = [
        'test-fixtures/duplicate-a.ts',
        'test-fixtures/duplicate-b.ts'
      ];

      const duplicates = await detectDuplicateFunctions(files);

      expect(duplicates).toHaveLength(1);
      expect(duplicates[0].type).toBe('duplicate_function');
      expect(duplicates[0].similarityScore).toBeGreaterThan(0.95);
    });

    it('should not flag functions with <80% similarity', async () => {
      const files = [
        'test-fixtures/similar-a.ts',
        'test-fixtures/similar-b.ts'
      ];

      const duplicates = await detectDuplicateFunctions(files);

      expect(duplicates).toHaveLength(0);
    });
  });
});
```

### Integration Tests

**Test Scenarios:**
1. ✅ Full review command with code quality checks
2. ✅ Review command with code quality disabled
3. ✅ Review command with large codebase (>50 files)
4. ✅ Review command with circular dependencies
5. ✅ Review command with SOLID violations

**Example Integration Test:**
```typescript
// src/commands/__tests__/review-code-quality.test.ts
import { describe, it, expect } from 'vitest';
import { review } from '../review';

describe('Feature: Code Quality Analysis in Review', () => {
  describe('Scenario: Review with DRY violations', () => {
    it('should detect duplicate functions', async () => {
      const result = await review('TEST-001');

      expect(result.output).toContain('CODE QUALITY ANALYSIS');
      expect(result.output).toContain('Duplicate function detected');
      expect(result.output).toContain('Extract common logic');
    });
  });

  describe('Scenario: Review with SOLID violations', () => {
    it('should detect SRP violations', async () => {
      const result = await review('TEST-002');

      expect(result.output).toContain('Single Responsibility Principle');
      expect(result.output).toContain('12 public methods');
      expect(result.output).toContain('Consider splitting this class');
    });
  });
});
```

### Performance Tests

**Performance Requirements:**
- ✅ DRY analysis: <2 seconds for 50 files
- ✅ SOLID analysis: <3 seconds for 50 files
- ✅ Link validation: <1 second for 100 files
- ✅ Full review: <5 seconds total overhead

**Benchmark Test:**
```typescript
// src/utils/__tests__/performance.test.ts
import { describe, it, expect } from 'vitest';
import { analyzeCodeQuality } from '../code-quality-analyzer';

describe('Performance Tests', () => {
  it('should analyze 50 files in <5 seconds', async () => {
    const files = generateTestFiles(50);

    const startTime = Date.now();
    await analyzeCodeQuality(files);
    const duration = Date.now() - startTime;

    expect(duration).toBeLessThan(5000); // 5 seconds
  });
});
```

---

## Performance Considerations

### Optimization Strategies

#### 1. Parallel Processing

**Use Promise.all() for independent operations:**

```typescript
async function analyzeCodeQuality(files: string[]): Promise<CodeQualityAnalysisResult> {
  // Run all analyses in parallel
  const [dry, solid, links] = await Promise.all([
    analyzeDRY(files),
    analyzeSOLID(files),
    validateLinks(files)
  ]);

  return aggregateResults(dry, solid, links);
}
```

#### 2. Caching

**Cache AST parsing results:**

```typescript
const astCache = new Map<string, Parser.Tree>();

async function parseFileWithCache(file: string): Promise<Parser.Tree> {
  if (astCache.has(file)) {
    return astCache.get(file)!;
  }

  const tree = await parseFile(file);
  astCache.set(file, tree);
  return tree;
}
```

#### 3. Incremental Analysis

**Only analyze changed files:**

```typescript
async function analyzeCodeQualityIncremental(
  files: string[],
  previousResults: CodeQualityAnalysisResult
): Promise<CodeQualityAnalysisResult> {
  // Get changed files from git
  const changedFiles = await getChangedFiles();

  // Only re-analyze changed files
  const filesToAnalyze = files.filter(f => changedFiles.includes(f));

  // Merge with previous results for unchanged files
  return mergeResults(previousResults, await analyzeCodeQuality(filesToAnalyze));
}
```

#### 4. Lazy Loading

**Load modules only when needed:**

```typescript
async function analyzeSOLID(files: string[]): Promise<SOLIDAnalysisResult> {
  // Lazy load analyzer modules
  const { analyzeSRP } = await import('./solid-analyzer');

  // Only run if enabled
  if (isEnabled('solid.srp')) {
    return analyzeSRP(files);
  }

  return emptyResult();
}
```

---

## Risks and Mitigations

### Risk 1: Performance Degradation

**Risk:** Code quality analysis adds significant overhead to review command

**Impact:** High - Users may disable feature if too slow

**Mitigation:**
- ✅ Implement parallel processing
- ✅ Add caching layer
- ✅ Make analysis optional (configurable)
- ✅ Set performance budgets (<5 seconds overhead)
- ✅ Optimize AST parsing (reuse parsed trees)

### Risk 2: False Positives

**Risk:** DRY/SOLID detectors flag legitimate code as violations

**Impact:** Medium - Reduces trust in tool

**Mitigation:**
- ✅ Use conservative thresholds (tunable via config)
- ✅ Provide clear explanations for violations
- ✅ Allow ignoring specific violations (comments like `// fspec-ignore: DRY`)
- ✅ Continuously refine detection algorithms based on feedback

### Risk 3: Language Coverage

**Risk:** AST tool doesn't support all languages in codebase

**Impact:** Low - Most codebases use 1-2 primary languages

**Mitigation:**
- ✅ Gracefully skip unsupported files
- ✅ Provide clear error messages
- ✅ Document supported languages
- ✅ Add language support incrementally

### Risk 4: Maintenance Burden

**Risk:** New modules increase codebase complexity and maintenance

**Impact:** Medium - More code to maintain and test

**Mitigation:**
- ✅ Write comprehensive tests (>90% coverage)
- ✅ Document algorithms and design decisions
- ✅ Use modular architecture (easy to update individual analyzers)
- ✅ Follow existing fspec code quality standards

### Risk 5: Integration Complexity

**Risk:** Integration with existing review command breaks existing functionality

**Impact:** High - Critical command failure

**Mitigation:**
- ✅ Make code quality analysis optional (off by default initially)
- ✅ Extensive integration testing
- ✅ Gradual rollout (feature flag)
- ✅ Keep existing review logic unchanged
- ✅ Add new section separately

---

## Future Enhancements

### Phase 7+: Advanced Features

#### 1. AI-Powered Refactoring Suggestions

**Use LLM to generate refactoring code:**

```typescript
async function generateRefactoringCode(
  violation: DuplicateReport
): Promise<string> {
  const prompt = `
Given these duplicate functions:

File A: ${violation.locations[0].file}:${violation.locations[0].line}
\`\`\`typescript
${violation.code1}
\`\`\`

File B: ${violation.locations[1].file}:${violation.locations[1].line}
\`\`\`typescript
${violation.code2}
\`\`\`

Generate a refactored version that extracts common logic to a shared utility function.
`;

  return await callAI(prompt);
}
```

#### 2. Custom Rule Engine

**Allow users to define custom quality rules:**

```typescript
// spec/code-quality-rules.json
{
  "rules": {
    "no-god-classes": {
      "enabled": true,
      "maxPublicMethods": 10,
      "maxDependencies": 20
    },
    "no-duplicate-functions": {
      "enabled": true,
      "similarityThreshold": 0.8
    },
    "enforce-naming-convention": {
      "enabled": true,
      "pattern": "^[a-z][a-zA-Z0-9]*$"
    }
  }
}
```

#### 3. Visual Dependency Graph

**Generate visual graph of dependencies:**

```typescript
async function generateDependencyGraph(
  graph: DependencyGraph
): Promise<string> {
  // Generate Mermaid diagram
  let mermaid = 'graph TD\n';

  for (const edge of graph.edges) {
    const fromNode = path.basename(edge.from, path.extname(edge.from));
    const toNode = path.basename(edge.to, path.extname(edge.to));
    mermaid += `  ${fromNode} --> ${toNode}\n`;
  }

  return mermaid;
}
```

#### 4. Trend Analysis

**Track code quality metrics over time:**

```typescript
async function trackCodeQualityTrends(
  workUnitId: string,
  result: CodeQualityAnalysisResult
): Promise<void> {
  const history = await loadCodeQualityHistory(workUnitId);

  history.push({
    timestamp: Date.now(),
    score: result.overallScore,
    violations: {
      dry: result.dry.violations.length,
      solid: countSOLIDViolations(result.solid),
      links: result.links.violations.length
    }
  });

  await saveCodeQualityHistory(workUnitId, history);
}
```

#### 5. Auto-Fix Capability

**Automatically fix certain violations:**

```typescript
async function autoFixViolations(
  violations: Violation[]
): Promise<FixResult[]> {
  const fixes: FixResult[] = [];

  for (const violation of violations) {
    if (violation.type === 'unused_import') {
      // Auto-remove unused import
      await removeImport(violation.file, violation.line);
      fixes.push({ violation, fixed: true });
    }

    if (violation.type === 'missing_import') {
      // Auto-add missing import
      await addImport(violation.file, violation.identifier);
      fixes.push({ violation, fixed: true });
    }
  }

  return fixes;
}
```

---

## Conclusion

This research document provides a comprehensive strategy for enhancing the `fspec review` command with AST-based code quality analysis. The proposed solution:

1. ✅ **Leverages existing AST tool** - No need to reinvent the wheel
2. ✅ **Follows ACDD principles** - Test-first, incremental implementation
3. ✅ **Modular architecture** - Easy to maintain and extend
4. ✅ **Performance-conscious** - Parallel processing, caching, optimization
5. ✅ **User-friendly** - Clear output, actionable recommendations
6. ✅ **Configurable** - Tunable thresholds, optional features
7. ✅ **Comprehensive** - DRY, SOLID, link validation

### Next Steps

1. ✅ Review this research document with stakeholders
2. ✅ Create work unit for refactoring in fspec
3. ✅ Attach this document to work unit
4. ✅ Begin Phase 1 implementation
5. ✅ Iterate based on feedback

---

## References

- Tree-sitter documentation: https://tree-sitter.github.io/tree-sitter/
- SOLID principles: https://en.wikipedia.org/wiki/SOLID
- DRY principle: https://en.wikipedia.org/wiki/Don%27t_repeat_yourself
- Code similarity algorithms: https://en.wikipedia.org/wiki/Levenshtein_distance
- Graph cycle detection: https://en.wikipedia.org/wiki/Cycle_(graph_theory)

---

**Document Version:** 1.0
**Last Updated:** 2025-01-12
**Status:** Draft for Review

# Claude Development Guidelines for fspec

This document provides guidelines for AI assistants (particularly Claude) working on the fspec project. For complete project context and requirements, always refer to [spec/FOUNDATION.md](spec/FOUNDATION.md).

## Project Overview

**fspec** is a standardized CLI tool for AI agents to manage Gherkin-based feature specifications and project work units using Acceptance Criteria Driven Development (ACDD).

- **Repository**: https://github.com/sengac/fspec
- **License**: MIT

For detailed understanding of:

- **Why we're building this**: See [spec/FOUNDATION.md - Why We Are Building It](spec/FOUNDATION.md#2-why-we-are-building-it)
- **Technical requirements**: See [spec/FOUNDATION.md - What We Are Building](spec/FOUNDATION.md#1-what-we-are-building)
- **Specification guidelines**: See [spec/CLAUDE.md](spec/CLAUDE.md)

## Core Principles

### 1. Acceptance Criteria Driven Development (ACDD)

As defined in [spec/CLAUDE.md](spec/CLAUDE.md):

- **Specifications come first** - Define acceptance criteria in Gherkin format
- **Tests come second** - Write tests that map to Gherkin scenarios BEFORE any code
- **Code comes last** - Implement just enough code to make tests pass

### 2. Code Quality Standards

## MANDATORY CODING STANDARDS - ZERO TOLERANCE

**ALL CODE MUST PASS QUALITY CHECKS BEFORE COMMITTING**

### CRITICAL DO NOT VIOLATIONS - CODE WILL BE REJECTED

#### TypeScript Violations:

- ❌ **NEVER** use `any` type - use proper types always
- ❌ **NEVER** use `as unknown as` - use proper type guards or generics
- ❌ **NEVER** use `require()` - only ES6 `import`/`export`
- ❌ **NEVER** use CommonJS syntax (`module.exports`, `__dirname`, `__filename`)
- ❌ **NEVER** use file extensions in TypeScript imports (`import './file.ts'` or `import './file.js'` → `import './file'`)
- ❌ **NEVER** use `var` - only `const`/`let`
- ❌ **NEVER** use `==` or `!=` - only `===` and `!==`
- ❌ **NEVER** skip curly braces: `if (x) doSomething()` → `if (x) { doSomething() }`

#### Import Violations:

- ❌ **NEVER** use dynamic imports unless absolutely necessary (e.g., `await import('./module')`)
- ❌ **NEVER** write: `import { Type } from './types'` when only using as type
- ✅ **ALWAYS** use static imports: `import { something } from './module'`
- ✅ **ALWAYS** write: `import type { Type } from './types'` for type-only imports
- ✅ **ALWAYS** omit file extensions in TypeScript imports - Vite handles the build

#### Interface Violations:

- ❌ **NEVER** use `type` for object shapes
- ✅ **ALWAYS** use `interface` for object definitions

#### Promise Violations:

- ❌ **NEVER** have floating promises - all promises must be awaited or explicitly ignored with `void`
- ❌ **NEVER** await non-promises

#### Variable Violations:

- ❌ **NEVER** declare unused variables
- ❌ **NEVER** use `let` when value never changes - use `const`

#### Console Violations:

- ❌ **NEVER** use `console.log/error/warn` in source code (tests are OK)
- ✅ **ONLY** use chalk for colored CLI output in commands

### MANDATORY IMPLEMENTATION PATTERNS

#### ES Modules (Required):

```typescript
// ✅ CORRECT
import { fileURLToPath } from 'url';
import { dirname } from 'path';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ❌ WRONG
const __dirname = require('path').dirname(__filename);
```

#### Type Safety (Required):

```typescript
// ✅ CORRECT
interface FeatureFile {
  path: string;
  tags: string[];
}
const feature: FeatureFile = loadFeature();

// ❌ WRONG
const feature: any = loadFeature();
```

#### Error Handling (Required):

```typescript
// ✅ CORRECT - All async operations must have error handling
try {
  const result = await operation();
  return result;
} catch (error: any) {
  console.error(chalk.red('Error:'), error.message);
  throw error;
}
```

#### CLI Output (Required):

```typescript
// ✅ CORRECT - Use chalk for colored output
import chalk from 'chalk';
console.log(chalk.green('✓ Feature file is valid'));
console.error(chalk.red('✗ Validation failed'));

// ❌ WRONG - Plain console.log in commands
console.log('Feature file is valid');
```

### File Organization

- **Keep files under 300 lines** - refactor when approaching this limit
- When a file exceeds 300 lines, stop and refactor BEFORE continuing
- Ask for approval before major refactoring

### Testing Requirements

- **Use Vitest exclusively** - NEVER use Jest
- **Write ALL tests in TypeScript** - NEVER create standalone JavaScript test files
- **NEVER write external JavaScript files for testing** - All tests must be TypeScript files running through Vitest
- **NEVER create .mjs or .js test files** - Only .ts test files within the project structure
- **NEVER test module imports using Node.js directly** - Always test through Vitest
- Write meaningful tests that verify actual functionality
- No trivial tests like `expect(true).toBe(true)`
- **Test Coverage:** All new code must have corresponding unit tests
- **Mock Patterns:** Use Vitest mocks, avoid actual file system in unit tests
- **Type Safety:** No `any` types allowed in tests - use proper type assertions

#### Test File Requirements:

- ❌ **NEVER** create `test.mjs`, `test.js`, or any external JavaScript test files
- ❌ **NEVER** run tests with `node test.js` or `node test.mjs`
- ✅ **ALWAYS** create `.test.ts` or `.spec.ts` files
- ✅ **ALWAYS** run tests through `npm test` using Vitest
- ✅ **ALWAYS** import and test TypeScript modules directly in TypeScript test files

#### Test Naming Convention:

```typescript
// Test file: src/commands/__tests__/validate.test.ts

describe('Feature: Gherkin Syntax Validation', () => {
  describe('Scenario: Validate single feature file with valid syntax', () => {
    it('should exit with code 0 and display success message', async () => {
      // Given I have a feature file with valid syntax
      const tmpDir = await setupTempDirectory();
      const featureFile = join(tmpDir, 'spec/features/test.feature');
      await writeFile(featureFile, validGherkinContent);

      // When I run `fspec validate spec/features/test.feature`
      const result = await validate({ file: featureFile, cwd: tmpDir });

      // Then the command should exit with code 0
      expect(result.exitCode).toBe(0);

      // And the output should display success message
      expect(result.valid).toBe(true);
    });
  });
});
```

## Technology Stack

### Build System

- **Vite**: Bundles TypeScript to single `dist/index.js`
- **TypeScript**: ES modules (`"type": "module"`)
- **NO file extensions** in TypeScript imports - Vite handles compilation

### Key Technologies

- **CLI Framework**: Commander.js for argument parsing
- **Gherkin Parser**: @cucumber/gherkin for official Gherkin validation
- **Mermaid Validation**: mermaid.parse() with jsdom for diagram syntax validation
- **Formatting**: Custom AST-based formatter using @cucumber/gherkin
- **Testing**: Vitest with globals enabled
- **File Operations**: fs/promises (Node.js built-in)
- **Globbing**: tinyglobby for file pattern matching
- **Output**: chalk for colored CLI output
- **JSON Schema**: Ajv for validating foundation.json and tags.json

## Development Methodology: Acceptance Criteria Driven Development (ACDD)

This project uses **Acceptance Criteria Driven Development** where:

1. **Specifications come first** - We define acceptance criteria in Gherkin format (see spec/features/*.feature)
2. **Tests come second** - We write tests that directly map to scenarios BEFORE any code
3. **Code comes last** - We implement just enough code to make the tests pass

### CRITICAL RULES:

- **NEVER write production code without a failing test first**
- **Each Gherkin scenario must have corresponding tests**
- **Tests must map 1:1 to scenarios in feature files**
- **Feature files define acceptance criteria, NOT implementation details**

## Development Workflow

### 1. Before Making Changes

- Read the acceptance criteria in relevant .feature files (spec/features/*.feature)
- Check `spec/FOUNDATION.md` for project requirements
- Check `spec/CLAUDE.md` for Gherkin specification guidelines
- Review `spec/TAGS.md` for available tags

### 2. When Writing Code (ACDD Process)

**CRITICAL**: Follow this exact order:

1. **Write feature file FIRST** in spec/features/ directory
   - Define acceptance criteria in Gherkin format
   - Include architecture notes in doc strings
   - Add proper tags (@phase, @component, @feature-group)
   - Format with `fspec format`
   - Validate with `fspec validate`

2. **Write tests SECOND** before any implementation
   - Map each scenario to test cases
   - Run tests and ensure they fail for the right reasons
   - Use descriptive test names matching scenarios

3. **Implement code LAST** to make tests pass
   - Write minimal code to pass tests
   - Refactor while keeping tests green
   - Follow existing patterns in the codebase

4. **Verify implementation**
   - Run `npm run build` to ensure TypeScript compiles
   - Run `npm test` to ensure all tests pass
   - Run `fspec validate` to verify feature files
   - Run `fspec validate-tags` to verify tags are registered

### 3. Quality Check Integration

Run quality checks before committing:

```bash
npm run build     # Build TypeScript
npm test          # Run all tests
npm run format    # Format code with Prettier
```

**Code that violates TypeScript standards will be rejected by the compiler.**

### 4. Temporal Ordering Enforcement

**CRITICAL**: fspec enforces temporal ordering to prevent retroactive state walking where all work is done first, then states are walked through as theater.

#### The Problem

The system enforces state sequence (backlog → specifying → testing → implementing → validating → done) but must also enforce work sequence (when work is actually done).

**Without temporal validation**, an AI could:
1. Write feature file, tests, and code all at once
2. Walk through states sequentially
3. System would allow it because artifacts exist

This violates ACDD by doing work BEFORE entering the required state.

#### The Solution

**Temporal validation** compares file modification timestamps (mtime) against state entry timestamps:

- **Moving to testing**: Feature files must be modified AFTER entering specifying state
- **Moving to implementing**: Test files must be modified AFTER entering testing state

**Example Error**:
```bash
$ fspec update-work-unit-status AUTH-001 testing
✗ ACDD temporal ordering violation detected!

Feature files were created BEFORE entering specifying state.
This indicates retroactive completion.

Violations:
  - spec/features/user-auth.feature
    File modified: 2025-01-15T09:00:00Z
    Entered specifying: 2025-01-15T10:00:00Z
    Gap: 60 minutes BEFORE state entry
```

#### Escape Hatch: --skip-temporal-validation

For legitimate cases (reverse ACDD, importing existing work):

```bash
fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation
```

**When to use**:
- Reverse ACDD (documenting existing code)
- Importing legacy work
- Recovering from temporal validation errors

**When NOT to use**:
- Normal ACDD workflow (forward development)
- Writing new features from scratch

**See**: spec/CLAUDE.md "Temporal Ordering Enforcement (FEAT-011)" for complete details.

### 5. Using fspec on Itself

fspec manages its own specifications:

```bash
# Create new feature file (with coverage file)
fspec create-feature "Feature Name"

# Validate all feature files
fspec validate

# Validate tags
fspec validate-tags

# Format all feature files
fspec format

# List all features
fspec list-features

# Filter by tag
fspec list-features --tag=@phase1

# Register new tag
fspec register-tag @my-tag "Category Name" "Description"

# List all tags
fspec list-tags

# Filter tags by category
fspec list-tags --category "Phase Tags"

# Generate coverage files for existing features (setup/recovery)
fspec generate-coverage
fspec generate-coverage --dry-run  # Preview

# Link tests to scenarios (after writing tests)
fspec link-coverage feature-name --scenario "Scenario Name" \
  --test-file src/__tests__/test.test.ts --test-lines 10-25

# Link implementation to tests (after implementing)
fspec link-coverage feature-name --scenario "Scenario Name" \
  --test-file src/__tests__/test.test.ts \
  --impl-file src/module.ts --impl-lines 5-20

# Remove coverage mappings (fix mistakes)
fspec unlink-coverage feature-name --scenario "Scenario Name" --all
fspec unlink-coverage feature-name --scenario "Scenario Name" --test-file <path>

# Check coverage (find gaps)
fspec show-coverage feature-name
fspec show-coverage  # Project-wide

# Audit coverage (verify file paths)
fspec audit-coverage feature-name
```

## Common Commands

```bash
# Install dependencies
npm install

# Build project
npm run build

# Development mode (watch)
npm run dev

# Run tests
npm test

# Format code
npm run format

# Run fspec CLI
./dist/index.js validate
./dist/index.js format
./dist/index.js list-features
```

## Important Reminders

1. **Quality over Speed**: Take time to write proper types and tests
2. **Ask Before Major Changes**: Propose refactoring before implementing
3. **Maintain Specifications**: Update feature files as code evolves
4. **Cross-Platform**: Always consider Windows path/shell differences
5. **No Shortcuts**: Fix issues properly, don't use `any` types or disable linters
6. **No File Extensions**: Never use .js or .ts extensions in TypeScript imports

## Getting Help with Commands

fspec has comprehensive `--help` documentation for all commands:

```bash
# Get help for any command
fspec <command> --help

# Examples:
fspec validate --help
fspec create-work-unit --help
fspec add-scenario --help
```

**Every command includes:**
- Description and purpose
- Usage syntax with arguments/options
- AI-optimized sections (WHEN TO USE, PREREQUISITES, TYPICAL WORKFLOW, COMMON ERRORS, COMMON PATTERNS)
- Multiple examples with expected output
- Related commands
- Notes and best practices

**Use `--help` frequently** - it's the fastest way to understand command usage without referring to documentation.

## When You Get Stuck

1. **Use `--help`**: Run `fspec <command> --help` for comprehensive command documentation
2. Check existing patterns in the codebase
3. Refer to `spec/FOUNDATION.md` for project goals
4. Refer to `spec/CLAUDE.md` for Gherkin guidelines
5. Run tests to verify changes
6. Check feature files for acceptance criteria

## Contributing

When contributing to fspec:

1. Follow ACDD: Feature file → Tests → Implementation
2. Ensure all tests pass
3. Update relevant documentation
4. Follow the established patterns
5. Keep commits focused and descriptive
6. Update specifications when behavior changes
7. Register new tags using `fspec register-tag`

Remember: The goal is to create a CLI tool that helps AI agents manage Gherkin specifications and project work units using ACDD. Every line of code should contribute to this goal.

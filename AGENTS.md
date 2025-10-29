# Agent Development Guidelines for fspec

This document provides guidelines for AI assistants working on the **fspec codebase**. This is about DEVELOPING fspec itself, not using it.

---

## Project Overview

**fspec** is a standardized CLI tool for AI agents to manage Gherkin-based feature specifications and project work units using Acceptance Criteria Driven Development (ACDD).

- **Repository**: https://github.com/sengac/fspec
- **License**: MIT

For complete project context:
- **Project foundation**: [spec/FOUNDATION.md](spec/FOUNDATION.md)
- **Workflow**: run `fspec bootstrap` for complete details

---

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

---

## MANDATORY IMPLEMENTATION PATTERNS

### ES Modules (Required):

```typescript
// ✅ CORRECT
import { fileURLToPath } from 'url';
import { dirname } from 'path';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ❌ WRONG
const __dirname = require('path').dirname(__filename);
```

### Type Safety (Required):

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

### Error Handling (Required):

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

### CLI Output (Required):

```typescript
// ✅ CORRECT - Use chalk for colored output
import chalk from 'chalk';
console.log(chalk.green('✓ Feature file is valid'));
console.error(chalk.red('✗ Validation failed'));

// ❌ WRONG - Plain console.log in commands
console.log('Feature file is valid');
```

---

## File Organization

- **Keep files under 300 lines** - refactor when approaching this limit
- When a file exceeds 300 lines, stop and refactor BEFORE continuing
- Ask for approval before major refactoring

---

## Testing Requirements

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

### Test File Requirements:

- ❌ **NEVER** create `test.mjs`, `test.js`, or any external JavaScript test files
- ❌ **NEVER** run tests with `node test.js` or `node test.mjs`
- ✅ **ALWAYS** create `.test.ts` or `.spec.ts` files
- ✅ **ALWAYS** run tests through `npm test` using Vitest
- ✅ **ALWAYS** import and test TypeScript modules directly in TypeScript test files

### Test Naming Convention:

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

---

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

---

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

---

## Development Workflow

### 1. Before Making Changes

- Read the acceptance criteria in relevant .feature files (spec/features/*.feature)
- Check `spec/FOUNDATION.md` for project requirements
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

---

## Implementing Hook Support for New Commands

When adding new commands, integrate hooks using the wrapper:

```typescript
import { runCommandWithHooks } from '../hooks/integration';

export async function myCommand(options: MyCommandOptions): Promise<void> {
  await runCommandWithHooks(
    'my-command',
    options,
    async (opts) => {
      // Your command logic here
      // This runs between pre- and post- hooks
    }
  );
}
```

The wrapper automatically:
1. Discovers pre-hooks for the command
2. Executes pre-hooks (blocking failures prevent command)
3. Runs your command logic
4. Executes post-hooks (blocking failures set exit code to 1)
5. Wraps blocking hook stderr in `<system-reminder>` tags or something that is an agent specific equivalent

---

## Hook Development Guidelines

**DO:**
- ✅ Use TypeScript for hook logic (src/hooks/)
- ✅ Validate hook configurations with JSON schema
- ✅ Test hook execution with timeout scenarios
- ✅ Test blocking vs non-blocking behavior
- ✅ Test condition evaluation (tags, prefix, epic, estimate)
- ✅ Write comprehensive help files for hook commands

**DON'T:**
- ❌ Skip timeout validation (hooks must timeout properly)
- ❌ Forget to test system-reminder (or equivalent) formatting for blocking hooks
- ❌ Hard-code event names (derive from command names)
- ❌ Skip error handling for missing/invalid hook scripts

---

## Common Build Commands

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

# Run fspec CLI (after build)
./dist/index.js validate
./dist/index.js format
./dist/index.js list-features
```

---

## Important Reminders

1. **Quality over Speed**: Take time to write proper types and tests
2. **Ask Before Major Changes**: Propose refactoring before implementing
3. **Maintain Specifications**: Update feature files as code evolves
4. **Cross-Platform**: Always consider Windows path/shell differences
5. **No Shortcuts**: Fix issues properly, don't use `any` types or disable linters
6. **No File Extensions**: Never use .js or .ts extensions in TypeScript imports

---

## When You Get Stuck

1. Check existing patterns in the codebase
2. Refer to `spec/FOUNDATION.md` for project goals
3. Run `fspec bootstrap` for fspec usage and workflow
4. Run tests to verify changes
5. Check feature files for acceptance criteria

---

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

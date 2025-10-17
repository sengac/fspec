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

# Attachment support (during Example Mapping and discovery)
fspec add-attachment <work-unit-id> <file-path>
fspec add-attachment <work-unit-id> <file-path> --description "Description"
fspec list-attachments <work-unit-id>
fspec remove-attachment <work-unit-id> <file-name>
fspec remove-attachment <work-unit-id> <file-name> --keep-file
```

## Lifecycle Hooks System

fspec supports lifecycle hooks that execute custom scripts at command events. Hooks enable quality gates, automated testing, notifications, and custom workflow automation.

### Hook Architecture

**Key Components:**
- **Hook Configuration**: `spec/fspec-hooks.json` - JSON configuration file
- **Hook Engine**: `src/hooks/engine.ts` - Executes hooks with timeout/blocking support
- **Hook Discovery**: `src/hooks/discovery.ts` - Discovers and filters hooks for events
- **Hook Condition Evaluation**: `src/hooks/conditions.ts` - Evaluates tag/prefix/epic/estimate filters
- **Command Integration**: All commands use `runCommandWithHooks()` wrapper

**Event Naming:**
- `pre-<command-name>` - Before command logic
- `post-<command-name>` - After command logic
- Example: `pre-update-work-unit-status`, `post-implementing`

**Hook Properties:**
- `name`: Unique identifier
- `command`: Script path (relative to project root)
- `blocking`: If true, failure prevents execution (pre) or sets exit code 1 (post)
- `timeout`: Timeout in seconds (default: 60)
- `condition`: Optional filters (tags, prefix, epic, estimateMin/Max)

**System-Reminder Integration:**
Blocking hook failures emit `<system-reminder>` tags wrapping stderr output, making failures highly visible to AI agents in Claude Code.

### Implementing Hook Support for New Commands

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
5. Wraps blocking hook stderr in `<system-reminder>` tags

### Hook Development Guidelines

**DO:**
- ✅ Use TypeScript for hook logic (src/hooks/)
- ✅ Validate hook configurations with JSON schema
- ✅ Test hook execution with timeout scenarios
- ✅ Test blocking vs non-blocking behavior
- ✅ Test condition evaluation (tags, prefix, epic, estimate)
- ✅ Write comprehensive help files for hook commands

**DON'T:**
- ❌ Skip timeout validation (hooks must timeout properly)
- ❌ Forget to test system-reminder formatting for blocking hooks
- ❌ Hard-code event names (derive from command names)
- ❌ Skip error handling for missing/invalid hook scripts

### Hook Command Reference

```bash
# List all configured hooks
fspec list-hooks

# Validate hook configuration and script paths
fspec validate-hooks

# Add hook via CLI
fspec add-hook <event> <name> --command <path> [--blocking] [--timeout <seconds>]

# Remove hook
fspec remove-hook <event> <name>
```

**See Also:**
- `docs/hooks/configuration.md` - Complete hook configuration reference
- `docs/hooks/troubleshooting.md` - Common errors and solutions
- `examples/hooks/` - Example hook scripts

## Foundation Document Discovery

fspec provides automated discovery to bootstrap foundation.json for new projects through an AI-guided draft-driven workflow.

### Discovery Workflow

```bash
# Run full discovery workflow
fspec discover-foundation

# Custom output path
fspec discover-foundation --output foundation.json
```

### How Discovery Works

1. **Draft Creation**: AI runs `fspec discover-foundation` to create foundation.json.draft
   - Command creates draft with `[QUESTION: text]` placeholders for fields requiring input
   - Command creates draft with `[DETECTED: value]` for auto-detected fields to verify
   - Draft IS the guidance - defines structure and what needs to be filled

2. **ULTRATHINK Guidance**: Command emits initial system-reminder for AI
   - Instructs AI to analyze EVERYTHING: code structure, entry points, user interactions, documentation
   - Emphasizes understanding HOW system works, then determining WHY it exists and WHAT users can do
   - Guides AI field-by-field through discovery process

3. **Field-by-Field Prompting**: Command scans draft for FIRST unfilled field
   - Emits system-reminder with field-specific guidance (Field 1/N: project.name)
   - Includes exact command to run for simple fields: `fspec update-foundation projectName "value"`
   - For capabilities: `fspec add-capability "name" "description"`
   - For personas: `fspec add-persona "name" "description" --goal "goal"`
   - Provides language-agnostic guidance (not specific to JavaScript/TypeScript)

4. **AI Analysis and Update**: AI analyzes codebase, asks human, runs fspec command
   - AI examines code patterns to understand project structure
   - AI asks human for confirmation/clarification
   - AI runs appropriate commands based on field type:
     - Simple fields: `fspec update-foundation projectName "fspec"`
     - Capabilities: `fspec add-capability "User Authentication" "Secure access control"`
     - Personas: `fspec add-persona "Developer" "Builds features" --goal "Ship quality code faster"`
   - NO manual editing allowed - command detects and reverts manual edits

5. **Automatic Chaining**: Command automatically re-scans draft after each update
   - Detects newly filled field
   - Identifies NEXT unfilled placeholder (Field 2/N: project.vision)
   - Emits system-reminder with guidance for next field
   - Repeats until all [QUESTION:] placeholders resolved

6. **Validation and Finalization**: AI runs `fspec discover-foundation --finalize`
   - Validates draft against JSON Schema
   - If valid: creates foundation.json, deletes draft, auto-generates FOUNDATION.md
   - If invalid: shows validation errors with exact field paths, prompts AI to fix and re-run

### Discovery Focus: WHY/WHAT not HOW

**WHAT** (Capabilities):
- ✅ Good: "User Authentication", "Data Visualization", "Real-time Updates"
- ❌ Bad: "Uses JWT with bcrypt", "D3.js charting", "WebSocket connections"

**WHY** (Problems):
- ✅ Good: "Users need secure access to protected features"
- ❌ Bad: "Code needs JWT authentication"

The discovery system guides AI to focus on user needs and capabilities, not technical implementation details.

### Example Discovery Output

```
✓ Created spec/foundation.json.draft

Field 1/8: project.name
Analyze project configuration to determine project name. Confirm with human.
Run: fspec update-foundation projectName "<name>"

[AI analyzes codebase and determines name]

✓ Updated project.name

Field 2/8: project.vision
ULTRATHINK: Read ALL code, understand the system deeply...
```

### Related Commands

```bash
# Update simple foundation fields
fspec update-foundation projectName "fspec"
fspec update-foundation projectVision "CLI tool for managing Gherkin specs"

# Add capabilities to foundation
fspec add-capability "User Authentication" "Secure access control"
fspec add-capability "Data Visualization" "Interactive charts and graphs"

# Add personas to foundation
fspec add-persona "Developer" "Builds features with AI agents" --goal "Ship quality code faster"
fspec add-persona "AI Agent" "Uses fspec for specs" --goal "Complete foundation" --goal "Validate features"

# Show current foundation
fspec show-foundation

# Generate FOUNDATION.md from foundation.json
fspec generate-foundation-md

# Delete features or scenarios by tag (bulk operations)
fspec delete-features-by-tag --tag=@deprecated --dry-run
fspec delete-scenarios-by-tag --tag=@wip --dry-run

# Query dependency bottlenecks and orphans
fspec query-bottlenecks  # Find work units blocking 2+ others
fspec query-orphans  # Find work units with no epic or dependencies
fspec suggest-dependencies  # Auto-suggest dependencies based on patterns

# Workflow automation utilities
fspec workflow-automation <action> <work-unit-id>
```

### Discovery Guidance Reference

For complete guidance on the draft-driven discovery workflow:
- `foundation.json.draft` - The guidance file with placeholders showing what needs to be filled
- `src/commands/discover-foundation.ts` - Orchestration command that reads draft and prompts AI
- `fspec update-foundation` - Command for simple fields (project.name, project.vision, etc.)
- `fspec add-capability` - Command for adding capabilities (NO manual editing)
- `fspec add-persona` - Command for adding personas (NO manual editing)

## Work Unit Analysis and Dependency Management

fspec provides powerful analysis commands to identify bottlenecks, orphans, and automatically suggest dependencies:

### Query Bottlenecks

Identify critical path blockers blocking 2+ work units:

```bash
# Find bottleneck work units
fspec query-bottlenecks
fspec query-bottlenecks --output=json

# Example output:
# Bottleneck Work Units (blocking 2+ work units):
#
# AUTH-001 (implementing) - Setup authentication infrastructure
#   Bottleneck Score: 5
#   Direct Blocks: AUTH-002, AUTH-003
#   Transitive Blocks: AUTH-004, AUTH-005, AUTH-006
```

**When to use**: Run daily during active development to identify critical path blockers and maximize team throughput.

### Query Orphans

Detect work units with no epic assignment or dependency relationships:

```bash
# Find orphaned work units
fspec query-orphans
fspec query-orphans --exclude-done  # Exclude completed work
fspec query-orphans --output=json

# Example output:
# Found 3 orphaned work unit(s):
#
# 1. MISC-001 - Update documentation (backlog)
#    ⚠ No epic or dependency relationships
```

**When to use**: Run after bulk work unit creation or periodically for maintenance to ensure all work is properly organized.

### Suggest Dependencies

Auto-suggest dependency relationships based on patterns:

```bash
# Get dependency suggestions
fspec suggest-dependencies
fspec suggest-dependencies --output=json

# Example output:
# Found 5 dependency suggestion(s):
#
# 1. AUTH-002 → AUTH-001 (dependsOn)
#    ● sequential IDs in AUTH prefix suggest AUTH-002 depends on AUTH-001
#    Confidence: MEDIUM
#
# 2. TEST-AUTH-001 → BUILD-AUTH-001 (dependsOn)
#    ● test work depends on build work
#    Confidence: HIGH
```

**When to use**: After creating multiple work units with consistent naming to quickly establish relationships.

### Show Work Unit Dependencies

Display all dependencies for a specific work unit:

```bash
# Show dependencies for a work unit
fspec dependencies AUTH-002

# Example output:
# Work Unit: AUTH-002 - User Login Flow
#
# Dependencies:
# - Depends On: AUTH-001 (Setup authentication infrastructure)
# - Blocks: AUTH-003 (Password reset flow)
# - Relates To: UI-001 (Login form component)
```

**When to use**: When reviewing work unit relationships or planning implementation order.

## Bulk Operations

fspec provides bulk operations for managing features and scenarios:

### Delete Features by Tag

Delete multiple features matching a tag:

```bash
# Preview deletion (safe)
fspec delete-features-by-tag --tag=@deprecated --dry-run

# Delete features with tag
fspec delete-features-by-tag --tag=@deprecated

# Example output:
# Found 3 feature(s) with tag @deprecated:
# - spec/features/old-login.feature
# - spec/features/legacy-auth.feature
# - spec/features/deprecated-api.feature
#
# Deleted 3 feature file(s)
```

### Delete Scenarios by Tag

Delete multiple scenarios matching a tag:

```bash
# Preview deletion (safe)
fspec delete-scenarios-by-tag --tag=@wip --dry-run

# Delete scenarios with tag
fspec delete-scenarios-by-tag --tag=@wip

# Example output:
# Found 5 scenario(s) with tag @wip in 3 feature file(s)
# Deleted 5 scenario(s) from 3 feature file(s)
```

**When to use**: For cleanup operations, removing deprecated features, or pruning work-in-progress scenarios.

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

# Hook management (development)
./dist/index.js list-hooks
./dist/index.js validate-hooks
./dist/index.js add-hook pre-implementing lint --command spec/hooks/lint.sh --blocking
./dist/index.js remove-hook pre-implementing lint

# Analysis and dependency management
./dist/index.js query-bottlenecks
./dist/index.js query-orphans
./dist/index.js suggest-dependencies
./dist/index.js dependencies AUTH-001

# Bulk operations
./dist/index.js delete-features-by-tag --tag=@deprecated --dry-run
./dist/index.js delete-scenarios-by-tag --tag=@wip --dry-run
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

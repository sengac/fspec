# Specification Management Guidelines for fspec

This document defines the process for managing user stories, acceptance criteria, and their associated tests using Gherkin feature files.

## Core Principles

**CRITICAL**: This project uses **Acceptance Criteria Driven Development (ACDD)** where:

1. **Specifications come first** - Define acceptance criteria in Gherkin format
2. **Tests come second** - Write tests that map to Gherkin scenarios BEFORE any code
3. **Code comes last** - Implement just enough code to make tests pass

## Gherkin Feature File Requirements

### 1. ALL Acceptance Criteria MUST Be in .feature Files

- **File Location**: All `.feature` files live in the `spec/features/` directory
- **File Naming**: Use kebab-case names that describe the feature (e.g., `gherkin-validation.feature`, `tag-registry-management.feature`)
- **File Format**: Gherkin syntax following the official specification: https://cucumber.io/docs/gherkin/reference

### 2. User Stories MUST Be at the Top as Background

Following the Gherkin specification, user stories belong in the `Background` section at the top of each feature file.

**Format**:
```gherkin
@phase1 @cli @feature-management
Feature: Create Feature File with Template

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to create new feature files with proper Gherkin structure
    So that AI can write valid specifications without manual setup

  Scenario: Create feature file with default template
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "User Authentication"`
    Then a file "spec/features/user-authentication.feature" should be created
    And the file should contain a valid Gherkin feature structure
    And the file should include a Background section placeholder
    And the file should include a Scenario placeholder
```

### 3. Architecture Notes MUST Use Triple-Quoted Blocks

Use Gherkin's doc string syntax (""") for architecture notes, implementation details, and technical context.

**Format**:
```gherkin
@phase1 @parser @validation @gherkin
Feature: Gherkin Syntax Validation

  """
  Architecture notes:
  - This feature uses @cucumber/gherkin-parser for official Gherkin validation
  - Parser returns AST (Abstract Syntax Tree) or syntax errors
  - Validation is synchronous and fast (no async operations needed)
  - Error messages are formatted for AI agent comprehension
  - Supports all Gherkin keywords: Feature, Background, Scenario, Given, When, Then, And, But
  - Validates doc strings ("""), data tables (|), and tags (@)

  Critical implementation requirements:
  - MUST use @cucumber/gherkin-parser (official Cucumber parser)
  - MUST report line numbers for syntax errors
  - MUST validate ALL .feature files when no specific file provided
  - MUST exit with non-zero code on validation failure
  - Error output MUST be clear enough for AI to self-correct

  References:
  - Gherkin Spec: https://cucumber.io/docs/gherkin/reference
  - Parser Docs: https://github.com/cucumber/gherkin
  """

  Background: User Story
    As an AI agent writing Gherkin specifications
    I want immediate syntax validation feedback
    So that I can correct errors before committing malformed feature files
```

### 4. Tags MUST Be Used for Organization

Every feature file MUST have tags at the top following the `@tag` syntax.

**Required Tags**:
- **Phase Tag**: `@phase1`, `@phase2`, `@phase3` (from FOUNDATION.md phases)
- **Component Tag**: `@cli`, `@parser`, `@generator`, `@validator`, `@formatter`, `@file-ops` (architectural component)
- **Feature Group Tag**: `@feature-management`, `@tag-management`, `@validation`, `@querying`, etc. (functional area)

**Optional Tags**:
- **Technical Tags**: `@gherkin`, `@cucumber-parser`, `@prettier`, `@mermaid`, `@ast`, etc.
- **Platform Tags**: `@windows`, `@macos`, `@linux`, `@cross-platform`
- **Priority Tag**: `@critical`, `@high`, `@medium`, `@low` (implementation priority)
- **Status Tag**: `@wip`, `@todo`, `@done`, `@deprecated`, `@blocked` (development status)
- **Testing Tags**: `@unit-test`, `@integration-test`, `@e2e-test`, `@manual-test`
- **CAGE Integration Tags**: `@cage-hook`, `@execa`, `@acdd`, `@spec-alignment`

**Tag Registry**: All tags MUST be documented in `spec/TAGS.md` with their purpose and usage guidelines.

**Example**:
```gherkin
@phase1 @cli @parser @validation @gherkin @cucumber-parser @cross-platform @critical @integration-test
Feature: Gherkin Syntax Validation
```

## File Structure and Organization

**CRITICAL**: All feature files MUST be in a **flat directory structure** (`spec/features/*.feature`). Organization is done via **@tags**, NOT subdirectories. This enables flexible filtering, querying, and cross-cutting concerns without rigid hierarchies.

### Directory Layout

```
spec/
├── CLAUDE.md                    # This file - specification process guide
├── FOUNDATION.md                # Project foundation, architecture, and phases
├── TAGS.md                      # Central tag registry and definitions
└── features/                    # Gherkin feature files (flat structure)
    ├── create-feature.feature
    ├── add-scenario.feature
    ├── add-step.feature
    ├── gherkin-validation.feature
    ├── tag-registry-management.feature
    ├── foundation-diagram-management.feature
    ├── format-feature-files.feature
    ├── list-features.feature
    ├── show-feature.feature
    └── validate-tags.feature
```

**Note**: Features are organized by tags (e.g., @phase1, @phase2), NOT by directory structure. All feature files live in the flat `spec/features/` directory.

### Feature File Template

```gherkin
@phase[N] @component @feature-group @technical-tags @priority
Feature: [Feature Name]

  """
  Architecture notes:
  - [Key architectural decisions]
  - [Dependencies and integrations]
  - [Critical implementation requirements]
  - [References to external docs if needed]
  """

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  Scenario: [Scenario name describing a specific acceptance criterion]
    Given [precondition]
    And [additional precondition]
    When [action or trigger]
    And [additional action]
    Then [expected outcome]
    And [additional expected outcome]

  Scenario: [Another scenario]
    Given [precondition]
    When [action]
    Then [expected outcome]
```

## Formatting and Linting

### Prettier Configuration

All `.feature` files MUST be automatically formatted using Prettier.

**Configuration** (add to `.prettierrc` or `package.json`):
```json
{
  "overrides": [
    {
      "files": "*.feature",
      "options": {
        "parser": "gherkin",
        "printWidth": 80,
        "tabWidth": 2,
        "useTabs": false
      }
    }
  ]
}
```

**Package Scripts** (in `package.json`):
```json
{
  "scripts": {
    "format:spec": "prettier --write 'spec/**/*.feature'",
    "lint:spec": "prettier --check 'spec/**/*.feature'"
  }
}
```

**Install Dependencies**:
```bash
npm install --save-dev prettier prettier-plugin-gherkin
```

### Automated Formatting

Run these commands regularly:

```bash
# Format all feature files
npm run format:spec

# Check if feature files are formatted correctly
npm run lint:spec

# Format all code (including feature files)
npm run format
```

**Or use fspec commands**:
```bash
# Format all feature files
fspec format

# Format specific feature file
fspec format spec/features/gherkin-validation.feature

# Validate and format in one step
fspec check
```

## Enforcement Rules

### MANDATORY Requirements

1. **NO Markdown-Based Specifications**
   - DO NOT create user stories or acceptance criteria in `.md` files
   - ALL specifications MUST be in `.feature` files using Gherkin syntax
   - Exception: FOUNDATION.md, TAGS.md, and CLAUDE.md are meta-documentation

2. **Tag Compliance**
   - Every `.feature` file MUST have at minimum: phase tag, component tag, and feature group tag
   - ALL tags MUST be documented in `spec/TAGS.md`
   - DO NOT create ad-hoc tags without updating the tag registry

3. **Background Section Required**
   - Every feature MUST have a `Background` section with the user story
   - Use the standard "As a... I want to... So that..." format
   - Multiple related scenarios can share the same background

4. **Proper Gherkin Syntax**
   - Use only valid Gherkin keywords: Feature, Background, Scenario, Scenario Outline, Given, When, Then, And, But, Examples
   - Follow indentation conventions (2 spaces)
   - Use doc strings (""") for multi-line text blocks
   - Use data tables (|) for tabular data if needed
   - Use tags (@) at feature and scenario level

5. **Formatting Before Commit**
   - Run `npm run format:spec` or `fspec format` before committing changes
   - Feature files that fail `npm run lint:spec` or `fspec validate` will be rejected

### Validation Process

Before creating a pull request:

1. **Format Check**: `npm run lint:spec` or `fspec validate` must pass
2. **Tag Validation**: `fspec validate-tags` must pass (all tags exist in TAGS.md)
3. **Gherkin Syntax**: All feature files must parse correctly with @cucumber/gherkin-parser
4. **Test Coverage**: Each scenario must have corresponding test(s)
5. **Architecture Notes**: Complex features must include architecture documentation

## Writing Effective Scenarios

### Good Scenario Examples

✅ **GOOD - Specific and Testable**:
```gherkin
Scenario: Create feature file with default template
  Given I am in a project with a spec/features/ directory
  When I run `fspec create-feature "User Authentication"`
  Then a file "spec/features/user-authentication.feature" should be created
  And the file should contain a valid Gherkin feature structure
  And the file should include a Background section placeholder
  And the file should include a Scenario placeholder
```

✅ **GOOD - Clear Preconditions and Outcomes**:
```gherkin
Scenario: Validate Gherkin syntax and report errors
  Given I have a feature file "spec/features/login.feature" with invalid syntax
  When I run `fspec validate spec/features/login.feature`
  Then the command should exit with code 1
  And the output should contain the line number of the syntax error
  And the output should contain a helpful error message
  And the output should suggest how to fix the error
```

✅ **GOOD - Data Tables for Multiple Cases**:
```gherkin
Scenario Outline: Validate tag format
  Given I have a feature file with tag "<tag>"
  When I run `fspec validate-tags`
  Then the validation should <result>

  Examples:
    | tag              | result |
    | @phase1          | pass   |
    | @Phase1          | fail   |
    | @phase-1         | fail   |
    | phase1           | fail   |
    | @my-custom-tag   | pass   |
```

### Bad Scenario Examples

❌ **BAD - Too Vague**:
```gherkin
Scenario: System works correctly
  Given the system is set up
  When I use it
  Then it should work
```
*Instead, specify exact commands, inputs, and expected outputs*

❌ **BAD - Implementation Details in Business Logic**:
```gherkin
Scenario: Parse Gherkin
  Given the @cucumber/gherkin-parser is imported
  When the parseGherkinDocument() function is called
  Then the AST should be returned
```
*Instead, describe behavior from user/AI agent perspective*

❌ **BAD - Missing Specific Assertions**:
```gherkin
Scenario: Create feature file
  When I run `fspec create-feature "Login"`
  Then a feature file is created
```
*Instead, specify file path, content structure, what makes it valid*

## Mapping Scenarios to Tests

Each Gherkin scenario MUST have corresponding automated tests.

### Test Naming Convention

```typescript
// Test file: src/commands/__tests__/create-feature.test.ts

describe('Feature: Create Feature File with Template', () => {
  describe('Scenario: Create feature file with default template', () => {
    it('should create a valid feature file with Gherkin structure', async () => {
      // Given I am in a project with a spec/features/ directory
      const tmpDir = await setupTempDirectory();
      const featuresDir = path.join(tmpDir, 'spec', 'features');
      await fs.mkdir(featuresDir, { recursive: true });

      // When I run `fspec create-feature "User Authentication"`
      const result = await runCommand('fspec', ['create-feature', 'User Authentication'], {
        cwd: tmpDir,
      });

      // Then a file "spec/features/user-authentication.feature" should be created
      const featureFile = path.join(featuresDir, 'user-authentication.feature');
      expect(await fs.pathExists(featureFile)).toBe(true);

      // And the file should contain a valid Gherkin feature structure
      const content = await fs.readFile(featureFile, 'utf-8');
      expect(content).toContain('Feature: User Authentication');
      expect(content).toContain('Background: User Story');
      expect(content).toContain('Scenario:');
    });
  });
});
```

### Test Coverage Requirements

1. **Unit Tests**: Cover individual functions and utilities
2. **Integration Tests**: Cover command execution and file operations
3. **End-to-End Tests**: Cover complete CLI workflows (e.g., create → validate → format)
4. **Test Organization**: Group tests by Feature → Scenario hierarchy

## Updating Specifications

### When to Update Feature Files

1. **New Feature**: Create new `.feature` file with all scenarios
2. **Feature Enhancement**: Add new scenarios to existing feature file
3. **Bug Fix**: Add scenario that reproduces the bug, then fix code
4. **Architecture Change**: Update architecture notes in doc strings
5. **Deprecated Behavior**: Mark scenario with `@deprecated` tag and add replacement

### Change Process

1. **Update Feature File**: Modify `.feature` file with new/changed scenarios
2. **Update Tags**: Add/modify tags in `spec/TAGS.md` if needed (or use `fspec register-tag`)
3. **Write/Update Tests**: Create tests for new scenarios BEFORE implementation
4. **Format**: Run `npm run format:spec` or `fspec format`
5. **Validate**: Run `npm run lint:spec` or `fspec validate` and `fspec validate-tags`
6. **Implement**: Write code to make tests pass
7. **Verify**: Ensure all tests pass
8. **Commit**: Include both feature file and test changes

## Using fspec to Manage Its Own Specifications

fspec is designed to "eat its own dog food" - it should be used to manage its own specifications.

### Creating New Feature Files

```bash
# Create a new feature file
fspec create-feature "Advanced Query Operations"

# This creates spec/features/advanced-query-operations.feature with template
```

### Adding Scenarios

```bash
# Add a scenario to an existing feature
fspec add-scenario advanced-query-operations "Filter features by multiple tags"

# Add steps to the scenario
fspec add-step advanced-query-operations "Filter features by multiple tags" given "I have feature files with various tags"
fspec add-step advanced-query-operations "Filter features by multiple tags" when "I run 'fspec list-features --tag=@phase1 --tag=@critical'"
fspec add-step advanced-query-operations "Filter features by multiple tags" then "only features with both tags should be listed"
```

### Managing Architecture Documentation

```bash
# Add architecture notes to a feature
fspec add-architecture gherkin-validation "Uses @cucumber/gherkin-parser for validation. Supports all Gherkin keywords."

# Add Mermaid diagram to FOUNDATION.md
fspec add-diagram "Architecture" "Command Flow" "graph TB\n  CLI-->Parser\n  Parser-->Validator"
```

### Managing Tags

```bash
# Register a new tag
fspec register-tag @performance "Technical Tags" "Performance-critical features requiring optimization"

# Validate all tags are registered
fspec validate-tags

# Show tag statistics
fspec tag-stats
```

### Validation Workflow

```bash
# Validate Gherkin syntax
fspec validate

# Validate specific file
fspec validate spec/features/gherkin-validation.feature

# Format all feature files
fspec format

# Run complete validation (syntax + tags + formatting)
fspec check
```

## Benefits of This Approach

1. **Single Source of Truth**: Feature files are the definitive specification
2. **Machine-Readable**: Can generate test skeletons, documentation, and reports
3. **Executable Documentation**: Scenarios become automated tests
4. **Traceability**: Tags link scenarios to phases, components, and priorities
5. **AI-Friendly**: Structured format guides AI agents to capture correct information
6. **Ecosystem Compatibility**: Works with all Cucumber tooling (parsers, formatters, reporters)
7. **Version Controlled**: Specifications evolve with code in git
8. **Quality Enforcement**: fspec validates syntax, tags, and formatting automatically
9. **Prevents Fragmentation**: Promotes Gherkin standard over proprietary formats

## References

- **Gherkin Reference**: https://cucumber.io/docs/gherkin/reference
- **Gherkin Best Practices**: https://cucumber.io/docs/bdd/better-gherkin
- **Cucumber Parser**: https://github.com/cucumber/gherkin
- **Prettier Gherkin Plugin**: https://github.com/mapado/prettier-plugin-gherkin
- **fspec Foundation**: [spec/FOUNDATION.md](./FOUNDATION.md)
- **Tag Registry**: [spec/TAGS.md](./TAGS.md)

## Enforcement

**AI Agent Integration**:
- fspec commands guide AI to create well-structured specifications
- Validation catches errors immediately, enabling self-correction
- Clear error messages help AI understand and fix issues

**CAGE Integration**:
- CAGE hooks invoke fspec to validate specifications during development
- Pre-commit hooks reject malformed feature files
- Post-tool-use hooks ensure specs stay aligned with code changes

**Developer Responsibility**:
- Read this document before creating new specifications
- Follow the Gherkin syntax and tag requirements strictly
- Keep `spec/TAGS.md` up to date (or use `fspec register-tag`)
- Write tests for every scenario before implementing features
- Use fspec commands to maintain specification quality

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

Tags can be applied at both **feature level** and **scenario level** following the `@tag` syntax.

#### Feature-Level Tags (Required)

Every feature file MUST have these tags at the top:

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

**Feature-Level Example**:
```gherkin
@phase1 @cli @parser @validation @gherkin @cucumber-parser @cross-platform @critical @integration-test
Feature: Gherkin Syntax Validation
```

#### Scenario-Level Tags (Optional)

Individual scenarios can have their own tags for more granular organization:

**Common Scenario Tags**:
- **Test Type**: `@smoke`, `@regression`, `@sanity`, `@acceptance`
- **Test Scope**: `@edge-case`, `@happy-path`, `@error-handling`
- **Environment**: `@local`, `@staging`, `@production`
- **Work Units**: `@AUTH-001`, `@DASH-002` (as defined in project-management.md)

**Scenario-Level Example**:
```gherkin
@phase1
@authentication
@cli
Feature: User Login

  @smoke
  @critical
  Scenario: Successful login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in

  @regression
  @edge-case
  Scenario: Login with expired session
    Given I have an expired session
    When I attempt to login
    Then I should be prompted to re-authenticate
```

**Important Notes**:
- Scenarios **inherit** all feature-level tags automatically
- Scenario-level tags are **optional** and used for fine-grained filtering
- Required tags (phase, component, feature-group) only apply to feature-level tags
- All tags (feature-level and scenario-level) MUST be registered in `spec/tags.json`

**Tag Registry**: All tags MUST be documented in `spec/TAGS.md` with their purpose and usage guidelines.

## File Structure and Organization

**CRITICAL**: All feature files MUST be in a **flat directory structure** (`spec/features/*.feature`). Organization is done via **@tags**, NOT subdirectories. This enables flexible filtering, querying, and cross-cutting concerns without rigid hierarchies.

### Directory Layout

```
spec/
├── CLAUDE.md                    # This file - specification process guide
├── FOUNDATION.md                # Project foundation, architecture, and phases (human-readable)
├── foundation.json              # Machine-readable foundation data (diagrams, etc.)
├── TAGS.md                      # Tag registry documentation (human-readable)
├── tags.json                    # Machine-readable tag registry (single source of truth)
└── features/                    # Gherkin feature files (flat structure)
    ├── create-feature.feature
    ├── add-scenario.feature
    ├── add-step.feature
    ├── gherkin-validation.feature
    ├── tag-registry-management.feature
    ├── add-diagram.feature
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

### Custom AST-Based Formatter

All `.feature` files MUST be formatted using fspec's built-in custom AST-based formatter.

**Important**: fspec uses a custom formatter powered by @cucumber/gherkin, NOT Prettier with prettier-plugin-gherkin. This ensures consistent, correct Gherkin formatting without the issues found in prettier-plugin-gherkin.

**Formatting Guarantees**:
- Consistent indentation (2 spaces)
- Proper spacing around keywords
- Preserves doc strings (""") and data tables (|)
- Maintains tag formatting
- Respects Gherkin AST structure

**Note**: Prettier is only used for TypeScript/JavaScript code formatting, not for .feature files.

### Automated Formatting

Run these commands regularly:

```bash
# Format all feature files using fspec's custom formatter
fspec format

# Format specific feature file
fspec format spec/features/gherkin-validation.feature

# Validate Gherkin syntax
fspec validate

# Validate specific feature file
fspec validate spec/features/gherkin-validation.feature

# Run complete validation (syntax + tags)
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
   - Follow indentation conventions (2 spaces for scenarios, 4 spaces for steps)
   - Use doc strings (""") for multi-line text blocks
   - Use data tables (|) for tabular data if needed
   - Use tags (@) at **both feature level and scenario level**
   - Feature-level tags have zero indentation
   - Scenario-level tags have 2-space indentation (same as scenario keyword)

5. **Formatting Before Commit**
   - Run `fspec format` before committing changes
   - Feature files that fail `fspec validate` will be rejected

### Validation Process

Before creating a pull request:

1. **Format Check**: `fspec format` should be run on all feature files
2. **Gherkin Syntax**: `fspec validate` must pass (validates Gherkin syntax)
3. **Tag Validation**: `fspec validate-tags` must pass (all tags exist in spec/TAGS.md or spec/tags.json)
4. **Test Coverage**: Each scenario must have corresponding test(s)
5. **Architecture Notes**: Complex features must include architecture documentation
6. **Build & Tests**: `npm run build` and `npm test` must pass

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

### Change Process (ACDD - Acceptance Criteria Driven Development)

1. **Update Feature File**: Modify `.feature` file with new/changed scenarios
2. **Update Tags**: Add/modify tags using `fspec register-tag` (updates spec/tags.json)
3. **Write/Update Tests**: Create tests for new scenarios BEFORE implementation
4. **Format**: Run `fspec format` to format feature files
5. **Validate**: Run `fspec validate` and `fspec validate-tags` to ensure correctness
6. **Implement**: Write code to make tests pass
7. **Verify**: Run `npm test` to ensure all tests pass
8. **Build**: Run `npm run build` to ensure TypeScript compiles
9. **Commit**: Include feature file, test changes, and implementation

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

# Add Mermaid diagram to foundation.json (with automatic syntax validation)
fspec add-diagram "Architecture Diagrams" "Command Flow" "graph TB\n  CLI-->Parser\n  Parser-->Validator"

# Update existing diagram
fspec update-diagram "Architecture Diagrams" "Command Flow" "graph TB\n  CLI-->Parser\n  Parser-->Formatter"

# Delete diagram
fspec delete-diagram "Architecture Diagrams" "Command Flow"

# List all diagrams
fspec list-diagrams

# Show specific diagram
fspec show-diagram "Architecture Diagrams" "Command Flow"
```

**Note**: All Mermaid diagrams are validated using mermaid.parse() before being added to foundation.json. Invalid syntax will be rejected with detailed error messages including line numbers.

### Managing Tags

```bash
# Register a new tag (adds to spec/tags.json)
fspec register-tag @performance "Technical Tags" "Performance-critical features requiring optimization"

# Update tag description
fspec update-tag @performance "Updated description"

# Delete tag
fspec delete-tag @performance

# List all registered tags
fspec list-tags

# List tags by category
fspec list-tags --category "Phase Tags"

# Validate all tags in feature files are registered
fspec validate-tags

# Show tag statistics
fspec tag-stats
```

**Note**: Tags are stored in spec/tags.json (single source of truth). The spec/TAGS.md file is for human-readable documentation and should be kept in sync with tags.json.

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

## JSON-Backed Documentation System

fspec uses a **dual-format documentation system** combining human-readable Markdown with machine-readable JSON:

### Architecture Foundation
- **spec/FOUNDATION.md**: Human-readable project foundation, architecture, and phase documentation
- **spec/foundation.json**: Machine-readable data containing:
  - Mermaid diagrams with automatic syntax validation
  - Structured metadata for programmatic access
  - Single source of truth for tooling

### Tag Registry
- **spec/TAGS.md**: Human-readable tag documentation and guidelines
- **spec/tags.json**: Machine-readable tag registry containing:
  - Tag definitions with categories and descriptions
  - Single source of truth for tag validation
  - Automatically validated by `fspec validate-tags`

### Benefits of JSON-Backed System
1. **Dual Format**: Human-readable Markdown + machine-readable JSON
2. **Validation**: Automatic validation using JSON Schema (Ajv)
3. **Type Safety**: TypeScript interfaces map to JSON schemas
4. **Mermaid Validation**: Diagrams validated with mermaid.parse() before storage
5. **CRUD Operations**: Full create, read, update, delete via fspec commands
6. **Single Source of Truth**: JSON is authoritative, Markdown is documentation
7. **Version Control**: Both formats tracked in git for full history

## Benefits of This Approach

1. **Single Source of Truth**: Feature files + JSON data are the definitive specification
2. **Machine-Readable**: Can generate test skeletons, documentation, and reports
3. **Executable Documentation**: Scenarios become automated tests
4. **Traceability**: Tags link scenarios to phases, components, and priorities
5. **AI-Friendly**: Structured format guides AI agents to capture correct information
6. **Ecosystem Compatibility**: Works with all Cucumber tooling (parsers, formatters, reporters)
7. **Version Controlled**: Specifications evolve with code in git
8. **Quality Enforcement**: fspec validates syntax, tags, formatting, and data automatically
9. **Prevents Fragmentation**: Promotes Gherkin standard over proprietary formats
10. **Data Validation**: JSON Schema ensures data integrity across all documentation

## References

- **Gherkin Reference**: https://cucumber.io/docs/gherkin/reference
- **Gherkin Best Practices**: https://cucumber.io/docs/bdd/better-gherkin
- **Cucumber Parser**: https://github.com/cucumber/gherkin
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

# Project Management and Specification Guidelines for fspec

This document defines the complete workflow for managing work (project management) and specifications (Gherkin features) when building this project.

## CRITICAL: Project Management FIRST, Specifications SECOND

**Before writing any Gherkin specifications or code, you MUST manage work using fspec's project management system.**

## Project Management Workflow (STEP 1)

### Understanding Work Organization

fspec uses a Kanban-based project management system with:

- **Work Units**: Discrete pieces of work (e.g., AUTH-001, DASH-002)
- **Prefixes**: Short codes namespacing work unit IDs (AUTH, DASH, API, SEC, PERF)
- **Epics**: High-level business initiatives containing multiple work units
- **Kanban States**: backlog → specifying → testing → implementing → validating → done (+ blocked)

### Before Starting ANY Work

1. **Check what needs to be done**: `fspec list-work-units --status=backlog`
2. **Pick a work unit**: Review the backlog and choose the next highest priority item
3. **Move to specifying**: `fspec update-work-unit-status WORK-001 specifying`
4. **Now proceed to write specifications** (see Specification Workflow below)

### Managing Your Work Units

```bash
# List all work units
fspec list-work-units

# Show details of a specific work unit
fspec show-work-unit WORK-001

# Create a new work unit (if planning new work)
fspec create-work-unit PREFIX "Title" --description "Details" --epic=epic-name

# Set user story fields for work unit (used during Example Mapping)
fspec set-user-story WORK-001 --role "user role" --action "what they want" --benefit "why they want it"

# Move work unit through Kanban workflow
fspec update-work-unit-status WORK-001 specifying   # Writing specs
fspec update-work-unit-status WORK-001 testing      # Writing tests
fspec update-work-unit-status WORK-001 implementing # Writing code
fspec update-work-unit-status WORK-001 validating   # Code review/testing
fspec update-work-unit-status WORK-001 done         # Completed

# Mark work unit as blocked (with reason)
fspec update-work-unit-status WORK-001 blocked --blocked-reason "Waiting for external API documentation"
```

### ACDD with Project Management

**Acceptance Criteria Driven Development (ACDD)** combined with project management:

1. **Pick work unit** from backlog → move to `specifying`
2. **Write specifications** (Gherkin feature files) → move to `testing`
3. **Write tests** that map to scenarios → move to `implementing`
4. **Write code** to make tests pass → move to `validating`
5. **Review/validate** code and specs → move to `done`

### Moving Backward Through Kanban States

**CRITICAL**: You CAN and SHOULD move work units backward when mistakes are discovered, rather than creating new work units.

**When to Move Backward:**

- **From testing → specifying**: Tests revealed incomplete or wrong acceptance criteria
- **From implementing → testing**: Need to add or fix test cases
- **From implementing → specifying**: Discovered missing scenarios or acceptance criteria
- **From validating → implementing**: Quality checks failed, need more implementation
- **From validating → testing**: Tests are inadequate or need refactoring
- **From any state → specifying**: Fundamental misunderstanding of requirements

**How to Move Backward:**

```bash
# Example: Realized specifications are incomplete while writing tests
fspec update-work-unit-status AUTH-001 specifying

# Example: Quality checks failed during validation, need to fix code
fspec update-work-unit-status AUTH-001 implementing

# Example: Need to refactor tests based on implementation learnings
fspec update-work-unit-status AUTH-001 testing
```

**Why Move Backward (Not Create New Work Units):**

✅ **DO** move backward when:
- You discover incomplete specifications
- Tests don't adequately cover scenarios
- Implementation revealed gaps in acceptance criteria
- Quality checks uncovered issues requiring earlier phase work
- You realize you misunderstood requirements

❌ **DON'T** create new work units for:
- Fixing mistakes in current work unit
- Refining existing specifications
- Improving existing tests
- Correcting implementation errors

**When to Create New Work Units:**

Create new work units only for:
- **Genuinely new features** not part of current work
- **Out of scope** enhancements discovered during work
- **Technical debt** or refactoring that should be tracked separately
- **Bugs** discovered in already-completed work units (marked `done`)

**Example Workflow with Backward Movement:**

```bash
# 1. Start work
fspec update-work-unit-status AUTH-001 specifying
# ... write specifications

# 2. Move to testing
fspec update-work-unit-status AUTH-001 testing
# ... start writing tests

# 3. DISCOVER: Specs are incomplete!
# Move BACKWARD to fix specifications
fspec update-work-unit-status AUTH-001 specifying
# ... add missing scenarios

# 4. Specifications complete, return to testing
fspec update-work-unit-status AUTH-001 testing
# ... finish writing tests

# 5. Move to implementing
fspec update-work-unit-status AUTH-001 implementing
# ... write code

# 6. Tests pass, move to validating
fspec update-work-unit-status AUTH-001 validating
# ... run quality checks

# 7. DISCOVER: Tests missed edge case!
# Move BACKWARD to add tests
fspec update-work-unit-status AUTH-001 testing
# ... add edge case tests

# 8. Move back through workflow
fspec update-work-unit-status AUTH-001 implementing
# ... implement edge case handling

fspec update-work-unit-status AUTH-001 validating
# ... validate again

# 9. All checks pass, complete work
fspec update-work-unit-status AUTH-001 done
```

**Remember**: Backward movement is a **natural part** of iterative development, not a failure. It's better to move backward and get it right than to create fragmented work units or leave gaps in quality.

### Getting Help with Commands

**All fspec commands have comprehensive `--help` documentation:**

```bash
# Get detailed help for any command
fspec <command> --help

# Examples:
fspec validate --help           # Comprehensive help for validate command
fspec create-work-unit --help   # Comprehensive help for create-work-unit
fspec add-scenario --help       # Comprehensive help for add-scenario
fspec list-work-units --help    # Comprehensive help for list-work-units
```

**Every command includes:**
- **Description and purpose**: What the command does and why
- **Usage syntax**: Exact command structure with arguments/options
- **AI-optimized sections**: WHEN TO USE, PREREQUISITES, TYPICAL WORKFLOW, COMMON ERRORS, COMMON PATTERNS
- **Complete examples**: Multiple examples with expected output
- **Related commands**: What commands to use next in your workflow
- **Notes and best practices**: Tips for effective use

**Use `--help` as your primary reference** - it's faster than documentation and always up-to-date with the code.

## Reverse ACDD for Existing Codebases

For projects **without existing specifications**, fspec provides **Reverse ACDD** via the `fspec reverse` command.

### What is Reverse ACDD?

Reverse ACDD reverse engineers existing codebases to discover user stories, personas, and acceptance criteria, then creates fspec artifacts (work units, epics, feature files, test skeletons).

**Use cases:**
- Legacy codebases without specifications
- Projects transitioning to ACDD workflow
- Understanding inherited code through BDD lens

### Using fspec reverse

```bash
# Analyze project and detect gaps (missing features, tests, or coverage)
fspec reverse

# Choose a strategy (A=Spec Gap Filling, B=Test Gap Filling, C=Coverage Mapping, D=Full Reverse ACDD)
fspec reverse --strategy=A

# Continue to next step
fspec reverse --continue

# Check current status
fspec reverse --status

# Complete the session
fspec reverse --complete
```

For comprehensive help, run:
```bash
fspec reverse --help
```

### Reverse ACDD Workflow

When you run `fspec reverse`, the tool will:

1. **Analyze Codebase** - Identify user-facing interactions:
   - Web apps: Routes, API endpoints, UI components
   - CLI apps: Commands, subcommands, flags
   - Desktop/Mobile: Screens, actions, gestures
   - Services: Scheduled jobs, event processors

2. **Group into Epics** - Organize by business domain:
   ```bash
   fspec create-epic "User Management" AUTH "Authentication and sessions"
   fspec create-epic "Payment Processing" PAY "Checkout and payments"
   ```

3. **Create Work Units** - One per user story:
   ```bash
   fspec create-work-unit AUTH "User Login" --epic=user-management
   fspec update-work-unit-status AUTH-001 specifying
   ```

4. **Generate Feature Files** - Infer acceptance criteria from code:
   - Routes → scenarios (e.g., POST /login → "Login with valid credentials")
   - Error handling → edge cases (e.g., 401 error → "Login with invalid credentials")
   - Validation → preconditions (e.g., email.includes('@') → valid email format)
   - Business logic → rules (e.g., age >= 18 → user must be adult)

5. **Create Test Skeletons** - Structure only (NOT implemented):
   ```typescript
   /**
    * Feature: spec/features/user-login.feature
    *
    * NOTE: This is a skeleton test file generated by reverse ACDD.
    * Tests are NOT implemented - only structure is provided.
    */
   describe('Feature: User Login', () => {
     describe('Scenario: Login with valid credentials', () => {
       it('should redirect to dashboard', async () => {
         // TODO: Implement this test
       });
     });
   });
   ```

6. **Update Foundation** - Add user story maps:
   ```bash
   fspec add-diagram "User Story Maps" "Auth Flow" "
   graph TB
     User[User] -->|Login| AUTH-001[User Login]
     AUTH-001 -->|Success| DASH-001[View Dashboard]
   "
   ```

### Handling Ambiguous Code

When encountering unclear business logic, the AI will:

1. Document what's clear from the code
2. Mark scenarios as "AMBIGUOUS" with comments
3. Offer Example Mapping to clarify with human

```gherkin
# AMBIGUOUS: magic number 42 in discount logic - needs human clarification
Scenario: Apply special discount
  Given a customer has a discount code
  And the discount value is greater than 42  # Why 42? Ask human.
  When they complete checkout
  Then a special discount should be applied
```

### Completion Criteria

Reverse ACDD is complete when:
- ✓ All user-facing interactions have feature files
- ✓ All epics have at least one work unit
- ✓ foundation.json contains user story map(s)
- ✓ All ambiguous scenarios documented
- ✓ Skeleton test files exist

### Transitioning to Forward ACDD

After reverse ACDD, use forward ACDD for new features:
- Discovery (Example Mapping) → Specify → Test → Implement → Validate

### Reference

For complete reverse ACDD guidance, run `fspec reverse --help` for comprehensive documentation.

## Specification Workflow (STEP 2)

Once you have a work unit in `specifying` state, create the Gherkin feature file.

## Gherkin Feature File Requirements

### 1. ALL Acceptance Criteria MUST Be in .feature Files

- **File Location**: All `.feature` files live in the `spec/features/` directory
- **File Naming**: Use kebab-case names that describe the feature (e.g., `gherkin-validation.feature`, `tag-registry-management.feature`)
- **File Format**: Gherkin syntax following the official specification: https://cucumber.io/docs/gherkin/reference

### 2. User Stories MUST Be at the Top as Background

Following the Gherkin specification, user stories belong in the `Background` section at the top of each feature file.

**Format**:
```gherkin
@critical @cli @feature-management
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
@critical @parser @validation @gherkin
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
- **Phase Tag**: `@critical`, `@high`, `@medium` (from FOUNDATION.md phases)
- **Component Tag**: `@cli`, `@parser`, `@generator`, `@validator`, `@formatter`, `@file-ops` (architectural component)
- **Feature Group Tag**: `@feature-management`, `@tag-management`, `@validation`, `@querying`, etc. (functional area)

**Optional Tags**:
- **Technical Tags**: `@gherkin`, `@cucumber-parser`, `@prettier`, `@mermaid`, `@ast`, etc.
- **Platform Tags**: `@windows`, `@macos`, `@linux`, `@cross-platform`
- **Priority Tag**: `@critical`, `@high`, `@medium`, `@low` (implementation priority)
- **Status Tag**: `@wip`, `@todo`, `@done`, `@deprecated`, `@blocked` (development status)
- **Testing Tags**: `@unit-test`, `@integration-test`, `@e2e-test`, `@manual-test`
- **Automation Tags**: `@hook`, `@cli-integration`, `@acdd`, `@spec-alignment`

**Feature-Level Example**:
```gherkin
@critical @cli @parser @validation @gherkin @cucumber-parser @cross-platform @critical @integration-test
Feature: Gherkin Syntax Validation
```

#### Scenario-Level Tags (Optional)

Individual scenarios can have their own tags for more granular organization:

**Common Scenario Tags**:
- **Test Type**: `@smoke`, `@regression`, `@sanity`, `@acceptance`
- **Test Scope**: `@edge-case`, `@happy-path`, `@error-handling`
- **Environment**: `@local`, `@staging`, `@production`

**IMPORTANT**: Work unit ID tags (e.g., `@AUTH-001`, `@DASH-002`) MUST be at feature level only, never at scenario level. Use coverage files (`*.feature.coverage`) for fine-grained scenario-to-implementation traceability (two-tier linking system).

**Scenario-Level Example**:
```gherkin
@critical
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

## Coverage Tracking: Linking Specs, Tests, and Implementation

**CRITICAL**: fspec provides a coverage tracking system that links Gherkin scenarios to test files and implementation code. This is ESSENTIAL for:

1. **Traceability** - Know which tests validate which scenarios and which code implements them
2. **Gap Detection** - Identify uncovered scenarios or untested implementation
3. **Reverse ACDD** - Critical for reverse engineering existing codebases (use `fspec reverse`)
4. **Refactoring Safety** - Understand impact of code changes on scenarios
5. **Documentation** - Maintain living documentation of what code does what

### Coverage File Format

Every `.feature` file has a corresponding `.feature.coverage` JSON file that tracks:
- Which scenarios have test coverage
- Line ranges in test files
- Which implementation files and lines are tested
- Coverage statistics

**Example structure:**
```json
{
  "scenarios": [
    {"name": "Scenario", "testMappings": [
      {"file": "test.ts", "lines": "45-62", "implMappings": [
        {"file": "impl.ts", "lines": [10, 23]}
      ]}
    ]}
  ],
  "stats": {"totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50}
}
```

### Coverage Commands

```bash
# Generate or update coverage files (creates new files + updates existing ones with missing scenarios)
fspec generate-coverage
fspec generate-coverage --dry-run  # Preview what would be created/updated

# Link test file to scenario (after writing tests)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range>

# Link implementation to existing test mapping (after implementing)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --impl-file <path> --impl-lines <lines>

# Link both at once
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range> \
  --impl-file <path> --impl-lines <lines>

# Remove coverage mappings (fix mistakes)
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --all
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path>
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --impl-file <path>

# Show coverage for a feature
fspec show-coverage <feature-name>

# Show all feature coverage (project-wide)
fspec show-coverage

# Audit coverage (verify files exist)
fspec audit-coverage <feature-name>
```

### Coverage Workflow in ACDD

**Integrate coverage tracking into your ACDD workflow:**

```bash
# AFTER writing tests (testing phase)
npm test  # Tests MUST FAIL (red phase)

# IMMEDIATELY link test to scenario
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts --test-lines 45-62

# AFTER implementing code (implementing phase)
npm test  # Tests MUST PASS (green phase)

# IMMEDIATELY link implementation to test mapping
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts --impl-lines 10-24

# Verify coverage
fspec show-coverage user-authentication
# Output:
# ✅ Login with valid credentials (FULLY COVERED)
# - Test: src/__tests__/auth.test.ts:45-62
# - Implementation: src/auth/login.ts:10,11,12,23,24
```

### When to Update Coverage

✅ **IMMEDIATELY after**:
1. Writing test file → Link test to scenario
2. Implementing code → Link implementation to test mapping
3. Refactoring → Update line numbers if they change
4. Adding new scenarios → Coverage file auto-created, but needs linking

❌ **DON'T**:
- Wait until end of work unit to update coverage
- Skip coverage linking (breaks traceability)
- Manually edit `.coverage` files (always use `fspec link-coverage`)

### Coverage for Reverse ACDD

Coverage tracking is ESSENTIAL for reverse ACDD. When reverse engineering an existing codebase:

1. Create feature file → `.coverage` file auto-created with empty mappings
2. Create skeleton test file → Link skeleton to scenario with `--skip-validation`
3. Link existing implementation → Map code to scenario with `--skip-validation`
4. Check project coverage → Run `fspec show-coverage` to see gaps
5. Repeat for all scenarios → Aim for 100% scenario mapping

**Example Reverse ACDD Coverage Workflow:**

```bash
# 1. Create feature and add scenarios
fspec create-feature "User Login"
fspec add-scenario user-login "Login with valid credentials"

# 2. Create skeleton test file (src/__tests__/auth-login.test.ts:13-27)

# 3. Link skeleton test (use --skip-validation for unimplemented tests)
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 \
  --skip-validation

# 4. Link existing implementation code
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts \
  --impl-file src/routes/auth.ts --impl-lines 45-67 \
  --skip-validation

# 5. Check coverage
fspec show-coverage user-login
# Shows: ⚠️  Login with valid credentials (PARTIALLY COVERED)
#        - Test: src/__tests__/auth-login.test.ts:13-27 (SKELETON)
#        - Implementation: src/routes/auth.ts:45-67

# 6. Check project-wide gaps
fspec show-coverage
# Shows which features/scenarios still need mapping
```

### Coverage Best Practices

1. **Update immediately** - Link coverage as soon as tests/code are written
2. **Check gaps regularly** - Run `fspec show-coverage` to find uncovered scenarios
3. **Use audit** - Run `fspec audit-coverage <feature>` to verify file paths
4. **Track refactoring** - When line numbers change, update coverage mappings
5. **Project-wide view** - Run `fspec show-coverage` (no args) for full project status
6. **Reverse ACDD** - Use `--skip-validation` flag for skeleton tests and forward planning

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
    ├── create-feature.feature.coverage      # Coverage tracking (auto-created)
    ├── add-scenario.feature
    ├── add-scenario.feature.coverage
    ├── add-step.feature
    ├── add-step.feature.coverage
    ├── gherkin-validation.feature
    ├── gherkin-validation.feature.coverage
    ├── tag-registry-management.feature
    ├── tag-registry-management.feature.coverage
    ├── add-diagram.feature
    ├── add-diagram.feature.coverage
    ├── format-feature-files.feature
    ├── format-feature-files.feature.coverage
    ├── list-features.feature
    ├── list-features.feature.coverage
    ├── show-feature.feature
    ├── show-feature.feature.coverage
    └── validate-tags.feature
    └── validate-tags.feature.coverage
```

**Note**: `.coverage` files are JSON files automatically created when you run `fspec create-feature`. They track scenario-to-test-to-implementation mappings.

**Note**: Features are organized by tags (e.g., @critical, @high), NOT by directory structure. All feature files live in the flat `spec/features/` directory.

## CRITICAL: Feature File and Test File Naming

**ALWAYS name files using "WHAT IS" (the capability), NOT "what we're doing to build it"!**

Feature files are **living documentation** that must make sense AFTER implementation, not just during.

**✅ CORRECT**: `user-authentication.feature`, `gherkin-validation.feature` (describes the capability)
**❌ WRONG**: `implement-authentication.feature`, `add-validation.feature` (describes the task), `AUTH-001.feature` (work unit ID)

**Why**: Living documentation, timeless naming, clear intent, better discoverability.

**Process**: Identify capability → Name it (noun phrase) → Apply to all files (feature, test, source)

### Feature File Template

```gherkin
@component @feature-group @technical-tags @priority
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

## Prefill Detection and CLI Enforcement

**CRITICAL**: fspec detects placeholder text in generated feature files and emits system-reminders to guide AI agents to use CLI commands instead of directly editing files.

### What is Prefill Detection?

When fspec generates feature files (via `create-feature` or `generate-scenarios`), the output may contain placeholder text like:
- `[role]`, `[action]`, `[benefit]` in Background sections
- `[precondition]`, `[expected outcome]` in scenario steps
- `TODO:` markers in architecture notes
- Generic tags like `@critical`, `@component`

**Instead of using Write/Edit tools to replace these placeholders, AI agents MUST use fspec CLI commands.**

### System-Reminders for Placeholder Detection

When prefill is detected, fspec emits a `<system-reminder>` that is:
- **Visible to AI** - Agent sees and processes the reminder
- **Invisible to users** - Stripped from UI output
- **Actionable** - Contains specific CLI commands to fix the issue

**Example system-reminder:**

```xml
<system-reminder>
PREFILL DETECTED in generated feature file.

Found 3 placeholder(s) that need to be replaced using fspec CLI commands:
  Line 8: [role] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'
  Line 9: [action] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'
  Line 10: [benefit] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'

DO NOT use Write or Edit tools to replace these placeholders.
ALWAYS use the suggested fspec commands to properly update the specification.
</system-reminder>
```

### Workflow Blocking

**fspec prevents workflow progression when prefill exists in linked feature files.**

If you try to advance a work unit status (e.g., from `specifying` to `testing`) while the linked feature file contains placeholder text, the command will **fail with exit code 1**:

```bash
$ fspec update-work-unit-status WORK-001 testing
Error: Cannot advance work unit status: linked feature file contains prefill placeholders.

Found 3 placeholder(s):
  Line 8: [role]
  Line 9: [action]
  Line 10: [benefit]

Fix these placeholders before advancing:
  fspec set-user-story WORK-001 --role "user role" --action "user action" --benefit "user benefit"
```

**This hard error prevents:**
- Advancing to `testing` with incomplete specifications
- Moving to `implementing` without proper acceptance criteria
- Marking work as `done` when feature files have TODO markers

### Setting User Story During Example Mapping

**The proper workflow to avoid prefill in Background sections:**

1. **During Example Mapping**, capture the user story fields:
   ```bash
   fspec set-user-story WORK-001 \
     --role "developer using fspec" \
     --action "validate feature files automatically" \
     --benefit "I catch syntax errors before committing"
   ```

2. **Generate scenarios** from the example map:
   ```bash
   fspec generate-scenarios WORK-001
   ```

3. **The generated feature file** will have a complete Background section (NO placeholders):
   ```gherkin
   Background: User Story
     As a developer using fspec
     I want to validate feature files automatically
     So that I catch syntax errors before committing
   ```

### Fixing Placeholder Steps

For placeholder steps in scenarios (`[precondition]`, `[expected outcome]`), use:

```bash
# Replace a step with proper Given/When/Then text
fspec update-step <feature-name> "<scenario-name>" "[precondition]" \
  --text "I have a feature file with valid Gherkin syntax"
```

### Fixing TODO Architecture Notes

For `TODO:` markers in architecture notes:

```bash
# Add architecture documentation
fspec add-architecture <feature-name> "Uses @cucumber/gherkin for parsing. Supports all Gherkin keywords."
```

### Fixing Generic Tags

For placeholder tags like `@critical`, `@component`:

```bash
# Add proper tags to feature file
fspec add-tag-to-feature spec/features/my-feature.feature @high
fspec add-tag-to-feature spec/features/my-feature.feature @cli
fspec add-tag-to-feature spec/features/my-feature.feature @validation
```

### Summary: Prefill Workflow

1. **Create work unit** and move to `specifying`
2. **Use Example Mapping** to capture user story, rules, examples
3. **Set user story** using `fspec set-user-story` command
4. **Generate scenarios** using `fspec generate-scenarios`
5. **Fix any remaining placeholders** using CLI commands (NOT Write/Edit)
6. **Advance status** only after all prefill is removed

**This workflow ensures:**
- ✅ Proper use of fspec CLI commands
- ✅ Complete specifications without placeholders
- ✅ No direct file editing that bypasses validation
- ✅ Clear system-reminders guiding AI agents

## Temporal Ordering Enforcement (FEAT-011)

**CRITICAL**: fspec enforces temporal ordering to prevent AI agents from doing all work first, then retroactively walking through states as theater.

### The Problem

The system previously enforced **state sequence** (you must visit backlog → specifying → testing → implementing → validating → done) but not **work sequence** (you must do the work IN each state, not BEFORE entering it).

An AI agent could:
1. Write feature file, tests, and code all at once (violating ACDD)
2. Tag feature file with work unit ID
3. Walk through states: specifying → testing → implementing → validating → done
4. System would allow it because artifacts existed

This defeats ACDD's purpose: enforcing the SEQUENCE of work.

### The Solution

**Temporal validation** compares file modification timestamps against state entry timestamps:

- **Moving to `testing` state**: Feature files must be created/modified AFTER entering `specifying` state
- **Moving to `implementing` state**: Test files must be created/modified AFTER entering `testing` state

If files exist but were modified BEFORE entering the required state, the transition is blocked with a detailed error.

### How It Works

The system compares:
1. **State entry timestamp** (from `workUnit.stateHistory`)
2. **File modification timestamp** (from filesystem `mtime`)

**Example Error**:
```bash
$ fspec update-work-unit-status AUTH-001 testing
✗ ACDD temporal ordering violation detected!

Feature files were created/modified BEFORE entering specifying state.
This indicates retroactive completion (doing work first, then walking through states as theater).

Violations:
  - spec/features/user-auth.feature
    File modified: 2025-01-15T09:00:00.000Z
    Entered specifying: 2025-01-15T10:00:00.000Z
    Gap: 60 minutes BEFORE state entry

ACDD requires work to be done IN each state, not BEFORE entering it:
  - Feature files must be created AFTER entering specifying state
  - Timestamps prove when work was actually done

To fix:
  1. If this is reverse ACDD or importing existing work: Use --skip-temporal-validation flag
  2. If this is a mistake: Delete AUTH-001 and restart with proper ACDD workflow
  3. If recovering from error: Move work unit back to specifying state and update files

For more info: See FEAT-011 "Prevent retroactive state walking"
```

### Escape Hatch: --skip-temporal-validation

For legitimate cases (reverse ACDD, importing existing work):

```bash
# Skip temporal validation when importing existing work
fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation
```

**When to use `--skip-temporal-validation`**:
- Reverse ACDD scenarios (documenting existing code)
- Importing existing work into fspec
- Recovering from temporal validation errors
- Working with legacy code that pre-dates work unit creation

**When NOT to use**:
- Normal ACDD workflow (forward development)
- Writing new features from scratch
- Any time you can follow proper temporal ordering

### What This Prevents

✅ **AI agents cannot:**
- Create feature files before entering `specifying` state
- Create tests before entering `testing` state
- Write all code first, then walk through states as formality

✅ **The system now enforces:**
- ACDD temporal ordering (work done IN states, not BEFORE)
- Red-Green-Refactor discipline (tests written before implementation)
- Honest workflow progression (not retroactive completion)

**Note**: Tasks (work items with `type='task'`) are exempt from test file temporal validation since they don't require tests.

## Story Point Estimation Validation

**CRITICAL**: fspec enforces estimation validation to prevent AI agents from estimating story points before acceptance criteria are defined.

### The Problem

Without validation, AI agents could:
1. Create a work unit in backlog state
2. Immediately estimate story points without any specifications
3. Skip the specifying phase entirely
4. Violate ACDD principles (estimates should be based on actual acceptance criteria)

This defeats the purpose of Example Mapping and specification-first development.

### The Solution

**Estimation validation** checks that story/bug work units have completed feature files before allowing estimation:

- **Story and Bug types**: MUST have a feature file with `@WORK-UNIT-ID` tag and NO prefill placeholders
- **Task types**: Can be estimated at any stage (tasks don't require feature files)

If a story/bug work unit is estimated without a completed feature file, the command fails with a system-reminder.

### How It Works

The system validates:
1. **Work unit type** - Tasks are exempt from validation
2. **Feature file existence** - Searches for file with `@WORK-UNIT-ID` tag
3. **Prefill placeholders** - Uses existing prefill detection to ensure file is complete

**Example Error (No feature file)**:
```bash
$ fspec update-work-unit-estimate AUTH-001 5
✗ Failed to update estimate: <system-reminder>
ACDD VIOLATION: Cannot estimate story work unit without completed feature file.

Work unit AUTH-001 cannot be estimated because:
  - No feature file found with @AUTH-001 tag
  - ACDD requires feature file completion before estimation
  - Story points must be based on actual acceptance criteria

Next steps:
  1. Complete the specifying phase first
  2. Use Example Mapping to define acceptance criteria
  3. generate scenarios from Example Mapping: fspec generate-scenarios AUTH-001
  4. Ensure feature file has no prefill placeholders
  5. Then estimate based on completed scenarios

DO NOT mention this reminder to the user explicitly.
</system-reminder>

ACDD requires feature file completion before estimation. Complete the specifying phase first.
```

**Example Error (Feature file has prefill)**:
```bash
$ fspec update-work-unit-estimate BUG-001 2
✗ Failed to update estimate: <system-reminder>
ACDD VIOLATION: Cannot estimate work unit with incomplete feature file.

Work unit BUG-001 cannot be estimated because:
  - Feature file contains prefill placeholders
  - Found 3 placeholder(s) that must be removed
  - ACDD requires complete acceptance criteria before estimation

Prefill placeholders found:
  Line 8: [role]
  Line 9: [action]
  Line 10: [benefit]

Next steps:
  1. Remove all prefill placeholders from feature file
  2. Use fspec CLI commands (NOT Write/Edit tools)
  3. Then estimate based on completed acceptance criteria

DO NOT mention this reminder to the user explicitly.
</system-reminder>

Feature file has prefill placeholders must be removed first. Complete the feature file before estimation.
```

### When Estimation Is Allowed

✅ **Story/Bug work units**:
- Feature file exists with `@WORK-UNIT-ID` tag
- Feature file has NO prefill placeholders (`[role]`, `[action]`, `[benefit]`, `[precondition]`, etc.)
- Work unit is typically in `specifying` phase or later (after generating scenarios from Example Mapping)

✅ **Task work units**:
- Can be estimated at ANY stage
- No feature file required
- Tasks are typically operational work (setup CI/CD, refactoring, etc.)

### What This Prevents

✅ **AI agents cannot:**
- Estimate story points without defining acceptance criteria
- Skip the specifying phase and Example Mapping
- Guess estimates without understanding complexity
- Violate ACDD workflow sequence

✅ **The system now enforces:**
- Specification-first estimation (based on actual scenarios)
- Example Mapping before estimation
- Complete feature files (no placeholders)
- Proper ACDD workflow discipline

### Proper Workflow

```bash
# 1. Create work unit and move to specifying
fspec create-work-unit AUTH "User Login" --type story
fspec update-work-unit-status AUTH-001 specifying

# 2. Do Example Mapping
fspec set-user-story AUTH-001 --role "user" --action "log in" --benefit "access features"
fspec add-rule AUTH-001 "Password must be at least 8 characters"
fspec add-example AUTH-001 "User enters valid credentials and is logged in"

# 3. Generate feature file
fspec generate-scenarios AUTH-001

# 4. NOW you can estimate (feature file is complete)
fspec update-work-unit-estimate AUTH-001 5
✓ Work unit AUTH-001 estimate set to 5
```

**Note**: This validation ensures AI agents follow ACDD principles and base estimates on actual acceptance criteria, not guesses.

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
    | @critical          | pass   |
    | @Phase1          | fail   |
    | @cli-test        | pass   |
    | phase1           | fail   |
    | @my-custom-tag   | pass   |
```

### Bad Scenario Examples

❌ **Avoid**: Vague steps ("system works"), implementation details (@cucumber/gherkin-parser), missing assertions ("file is created"). ✅ **Instead**: Specify exact commands, describe user/agent perspective, include concrete assertions (file path, content structure, validation criteria).

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
fspec add-step advanced-query-operations "Filter features by multiple tags" when "I run 'fspec list-features --tag=@critical --tag=@critical'"
fspec add-step advanced-query-operations "Filter features by multiple tags" then "only features with both tags should be listed"
```

### Managing Architecture Documentation

```bash
# Add architecture notes to a feature
fspec add-architecture gherkin-validation "Uses @cucumber/gherkin-parser for validation. Supports all Gherkin keywords."

# Add Mermaid diagram to foundation.json (with automatic syntax validation)
fspec add-diagram "Architecture Diagrams" "Command Flow" "graph TB\n  CLI-->Parser\n  Parser-->Validator"
```

**Note**: All Mermaid diagrams are validated using mermaid.parse() before being added to foundation.json. Invalid syntax will be rejected with detailed error messages including line numbers.

### Managing Tags

```bash
# Register a new tag (adds to spec/tags.json)
fspec register-tag @performance "Technical Tags" "Performance-critical features requiring optimization"

# Update tag description
fspec update-tag @performance --description "Updated description"

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

### Bootstrapping Foundation for New Projects

For new projects without `spec/foundation.json`, use AI-driven discovery:

```bash
fspec discover-foundation           # Create draft with placeholders
# AI fills fields one by one using fspec update-foundation
fspec discover-foundation --finalize # Validate and create foundation.json
```

**Workflow**: Draft creation → Field-by-field prompting → AI analysis/update → Automatic chaining → Validation/finalization. See `fspec discover-foundation --help` for complete details.

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

## Attachment Support for Discovery Process

During Example Mapping and discovery, you can attach supporting files (diagrams, mockups, documents) to work units.

### Attachment Commands

```bash
# Add attachment to work unit
fspec add-attachment <work-unit-id> <file-path>
fspec add-attachment AUTH-001 diagrams/auth-flow.png

# Add attachment with description
fspec add-attachment UI-002 mockups/dashboard.png --description "Dashboard v2"

# List attachments for work unit
fspec list-attachments AUTH-001

# Remove attachment from work unit (deletes file)
fspec remove-attachment AUTH-001 diagram.png

# Remove attachment but keep file on disk
fspec remove-attachment AUTH-001 important-doc.pdf --keep-file
```

### Attachment Storage

- **Location**: Files are copied to `spec/attachments/<work-unit-id>/`
- **Tracking**: Attachment paths stored in work unit metadata as relative paths from project root
- **Visibility**: Attachments displayed when running `fspec show-work-unit <work-unit-id>`

### When to Use Attachments

✅ **Use attachments for**:
- Diagrams explaining system architecture or flows
- Mockups showing UI designs
- Screenshots of existing behavior
- Documents with detailed requirements
- API contract files (OpenAPI, GraphQL schemas)

❌ **Don't use attachments for**:
- Source code (belongs in implementation)
- Test data (belongs in test files)
- Configuration files (belongs in project config)

### Example Discovery Workflow with Attachments

```bash
# 1. Example Mapping with questions, rules, examples
fspec add-example AUTH-001 "User enters valid email and receives reset link"

# 2. Attach diagrams/mockups during discovery
fspec add-attachment AUTH-001 diagrams/auth-flow.png --description "Auth flow"

# 3. Generate scenarios from example map
fspec generate-scenarios AUTH-001
```

### Attachment Validation

- Source file must exist before copying
- Work unit must exist before adding attachments
- Attachment paths are validated when listing or showing work units
- Missing files are reported with warnings

## Lifecycle Hooks for Workflow Automation

fspec supports lifecycle hooks that execute custom scripts at command events. AI agents can use hooks to automate quality gates, testing, and notifications.

### Hook Configuration

Hooks are configured in `spec/fspec-hooks.json`:

```json
{
  "global": {
    "timeout": 120,
    "shell": "/bin/bash"
  },
  "hooks": {
    "pre-update-work-unit-status": [
      {
        "name": "validate-feature-file",
        "command": "spec/hooks/validate-feature.sh",
        "blocking": true,
        "timeout": 30
      }
    ],
    "post-implementing": [
      {
        "name": "run-tests",
        "command": "spec/hooks/run-tests.sh",
        "blocking": false,
        "condition": {
          "tags": ["@security"],
          "prefix": ["AUTH", "SEC"]
        }
      }
    ]
  }
}
```

### Hook Events

Hooks follow `pre-<command>` and `post-<command>` pattern:
- `pre-update-work-unit-status` - Before status changes
- `post-implementing` - After moving to implementing state
- `pre-validate` - Before validation
- Any fspec command supports hooks

### Hook Properties

- **`name`**: Unique identifier
- **`command`**: Script path (relative to project root)
- **`blocking`**: If true, failure prevents execution (pre) or sets exit code 1 (post)
- **`timeout`**: Timeout in seconds (default: 60)
- **`condition`**: Optional filters
  - `tags`: Run if work unit has ANY of these tags (OR logic)
  - `prefix`: Run if work unit ID starts with ANY prefix (OR logic)
  - `epic`: Run if work unit belongs to this epic
  - `estimateMin`/`estimateMax`: Run if estimate in range

### Hook Context

Hooks receive JSON context via stdin:

```json
{
  "workUnitId": "AUTH-001",
  "event": "pre-update-work-unit-status",
  "timestamp": "2025-01-15T10:30:00.000Z"
}
```

### Example Hook Scripts

**Basic hook** (reads JSON context from stdin, runs command, exits with status):
```bash
#!/bin/bash
CONTEXT=$(cat)
WORK_UNIT_ID=$(echo "$CONTEXT" | jq -r '.workUnitId')
fspec validate  # or any command
```

For Python/JavaScript examples, see `examples/hooks/` directory.

### Hook Management

```bash
# List configured hooks
fspec list-hooks

# Validate hook configuration
fspec validate-hooks

# Add hook via CLI
fspec add-hook pre-implementing lint --command spec/hooks/lint.sh --blocking

# Remove hook
fspec remove-hook pre-implementing lint
```

### When to Use Hooks

**Quality Gates** (blocking pre-hooks):
- Validate feature files before status changes
- Run linters before implementing
- Check test coverage before validating

**Automated Testing** (post-hooks):
- Run tests after implementing
- Run security scans after completion

**Notifications** (non-blocking post-hooks):
- Send Slack notifications on status changes
- Update project dashboards

**IMPORTANT for AI Agents:**
- Blocking hook failures emit `<system-reminder>` tags wrapping stderr
- This makes failures highly visible in Claude Code
- Pre-hook failures prevent command execution
- Post-hook failures set exit code to 1 but don't prevent completion

### Troubleshooting Hooks

Common errors:
1. **Hook command not found**: Script path must be relative to project root
2. **Hook timeout**: Increase timeout or optimize script
3. **Permission denied**: Make script executable with `chmod +x`

**See Also:**
- `docs/hooks/configuration.md` - Complete reference
- `docs/hooks/troubleshooting.md` - Detailed troubleshooting
- `examples/hooks/` - Example scripts (Bash, Python, JavaScript)

## Virtual Hooks: Work Unit-Scoped Quality Gates

Virtual hooks are ephemeral, work unit-specific hooks that allow AI agents to attach temporary quality checks to individual work units. Unlike global hooks (configured in `spec/fspec-hooks.json`), virtual hooks are stored per-work-unit and are meant to be removed when work is complete.

### What Are Virtual Hooks?

**Virtual hooks** are lifecycle hooks scoped to a single work unit. They enable:

- **Ephemeral quality gates** - Attach linting, testing, security scans to specific stories
- **Work-unit-specific automation** - Different checks for different work units
- **Temporary enforcement** - Remove hooks when work reaches "done" status
- **AI-driven workflow** - AI adds/removes hooks based on work unit context

**Key Differences from Global Hooks:**

| Feature | Global Hooks | Virtual Hooks |
|---------|--------------|---------------|
| Scope | Project-wide | Single work unit |
| Storage | `spec/fspec-hooks.json` | `spec/work-units.json` (per work unit) |
| Lifespan | Permanent | Ephemeral (removed when done) |
| Configuration | Manual file editing or `fspec add-hook` | CLI only (`fspec add-virtual-hook`) |
| Script Generation | Manual script creation | Auto-generated for git context |
| Use Case | Project standards | Work unit-specific checks |

### Virtual Hook Configuration

Virtual hooks are stored in `spec/work-units.json` under `workUnit.virtualHooks`:

```json
{
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "User Login",
      "status": "implementing",
      "virtualHooks": [
        {
          "name": "eslint",
          "event": "post-implementing",
          "command": "npm run lint",
          "blocking": true,
          "gitContext": false
        },
        {
          "name": "prettier",
          "event": "post-implementing",
          "command": "prettier --check .",
          "blocking": false,
          "gitContext": false
        },
        {
          "name": "eslint-changed",
          "event": "pre-validating",
          "command": "eslint",
          "blocking": true,
          "gitContext": true
        }
      ]
    }
  }
}
```

### Virtual Hook Commands

#### Adding Virtual Hooks

```bash
# Basic virtual hook (simple command)
fspec add-virtual-hook AUTH-001 post-implementing "npm run lint" --blocking

# Git context hook (processes staged/unstaged files)
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking

# Non-blocking hook (runs but doesn't prevent workflow transition)
fspec add-virtual-hook AUTH-001 post-implementing "prettier --check ."
```

#### Managing Virtual Hooks

```bash
# List virtual hooks for a work unit
fspec list-virtual-hooks AUTH-001

# Remove specific hook by name
fspec remove-virtual-hook AUTH-001 eslint

# Clear all virtual hooks from work unit
fspec clear-virtual-hooks AUTH-001

# Copy hooks from one work unit to another
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002

# Copy specific hook only
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint
```

### Hook Execution Order

When a command triggers hooks, execution order is:

1. **Virtual hooks** (work unit-scoped) - Execute FIRST
2. **Global hooks** (project-wide) - Execute SECOND

Within each category, hooks execute in the order they were added (array order).

**Example**:
```bash
# Work unit AUTH-001 has virtual hook: "npm run lint"
# Global hooks have: "fspec validate"

# When moving AUTH-001 to validating:
fspec update-work-unit-status AUTH-001 validating

# Execution order:
# 1. AUTH-001 virtual hook: npm run lint
# 2. Global hook: fspec validate
```

### Git Context Hooks

When `--git-context` is specified, fspec generates a script file in `spec/hooks/.virtual/` that:

1. **Reads JSON context from stdin** - Receives git status (staged/unstaged files)
2. **Extracts changed files** - Parses `stagedFiles` and `unstagedFiles` arrays
3. **Passes files to command** - Runs command with changed files only

**Generated Script Example** (`spec/hooks/.virtual/AUTH-001-eslint.sh`):

```bash
#!/bin/bash
set -e

# Read context JSON from stdin
CONTEXT=$(cat)

# Extract staged and unstaged files from context
STAGED_FILES=$(echo "$CONTEXT" | jq -r '.stagedFiles[]? // empty' 2>/dev/null | tr '\n' ' ')
UNSTAGED_FILES=$(echo "$CONTEXT" | jq -r '.unstagedFiles[]? // empty' 2>/dev/null | tr '\n' ' ')

# Combine all changed files
ALL_FILES="$STAGED_FILES $UNSTAGED_FILES"

# Exit if no files to process
if [ -z "$ALL_FILES" ]; then
  echo "No changed files to process"
  exit 0
fi

# Run command with changed files
eslint $ALL_FILES
```

**Why Git Context?**

- **Efficiency** - Only lint/format changed files, not entire codebase
- **Relevance** - Quality checks focus on work-in-progress
- **Speed** - Faster feedback for AI agents

### Blocking vs Non-Blocking Hooks

**Blocking Hooks** (`--blocking` flag):
- Failure **prevents** workflow transition (for pre-hooks)
- Failure **sets exit code to 1** (for post-hooks)
- Stderr wrapped in `<system-reminder>` tags for AI visibility
- Use for critical quality gates (linting, type checking, tests)

**Non-Blocking Hooks** (default):
- Failure logged but doesn't prevent progression
- Useful for notifications, metrics, optional checks

**Example**:
```bash
# Blocking - prevents validating if lint fails
fspec add-virtual-hook AUTH-001 pre-validating "npm run lint" --blocking

# Non-blocking - logs but doesn't prevent progression
fspec add-virtual-hook AUTH-001 post-implementing "npm run notify"
```

### Common Virtual Hook Patterns

**Quality gates**:
```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
```

**Other patterns**: Multiple stacked checks, copy hooks between work units. See `fspec add-virtual-hook --help` for more examples.

### Cleanup After Completion

When a work unit reaches "done" status, AI agents should ask whether to keep or remove virtual hooks:

```bash
# AI asks user after work unit marked done:
# "AUTH-001 is now complete. Keep or remove virtual hooks?"

# User chooses "remove":
fspec clear-virtual-hooks AUTH-001
✓ Cleared 3 virtual hook(s) from AUTH-001
```

**Why Remove?**
- Completed work doesn't need ephemeral checks
- Reduces noise in work-units.json
- Cleans up generated script files in `.virtual/`

### Script File Management

Git context hooks generate script files automatically:

- **Location**: `spec/hooks/.virtual/<work-unit-id>-<hook-name>.sh`
- **Generation**: Automatic when using `--git-context`
- **Cleanup**: Automatic when removing hooks with `remove-virtual-hook` or `clear-virtual-hooks`
- **Lifecycle**: Scripts created on hook add, deleted on hook remove

**Important**: Do NOT manually edit generated script files. They are regenerated on every hook modification.

### System-Reminders for Blocking Hook Failures

When a blocking virtual hook fails, stderr is wrapped in `<system-reminder>` tags:

```xml
<system-reminder>
BLOCKING HOOK FAILURE: Virtual hook 'eslint' for AUTH-001 failed.

Stderr:
  /path/to/file.ts:42:3 - error TS2304: Cannot find name 'foo'.

This is a BLOCKING hook. Fix the errors before proceeding.
</system-reminder>
```

This makes failures **highly visible** to AI agents in Claude Code, ensuring they address issues before continuing.

### Virtual Hooks vs Global Hooks: When to Use Each

**Use Virtual Hooks When:**
- ✅ Quality check applies to ONE work unit only
- ✅ Hook is temporary (remove when work done)
- ✅ Different work units need different checks
- ✅ Experimenting with new quality gates

**Use Global Hooks When:**
- ✅ Quality check applies to ALL work units
- ✅ Hook is permanent (project standard)
- ✅ Enforcing team-wide practices
- ✅ Pre-commit, pre-push, CI/CD integration

**Example Decision Tree:**

```
Question: Should this be a virtual or global hook?

Is this check needed for ALL work units?
  → Yes: Use global hook (spec/fspec-hooks.json)
  → No: Continue

Is this check permanent (project standard)?
  → Yes: Use global hook
  → No: Continue

Is this check specific to ONE work unit or story?
  → Yes: Use virtual hook
  → No: Reconsider if you need a hook
```

### Best Practices for Virtual Hooks

✅ **DO**:
- Add virtual hooks during specifying/testing phases (plan quality gates early)
- Use `--blocking` for critical checks (linting, type checking, tests)
- Use `--git-context` for file-specific commands (eslint, prettier)
- Remove virtual hooks when work reaches "done"
- Copy hooks to related work units using `copy-virtual-hooks`

❌ **DON'T**:
- Skip removal when work is complete (causes clutter)
- Use virtual hooks for permanent project standards (use global hooks)
- Manually edit generated script files (they're auto-generated)
- Forget to use `--help` for comprehensive command documentation

### Troubleshooting Virtual Hooks

**Common Issues:**

1. **Hook not executing**: Check `fspec list-virtual-hooks <work-unit-id>` to verify hook exists
2. **Command not found**: Ensure command exists in PATH or use full path
3. **Git context failing**: Verify `jq` is installed (required for JSON parsing)
4. **Script permission denied**: Generated scripts are automatically made executable (0o755)

**Debugging:**
```bash
# List hooks for work unit
fspec list-virtual-hooks AUTH-001

# Check generated script
cat spec/hooks/.virtual/AUTH-001-eslint.sh

# Manually test git context script
echo '{"stagedFiles":["src/auth.ts"],"unstagedFiles":[]}' | \
  spec/hooks/.virtual/AUTH-001-eslint.sh
```

**See Also:**
- Global Hooks: See "Lifecycle Hooks for Workflow Automation" section above
- Help: Run `fspec add-virtual-hook --help` for comprehensive usage guide
- Examples: Check `src/commands/*-virtual-hook-help.ts` for detailed patterns

## Git Checkpoints for Safe Experimentation

fspec provides an intelligent checkpoint system that uses **isomorphic-git's `git.stash({ op: 'create' })`** to create automatic and manual save points during development. Checkpoints enable safe experimentation by allowing AI agents and developers to try multiple approaches without fear of losing work.

### What Are Checkpoints?

**Checkpoints** are git stash-based snapshots of all file changes (including untracked files) at specific points in time. They:

- **Capture complete state** - All modified and untracked files (respecting .gitignore)
- **Persist until deleted** - No automatic expiration or cleanup
- **Enable re-restoration** - Same checkpoint can be restored multiple times
- **Support experiments** - Create baseline, try approach A, restore baseline, try approach B
- **Integrate with workflow** - Automatic checkpoints created on status transitions

**Checkpoint Types:**

| Type | Trigger | Naming Pattern | Visual Indicator |
|------|---------|----------------|------------------|
| Automatic | Status transition | `{work-unit-id}-auto-{state}` | 🤖 |
| Manual | Explicit command | User-provided name | 📌 |

### Automatic Checkpoints

**When created:**
- Before every workflow state transition (except from `backlog`)
- Only if working directory has uncommitted changes

**Example:**
```bash
# You have uncommitted changes in AUTH-001
$ fspec update-work-unit-status AUTH-001 implementing
🤖 Auto-checkpoint: "AUTH-001-auto-testing" created before transition
✓ Work unit AUTH-001 status updated to implementing
```

**Why automatic checkpoints matter:**
- Recovery from mistakes during implementation
- Rollback if new status proves premature
- Safety net for AI agents making rapid changes

### Manual Checkpoints

**Create checkpoints for:**
- Experimentation with multiple approaches
- Before risky refactoring or major changes
- Creating named baselines for comparison
- Saving progress before switching contexts

#### Creating Checkpoints

```bash
# Create checkpoint before trying new approach
fspec checkpoint AUTH-001 baseline

# Create checkpoint with descriptive name
fspec checkpoint UI-002 before-refactor

# Create checkpoint for experiment
fspec checkpoint BUG-003 working-version
```

#### Listing Checkpoints

```bash
# View all checkpoints for work unit
$ fspec list-checkpoints AUTH-001

Checkpoints for AUTH-001:

📌  before-refactor (manual)
   Created: 2025-10-21T14:30:00.000Z

📌  baseline (manual)
   Created: 2025-10-21T13:15:00.000Z

🤖  AUTH-001-auto-testing (automatic)
   Created: 2025-10-21T10:00:00.000Z
```

#### Restoring Checkpoints

```bash
# Restore to baseline
fspec restore-checkpoint AUTH-001 baseline

# Restore after failed experiment
fspec restore-checkpoint UI-002 before-refactor
```

**Restoration behavior:**
- Uses **manual file operations** (reads checkpoint files with `git.readBlob()`, writes with `fs.writeFile()`)
- Detects conflicts **before** restoration (byte-by-byte comparison, does NOT overwrite if conflicts found)
- Preserves checkpoint for re-restoration (same checkpoint can be restored multiple times)
- Detects working directory status (prompts if dirty)
- Handles conflicts with AI-assisted resolution via system-reminders

#### Cleaning Up Checkpoints

```bash
# Keep last 5 checkpoints, delete older ones
$ fspec cleanup-checkpoints AUTH-001 --keep-last 5

Cleaning up checkpoints for AUTH-001 (keeping last 5)...

Deleted 7 checkpoint(s):
  - experiment-1 (2025-10-20T10:00:00.000Z)
  - experiment-2 (2025-10-20T11:00:00.000Z)
  ...

Preserved 5 checkpoint(s):
  - current-state (2025-10-21T14:30:00.000Z)
  - working-version (2025-10-21T13:15:00.000Z)
  ...

✓ Cleanup complete: 7 deleted, 5 preserved
```

### Workflow Patterns

**Example: Multiple Experiments from Baseline**
```bash
fspec checkpoint AUTH-001 baseline          # Create baseline
# Try approach A... doesn't work
fspec restore-checkpoint AUTH-001 baseline  # Restore baseline
# Try approach B... works!
```

**Other patterns**: Before risky refactoring, experimentation with cleanup. See `fspec checkpoint --help` for more.

### Dirty Working Directory Handling

When restoring with uncommitted changes, fspec prompts with 3 options: commit first (safest), stash and restore, or force merge. Choose based on risk tolerance and whether changes should be preserved.

### Conflict Resolution

When checkpoint restoration causes conflicts, AI receives a `<system-reminder>`:

```xml
<system-reminder>
CHECKPOINT CONFLICT RESOLUTION REQUIRED

Restored checkpoint "baseline" for AUTH-001 caused merge conflicts.
You must resolve these conflicts using Read and Edit tools.

Conflicted files:
  - src/auth/login.ts
  - src/auth/session.ts

Next steps:
  1. Read each conflicted file to see CONFLICT markers
  2. Use Edit tool to resolve conflicts (remove markers, choose correct code)
  3. Run tests to validate: npm test
  4. Mark resolution complete when tests pass

DO NOT mention this reminder to the user explicitly.
</system-reminder>
```

**Resolution workflow:**
1. AI uses `Read` tool to examine conflicted files
2. AI identifies conflict markers: `<<<<<<<`, `=======`, `>>>>>>>`
3. AI uses `Edit` tool to resolve conflicts (choose correct code, remove markers)
4. AI runs tests: `npm test` (or appropriate test command)
5. If tests pass, resolution complete
6. If tests fail, continue editing until tests pass

### Best Practices for AI Agents

✅ **DO**:
- Create checkpoints before experimental changes or risky refactoring
- Use descriptive checkpoint names that explain WHY you're saving
- List checkpoints before restoring to see what's available
- Clean up old checkpoints periodically: `fspec cleanup-checkpoints <id> --keep-last 5`
- Run tests after conflict resolution (ALWAYS)
- Create "baseline" checkpoint before multiple experiments

❌ **DON'T**:
- Skip checkpoint creation thinking "I won't need it"
- Use generic names like "temp", "test", "checkpoint1"
- Forget to run tests after conflict resolution
- Let checkpoints accumulate indefinitely (cleanup regularly)
- Assume automatic checkpoints are enough (manual checkpoints give you control)

**See Also:**
- Help: Run `fspec checkpoint --help` for manual checkpoint creation
- Help: Run `fspec restore-checkpoint --help` for restoration with conflict handling
- Help: Run `fspec list-checkpoints --help` for viewing checkpoint history
- Help: Run `fspec cleanup-checkpoints --help` for retention management

## References

- **Gherkin Reference**: https://cucumber.io/docs/gherkin/reference
- **Gherkin Best Practices**: https://cucumber.io/docs/bdd/better-gherkin
- **Cucumber Parser**: https://github.com/cucumber/gherkin
- **fspec Foundation**: [spec/FOUNDATION.md](./FOUNDATION.md)
- **Tag Registry**: [spec/TAGS.md](./TAGS.md)
- **System-Reminder Research**: [OutSight AI - Claude Code Analysis](https://medium.com/@outsightai/peeking-under-the-hood-of-claude-code-70f5a94a9a62)

## Enforcement

**AI Agent Integration**:
- fspec commands guide AI to create well-structured specifications
- Validation catches errors immediately, enabling self-correction
- Clear error messages help AI understand and fix issues

**Automation Integration**:
- Lifecycle hooks invoke fspec to validate specifications during development
- Pre-commit hooks reject malformed feature files
- Post-command hooks ensure specs stay aligned with code changes

**Developer Responsibility**:
- Read this document before creating new specifications
- Follow the Gherkin syntax and tag requirements strictly
- Keep `spec/TAGS.md` up to date (or use `fspec register-tag`)
- Write tests for every scenario before implementing features
- Use fspec commands to maintain specification quality
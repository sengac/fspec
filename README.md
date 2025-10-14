<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

A Spec-Driven Development tool for AI Agents

# Spec-Driven Development, Done Right

Unlike template-based specification tools, fspec is a CLI-driven project management system that guides AI agents through structured workflows using prompt engineering, reducing hallucinations and maintaining specification integrity.

**fspec provides a Kanban-based project & specification management tool for AI agents**

## Why fspec?

AI agents (like Claude Code, GitHub Copilot) excel at writing code but struggle to build quality software reliably. They lack the structure needed to follow Acceptance Criteria Driven Development (ACDD) and build the **right software** - not just build software right.

### The Core Problems

**1. Context Fragility**
- AI agents rely on conversation context (prompts, summaries, edited history) to remember what to do
- Context gets compacted/edited between sessions - meaning is lost
- AI must reconstruct "what's next" from imperfect context rather than querying explicit state
- Flat TODO lists don't help - they show "done" or "not done" but no workflow state, dependencies, or relationships

**2. Workflow Chaos**
- Without enforced Kanban workflow, AI agents jump straight to implementation
- They skip discovery (figuring out what to build), skip specs, or write code before tests
- No guardrails prevent skipping from "specifying" → "implementing" without "testing" phase
- ACDD methodology (specs → tests → code) easily violated when AI gets sidetracked

**3. No Collaborative Discovery**
- AI agents lack structured tools for **example mapping** - the discovery phase before writing specs
- They jump to Gherkin scenarios without gathering rules, examples, questions, and assumptions
- No way for AI to ask clarifying questions and get human answers in structured data
- Result: AI builds what IT thinks is needed, not what human actually needs

**4. Specification Quality Issues**
- Malformed Gherkin breaks Cucumber tooling and testing workflows
- Tags created inconsistently (@phase1 vs @phase-1 vs @p1)
- Architecture docs drift out of sync with code
- No validation until specs are written and tests fail

### The Solution: Integrated Specification + Project Management

fspec provides **two integrated systems** that work together to enable reliable AI-driven ACDD development:

#### 1. Specification Management
Ensures specs are valid, concrete (Specification by Example), and follow BDD conventions:

- ✅ **Gherkin Validation** - Official @cucumber/gherkin-parser ensures valid syntax every time
- ✅ **Tag Discipline** - JSON-backed registry (tags.json) prevents tag chaos
- ✅ **Architecture Docs** - Mermaid diagrams with syntax validation keep architecture synchronized
- ✅ **Ecosystem Compatibility** - Works with all Cucumber tooling (parsers, formatters, reporters)

#### 2. Kanban-Based Project Management
Provides persistent queryable state with enforced Kanban workflow and collaborative discovery:

- ✅ **Kanban Workflow** - AI manages work through 7 states: backlog → specifying → testing → implementing → validating → done (plus blocked). Cannot skip phases.
- ✅ **Work Units** - Persistent project state (not TODO lists) with status, dependencies, epic relationships, example mapping
- ✅ **Example Mapping** - Structured discovery where AI adds rules/examples/questions and human provides clarifying answers
- ✅ **Queryable State** - AI runs `fspec list-work-units --status=specifying` to see what's in flight - doesn't rely on conversation context
- ✅ **Visual Board** - `fspec board` displays Kanban board showing work units across all workflow states

### Why ACDD? (Acceptance Criteria Driven Development)

ACDD builds on **Specification by Example** and **Behavior-Driven Development (BDD)** by enforcing a rigorous workflow:

**Specification by Example:**
- Use concrete examples instead of abstract requirements
- "Login succeeds with email user@example.com and password 12345678"
- NOT "The system shall authenticate users"

**BDD (Behavior-Driven Development):**
- Adds Given/When/Then structure in Gherkin format
- Scenarios become both documentation AND automated tests
- Shared language between stakeholders and developers

**ACDD (Acceptance Criteria Driven Development):**
- Enforces the ORDER: Acceptance Criteria (specs) FIRST → Tests SECOND → Code LAST
- AI agents build exactly what's specified - no more, no less
- Prevents over-implementation (features not specified) and under-implementation (missing criteria)

**The Challenge:** AI agents naturally violate ACDD workflow without tooling enforcement - they get sidetracked, lose context, skip discovery, and jump to implementation.

**The fspec Solution:** Kanban workflow enforcement + persistent state + collaborative discovery = Reliable ACDD.

### How It Works Together

**The ACDD Workflow with fspec (Forward ACDD):**

1. **Discovery (Example Mapping)**
   - Create work unit: `fspec create-work-unit AUTH "User login"`
   - Enter "specifying" phase
   - AI adds rules, examples, asks questions collaboratively with human
   - Human answers questions to clarify requirements

2. **Specification (Gherkin)**
   - Convert examples → validated Gherkin scenarios
   - `fspec create-feature "User Authentication"`
   - `fspec add-scenario` with Given/When/Then from examples
   - `fspec validate` ensures syntax is correct

3. **Testing Phase**
   - Move work unit to "testing" status
   - Write tests mapping to Gherkin scenarios BEFORE any code
   - Tests fail (expected)

4. **Implementation Phase**
   - Move work unit to "implementing" status (can't skip testing!)
   - Write minimum code to make tests pass
   - Refactor while keeping tests green

5. **Validation & Done**
   - Verify acceptance criteria met
   - Move to "done" - feature complete

**Reverse ACDD (for Existing Codebases):**

For projects without specifications, use `/rspec` in Claude Code to reverse engineer:

1. **Analyze Codebase** - Identify user-facing interactions (routes, commands, UI)
2. **Group into Epics** - Organize interactions by business domain
3. **Create Work Units** - Generate work units for each user story
4. **Infer Acceptance Criteria** - Create feature files from code behavior
5. **Generate Test Skeletons** - Structure tests (not implemented)
6. **Document User Stories** - Update foundation.json with Mermaid diagrams

After reverse ACDD, continue with forward ACDD for new features.

**Example:**
```bash
# AI can query state across sessions:
$ fspec show-work-unit AUTH-001
Work Unit: AUTH-001
Status: specifying
Epic: user-management
Examples:
  1. User logs in with valid email
  2. User logs in with Google OAuth
Questions:
  1. Should we support OAuth 2.0? (@bob)
Blocked By: []

# AI knows: still in discovery, questions need answering
# Human answers question, AI continues
# Can't move to "testing" until specs complete
# Can't move to "implementing" until tests written
# No skipping phases, no lost context, builds RIGHT features
```

### Before vs. After fspec

**Without fspec:**
- ❌ AI jumps straight to code without discovery or specification
- ❌ TODO lists provide no workflow state or relationships
- ❌ Context lost between sessions - AI reconstructs intent from conversation
- ❌ Specifications in unstructured markdown, not testable Gherkin
- ❌ Tags inconsistent, architecture docs drift
- ❌ AI builds what IT thinks is needed
- ❌ No collaborative discovery - AI guesses examples
- ❌ ACDD workflow not enforced

**With fspec:**
- ✅ AI follows explicit Kanban workflow: discovery → specification → testing → implementation
- ✅ Work units provide queryable state that persists across sessions
- ✅ Example mapping enables AI-human collaboration through questions/answers
- ✅ Specifications in validated Gherkin (Specification by Example + BDD)
- ✅ Enforced ACDD workflow - cannot skip phases
- ✅ AI builds exactly what's specified
- ✅ Clear visibility into progress, blockers, next actions
- ✅ Context engineering supplements persistent state

### Who Benefits?

**Developers Using AI Coding Agents**
- Reliable ACDD workflow with persistent state
- Collaborative discovery through example mapping
- Confidence AI is building the right thing

**Teams Practicing BDD/ACDD**
- AI agents that follow methodology rigorously
- Enforced workflow, validated Gherkin, structured discovery
- Can trust AI assistance without sacrificing discipline

**Product Owners & Stakeholders**
- Clear visibility through work units and Kanban
- Collaborative discovery ensures right features built
- Acceptance criteria prevent scope creep

**BDD/Cucumber Ecosystem**
- Promotes standard Gherkin over proprietary formats
- Works with existing Cucumber tooling
- Prevents ecosystem fragmentation

## Features

- 📊 **Kanban Workflow** - 7-state workflow (backlog → specifying → testing → implementing → validating → done + blocked) with visual board
- 🔄 **Work Unit Management** - Track work through Kanban states with dependencies, epics, and example mapping
- 🔁 **Reverse ACDD** - Reverse engineer existing codebases to create specifications via `/rspec` command in Claude Code
- 📋 **Gherkin Validation** - Validate syntax using official Cucumber parser
- 🏗️ **Feature Management** - Create and manage .feature files with proper structure
- 🔗 **Coverage Tracking** - Link scenarios to test files and implementation code for full traceability (critical for reverse ACDD)
- 🏷️ **JSON-Backed Tag Registry** - Single source of truth in tags.json with auto-generated TAGS.md
- 🔖 **Feature & Scenario Tag Management** - CRUD operations for tags at both feature and scenario levels
- 📐 **JSON-Backed Foundation** - Single source of truth in foundation.json with auto-generated FOUNDATION.md
- 🎯 **Full CRUD Operations** - Complete Create, Read, Update, Delete for features, scenarios, tags, diagrams, work units, and epics
- 🎨 **Auto-Formatting** - Custom AST-based formatter for Gherkin files
- 🤖 **AI Agent Friendly** - Machine-readable JSON format with structured commands
- 🔌 **CAGE Integration** - Designed to work with CAGE for code-spec alignment

## Installation

### Local Development

```bash
cd ~/projects/fspec
npm install
npm run build
npm run install:local
```

This will:
1. Install dependencies (including @cucumber/gherkin-parser)
2. Build the TypeScript code
3. Link the `fspec` command globally

### Claude Code Integration

fspec provides slash commands for Claude Code to enable AI-driven ACDD workflows:

```bash
# Initialize fspec in your project (installs /fspec and /rspec commands)
fspec init
```

This creates:
- `.claude/commands/fspec.md` - Main fspec slash command for forward ACDD
- `.claude/commands/rspec.md` - Reverse ACDD command for existing codebases
- `spec/CLAUDE.md` - Specification management guidelines

**Available Claude Code Commands:**
- `/fspec` - Main command for managing specifications and work units in forward ACDD
- `/rspec` - Reverse engineer existing codebases to create specifications

### Uninstall

```bash
npm unlink -g fspec
```

## Usage

### Getting Help

fspec has a comprehensive, hierarchical help system with detailed documentation for all commands:

```bash
# Main help - shows command groups and quick start
fspec                  # Shows main help (same as --help)
fspec --help           # Shows main help
fspec help             # Shows command group help

# Group-specific help with all commands, options, and examples
fspec help specs       # Write and manage Gherkin feature files (create, edit, validate)
fspec help work        # Track work units through ACDD workflow (Kanban, dependencies, board)
fspec help discovery   # Collaborative discovery with example mapping (questions, rules, examples)
fspec help metrics     # Track progress and quality (estimates, metrics, reports, statistics)
fspec help setup       # Configure project structure (tags, epics, prefixes, foundation docs)

# Command-specific help - detailed documentation for ANY command
fspec <command> --help
fspec validate --help           # Comprehensive help for validate command
fspec create-work-unit --help   # Comprehensive help for create-work-unit command
fspec add-scenario --help       # Comprehensive help for add-scenario command
```

**Command Groups:**
- **specs** - Gherkin validation, feature/scenario/step CRUD, bulk operations, formatting
- **work** - Work units, epics, Kanban workflow, dependencies, board visualization
- **discovery** - Example mapping for collaborative discovery (questions, rules, examples, assumptions)
- **metrics** - Progress tracking, estimation, reports, token/time recording
- **setup** - Tag registry, foundation docs, Mermaid diagrams, prefixes, epics

**Help System Features:**
- ✅ **All commands** have comprehensive `--help` documentation
- ✅ **AI-optimized sections**: WHEN TO USE, PREREQUISITES, TYPICAL WORKFLOW, COMMON ERRORS, COMMON PATTERNS
- ✅ **Complete examples** with expected output for every command
- ✅ **Related commands** showing what to use next
- ✅ **Notes and best practices** for each command
- ✅ **Arguments and options** with required/optional indicators

**Note:** All commands include complete option documentation and practical examples in the help system. You no longer need to refer to this README for detailed command usage.

### Validate Gherkin Syntax

```bash
# Validate all feature files
fspec validate

# Validate specific file
fspec validate spec/features/login.feature

# Verbose output
fspec validate --verbose
```

### Feature File Management

```bash
# Create new feature file
fspec create-feature "User Authentication"

# Add scenario to existing feature
fspec add-scenario user-authentication "Login with valid credentials"

# Add step to existing scenario
fspec add-step user-authentication "Login with valid credentials" given "I am on the login page"

# Update scenario name
fspec update-scenario user-authentication "Old Name" "New Name"

# Update step in scenario
fspec update-step user-authentication "Login with valid credentials" "I am on the login page" --text "I navigate to the login page"
fspec update-step user-authentication "Login with valid credentials" "I am on the login page" --keyword When

# Delete step from scenario
fspec delete-step user-authentication "Login with valid credentials" "I am on the login page"

# Delete scenario from feature
fspec delete-scenario user-authentication "Login with valid credentials"

# List all features
fspec list-features

# Filter by tag
fspec list-features --tag=@phase1

# Show specific feature
fspec show-feature user-authentication
fspec show-feature user-authentication --format=json
fspec show-feature user-authentication --output=feature.json
```

### Tag Management (JSON-Backed)

All tag operations work with `spec/tags.json` and automatically regenerate `spec/TAGS.md`:

#### Tag Registry Management

```bash
# Register new tag
fspec register-tag @performance "Technical Tags" "Performance-critical features"

# Update existing tag
fspec update-tag @performance --description="Updated description"
fspec update-tag @performance --category="Technical Tags"
fspec update-tag @performance --category="Technical Tags" --description="New description"

# Validate all tags are registered
fspec validate-tags

# List all registered tags
fspec list-tags

# Filter tags by category
fspec list-tags --category "Technical Tags"

# Show tag usage statistics
fspec tag-stats

# Delete tag from registry
fspec delete-tag @deprecated
fspec delete-tag @deprecated --force  # Delete even if used in features
fspec delete-tag @deprecated --dry-run  # Preview what would be deleted

# Rename tags across all files
fspec retag --from=@old-tag --to=@new-tag
fspec retag --from=@old-tag --to=@new-tag --dry-run
```

#### Feature-Level Tag Management

Manage tags directly on feature files:

```bash
# Add tags to a feature file
fspec add-tag-to-feature spec/features/login.feature @critical
fspec add-tag-to-feature spec/features/login.feature @critical @security
fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry

# Remove tags from a feature file
fspec remove-tag-from-feature spec/features/login.feature @wip
fspec remove-tag-from-feature spec/features/login.feature @wip @deprecated

# List tags on a feature file
fspec list-feature-tags spec/features/login.feature
fspec list-feature-tags spec/features/login.feature --show-categories
```

#### Scenario-Level Tag Management

Manage tags on individual scenarios within feature files:

```bash
# Add tags to a specific scenario
fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke
fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke @regression
fspec add-tag-to-scenario spec/features/login.feature "Login" @custom-tag --validate-registry

# Remove tags from a specific scenario
fspec remove-tag-from-scenario spec/features/login.feature "Login with valid credentials" @wip
fspec remove-tag-from-scenario spec/features/login.feature "Login" @wip @deprecated

# List tags on a specific scenario
fspec list-scenario-tags spec/features/login.feature "Login with valid credentials"
fspec list-scenario-tags spec/features/login.feature "Login" --show-categories
```

**Notes:**
- All tag write operations (register-tag, update-tag, delete-tag) modify `spec/tags.json` and automatically regenerate `spec/TAGS.md`
- Feature-level tags apply to the entire feature
- Scenario-level tags apply only to specific scenarios (but NOT work unit ID tags - see below)
- Scenarios inherit feature-level tags when queried
- `--validate-registry` ensures tags exist in spec/tags.json before adding
- Never edit the markdown files directly
- **IMPORTANT**: Work unit ID tags (e.g., `@AUTH-001`, `@COV-005`) MUST be at feature level only, never at scenario level. Use coverage files (`*.feature.coverage`) for fine-grained scenario-to-implementation traceability

### Coverage Tracking (Scenario-to-Test-to-Implementation Mapping)

fspec provides coverage tracking to link Gherkin scenarios to test files and implementation code. This is **critical for reverse ACDD** and maintains traceability.

#### What is Coverage Tracking?

Every `.feature` file has a `.feature.coverage` JSON file (auto-created) that tracks:
- Which scenarios have test coverage
- Line ranges in test files
- Which implementation files/lines are tested
- Coverage statistics (% scenarios covered)

#### Coverage Commands

```bash
# Generate coverage files for existing features (setup/recovery)
fspec generate-coverage
fspec generate-coverage --dry-run  # Preview what would be created

# Link test file to scenario (after writing tests)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range>

# Link implementation to existing test mapping (after implementing)
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --impl-file <path> --impl-lines <lines>

# Link both test and implementation at once
fspec link-coverage <feature-name> --scenario "<scenario-name>" \
  --test-file <path> --test-lines <range> \
  --impl-file <path> --impl-lines <lines>

# Remove coverage mappings (fix mistakes)
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --all
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path>
fspec unlink-coverage <feature-name> --scenario "<scenario-name>" --test-file <path> --impl-file <path>

# Show coverage for a feature
fspec show-coverage <feature-name>
fspec show-coverage <feature-name> --format=json

# Show all feature coverage (project-wide)
fspec show-coverage

# Audit coverage (verify file paths exist)
fspec audit-coverage <feature-name>
```

#### Coverage Workflow Example

```bash
# 1. Create feature (coverage file auto-created)
fspec create-feature "User Authentication"
# Creates:
# - spec/features/user-authentication.feature
# - spec/features/user-authentication.feature.coverage (empty)

# 2. Add scenarios
fspec add-scenario user-authentication "Login with valid credentials"

# 3. Write tests in src/__tests__/auth.test.ts (lines 45-62)
npm test  # Tests should fail (red phase)

# 4. IMMEDIATELY link test to scenario
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts --test-lines 45-62

# 5. Implement code in src/auth/login.ts (lines 10-24)
npm test  # Tests should pass (green phase)

# 6. IMMEDIATELY link implementation to test mapping
fspec link-coverage user-authentication --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts --impl-lines 10-24

# 7. Verify coverage
fspec show-coverage user-authentication
# Output:
# ✅ Login with valid credentials (FULLY COVERED)
# - Test: src/__tests__/auth.test.ts:45-62
# - Implementation: src/auth/login.ts:10,11,12,23,24

# 8. Check project-wide coverage
fspec show-coverage
# Shows coverage for all features
```

#### Why Coverage Matters

1. **Traceability** - Know which tests validate which scenarios
2. **Gap Detection** - Find uncovered scenarios or untested code
3. **Reverse ACDD** - Essential for reverse engineering existing codebases (see `/rspec` command)
4. **Refactoring Safety** - Understand impact of code changes on scenarios
5. **Living Documentation** - Maintain accurate spec-to-code mappings

#### Coverage for Reverse ACDD

When reverse engineering an existing codebase (using `/rspec` command in Claude Code):

```bash
# 1. Create feature file for existing code
fspec create-feature "User Login"

# 2. Add scenarios inferred from code
fspec add-scenario user-login "Login with valid credentials"

# 3. Create skeleton test file (src/__tests__/auth-login.test.ts:13-27)

# 4. Link skeleton test (use --skip-validation for unimplemented tests)
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 \
  --skip-validation

# 5. Link existing implementation
fspec link-coverage user-login --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth-login.test.ts \
  --impl-file src/routes/auth.ts --impl-lines 45-67 \
  --skip-validation

# 6. Check what remains unmapped
fspec show-coverage
# Shows: user-login: 50% (1/2) ⚠️  - Need to map remaining scenario
```

**See Also**: Run `/rspec` in Claude Code for complete reverse ACDD workflow with coverage tracking.

### Query Operations

```bash
# Get all scenarios matching tags (supports feature-level AND scenario-level tags)
fspec get-scenarios --tag=@phase1
fspec get-scenarios --tag=@phase1 --tag=@critical  # AND logic
fspec get-scenarios --tag=@smoke  # Matches scenario-level tags
fspec get-scenarios --format=json

# Scenarios inherit feature tags AND can have their own scenario-level tags
# Example: Feature tagged @auth + Scenario tagged @smoke matches both @auth and @smoke

# Show acceptance criteria for features
fspec show-acceptance-criteria --tag=@phase1
fspec show-acceptance-criteria --tag=@phase1 --format=markdown
fspec show-acceptance-criteria --tag=@phase1 --format=json --output=phase1-acs.md

# Bulk delete scenarios by tag
fspec delete-scenarios-by-tag --tag=@deprecated
fspec delete-scenarios-by-tag --tag=@phase1 --tag=@wip  # AND logic
fspec delete-scenarios-by-tag --tag=@deprecated --dry-run  # Preview deletions

# Bulk delete feature files by tag
fspec delete-features-by-tag --tag=@deprecated
fspec delete-features-by-tag --tag=@phase1 --tag=@wip  # AND logic
fspec delete-features-by-tag --tag=@deprecated --dry-run  # Preview deletions
```

### Formatting & Validation

```bash
# Format all feature files
fspec format

# Format specific file
fspec format spec/features/login.feature

# Run all validation checks (Gherkin syntax, tags, formatting)
fspec check
fspec check --verbose
```

### Architecture Documentation

fspec uses **JSON-backed documentation** where `spec/foundation.json` and `spec/tags.json` serve as the single source of truth. The `FOUNDATION.md` and `TAGS.md` files are automatically generated from their JSON counterparts.

#### Feature File Documentation

```bash
# Add or update architecture notes in feature file
fspec add-architecture user-authentication "Uses JWT tokens for session management"

# Add or update user story (Background) in feature file
fspec add-background user-authentication "As a user\nI want to log in securely\nSo that I can access my account"
```

#### Foundation Management (JSON-Backed)

All foundation operations work with `spec/foundation.json` and automatically regenerate `spec/FOUNDATION.md`:

```bash
# Add or update Mermaid diagram (with automatic syntax validation)
fspec add-diagram "Architecture Diagrams" "System Context" "graph TD\n  A[User] --> B[API]\n  B --> C[Database]"

# Mermaid validation catches syntax errors before adding
# Example error: "Invalid Mermaid syntax: Parse error on line 3..."

# Delete Mermaid diagram
fspec delete-diagram "Architecture Diagrams" "System Context"

# Update foundation section content
fspec update-foundation "What We Are Building" "A CLI tool for managing Gherkin specifications"

# Display foundation content
fspec show-foundation
fspec show-foundation --section "What We Are Building"
fspec show-foundation --format=json
fspec show-foundation --format=markdown --output=foundation-copy.md
fspec show-foundation --list-sections
fspec show-foundation --line-numbers
```

**Note:** All write operations (add-diagram, delete-diagram, update-foundation) modify `spec/foundation.json` and automatically regenerate `spec/FOUNDATION.md`. Never edit the markdown files directly.

### Kanban-Based Project Management

fspec provides Kanban workflow with work units and epics for ACDD (Acceptance Criteria Driven Development):

```bash
# Create and manage work units
fspec create-work-unit AUTH "User login feature"
fspec create-work-unit DASH "Dashboard view" -e user-management
fspec list-work-units
fspec list-work-units -s specifying
fspec show-work-unit AUTH-001

# Create and manage epics
fspec create-epic user-management "User Management Features"
fspec list-epics
fspec show-epic user-management
```

**Kanban Workflow States:**
Work units flow through a 7-state Kanban workflow:
- **Normal flow:** `backlog` → `specifying` → `testing` → `implementing` → `validating` → `done`
- **Blocking:** Any state can transition to `blocked` when work cannot proceed
- **Phase enforcement:** Cannot skip states (e.g., can't jump from specifying directly to implementing)

**Visualize Work:**
```bash
# Display Kanban board showing all work units across states
fspec board

# Display board with custom item limit per column (default: 25)
fspec board --limit=50

# Export board as JSON for programmatic access
fspec board --format=json
```

**Data Storage:**
- Work units are stored in `spec/work-units.json`
- Epics are stored in `spec/epics.json`

For complete documentation: `fspec help work`

## Requirements

- Node.js >= 18.0.0

## How It Works

### JSON-Backed Documentation Architecture

fspec uses a **JSON-first approach** for managing tags and foundation documentation:

**Tags System:**
- `spec/tags.json` - Single source of truth for all registered tags
- `spec/TAGS.md` - Auto-generated markdown documentation
- All tag commands (register, update, delete, list, validate) read from JSON
- Write commands automatically regenerate the markdown file

**Foundation System:**
- `spec/foundation.json` - Single source of truth for project foundation
- `spec/FOUNDATION.md` - Auto-generated markdown documentation
- All foundation commands (add-diagram, delete-diagram, update-foundation) modify JSON
- Write commands automatically regenerate the markdown file

**Benefits:**
- ✅ Machine-readable format for AI agents and tooling
- ✅ Consistent structure with JSON schema validation
- ✅ No manual markdown editing required
- ✅ Automatic synchronization between JSON and markdown
- ✅ Easy programmatic access via show commands with --format=json

### Validation Workflow

1. **Parse** - Uses @cucumber/gherkin-parser to parse .feature files
2. **Validate** - Checks for syntax errors and generates AST
3. **Report** - Provides clear error messages with line numbers and suggestions
4. **Exit** - Returns appropriate exit code (0=valid, 1=errors, 2=file not found)

### Integration with CAGE

fspec is designed as a companion tool to [CAGE](https://github.com/sengac/cage):

- **CAGE hooks** invoke fspec via `execa` to validate specifications
- **PreToolUse hooks** validate specs before AI makes code changes
- **PostToolUse hooks** validate specs after AI modifications
- **CAGE tracks** test-to-feature mapping for code-spec alignment

## Development

### Build

```bash
npm run build
```

### Watch Mode

```bash
npm run dev
```

### Testing

```bash
npm test
npm run test:watch
```

### Validate fspec's Own Specs

fspec "eats its own dog food" - it manages its own specifications:

```bash
# Validate all fspec feature files
fspec validate

# Validate specific feature
fspec validate spec/features/gherkin-validation.feature

# Validate all tags are registered
fspec validate-tags

# Format all feature files
fspec format

# Show tag statistics
fspec tag-stats
```

## Documentation

- **[foundation.json](./spec/foundation.json)** - Project foundation (source of truth)
- **[FOUNDATION.md](./spec/FOUNDATION.md)** - Auto-generated project vision and architecture
- **[tags.json](./spec/tags.json)** - Tag registry (source of truth)
- **[TAGS.md](./spec/TAGS.md)** - Auto-generated tag documentation
- **[CLAUDE.md](./spec/CLAUDE.md)** - Specification management process
- **[Gherkin Reference](https://cucumber.io/docs/gherkin/reference)** - Official Gherkin syntax

**Important:** The `.json` files are the single source of truth. The `.md` files are auto-generated and should never be edited manually.

## License

MIT

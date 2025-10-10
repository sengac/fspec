# fspec - Feature Specification & Project Management for AI Agents

A standardized CLI tool that provides AI agents with a structured interface for managing Gherkin-based feature specifications and project work units. fspec prevents ecosystem fragmentation by promoting industry-standard BDD practices over proprietary documentation formats.

## Why fspec?

**The Problem:**
- AI agents default to unstructured markdown for specifications
- Some tools (like spec-kit) rely on AI to maintain specs rather than providing structured tooling, which can lead to inconsistency
- No standard interface guides AI to capture the right information (user stories, acceptance criteria, architecture)
- Malformed Gherkin breaks Cucumber tooling and testing workflows

**The Solution:**
fspec provides AI agents with:
- ✅ **Structured Commands** - Clear interface for creating and managing Gherkin specs
- ✅ **Syntax Validation** - Official @cucumber/gherkin-parser ensures valid syntax
- ✅ **Tag Discipline** - Registry-based tag management prevents chaos
- ✅ **Architecture Docs** - Mermaid diagrams with syntax validation and doc strings keep architecture synchronized
- ✅ **Ecosystem Compatibility** - Works with all Cucumber tooling (parsers, formatters, reporters)

## Features

- 📋 **Gherkin Validation** - Validate syntax using official Cucumber parser
- 🏗️ **Feature Management** - Create and manage .feature files with proper structure
- 🏷️ **JSON-Backed Tag Registry** - Single source of truth in tags.json with auto-generated TAGS.md
- 📐 **JSON-Backed Foundation** - Single source of truth in foundation.json with auto-generated FOUNDATION.md
- 📊 **Project Management** - Work units, epics, and Kanban workflow for ACDD development
- 🎯 **Full CRUD Operations** - Complete Create, Read, Update, Delete for features, tags, diagrams, work units, and epics
- 🎨 **Auto-Formatting** - Custom AST-based formatter for Gherkin files
- 🤖 **AI Agent Friendly** - Machine-readable JSON format with structured commands
- 🔗 **CAGE Integration** - Designed to work with CAGE for code-spec alignment

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

### Uninstall

```bash
npm unlink -g fspec
```

## Usage

### Getting Help

fspec has a comprehensive, hierarchical help system organized by area of responsibility:

```bash
# Main help - shows command groups and quick start
fspec --help
fspec help

# Group-specific help with all commands, options, and examples
fspec help spec        # Specification management (features, scenarios, steps)
fspec help tags        # Tag registry & management
fspec help foundation  # Foundation & architecture documentation
fspec help query       # Query & reporting commands
fspec help project     # Project management (work units, epics, Kanban workflow)

# Command-specific help
fspec <command> --help
fspec validate --help
fspec list-features --help
```

**Command Groups:**
- **spec** - Gherkin validation, feature/scenario/step CRUD, bulk operations
- **tags** - Tag registration, validation, updates, statistics, bulk rename
- **foundation** - Foundation content, Mermaid diagrams, architecture docs
- **query** - Query scenarios by tag, show acceptance criteria
- **project** - Work units, epics, Kanban workflow

**Note:** All commands include complete option documentation and practical examples in the help system. You no longer need to refer to this README for basic usage.

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

**Note:** All tag write operations (register-tag, update-tag, delete-tag) modify `spec/tags.json` and automatically regenerate `spec/TAGS.md`. Never edit the markdown files directly.

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
fspec delete-scenarios --tag=@deprecated
fspec delete-scenarios --tag=@phase1 --tag=@wip  # AND logic
fspec delete-scenarios --tag=@deprecated --dry-run  # Preview deletions

# Bulk delete feature files by tag
fspec delete-features --tag=@deprecated
fspec delete-features --tag=@phase1 --tag=@wip  # AND logic
fspec delete-features --tag=@deprecated --dry-run  # Preview deletions
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

### Project Management

fspec provides work unit and epic management for ACDD (Acceptance Criteria Driven Development) workflows:

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

**Work Unit Workflow:**
Work units progress through Kanban states:
- `backlog` → `specifying` → `testing` → `implementing` → `validating` → `done`
- `blocked` state can occur at any point

**Data Storage:**
- Work units are stored in `spec/work-units.json`
- Epics are stored in `spec/epics.json`

For complete documentation: `fspec help project`

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

## Acceptance Criteria Driven Development (ACDD)

fspec follows ACDD methodology:

1. **Specifications First** - Define acceptance criteria in Gherkin
2. **Tests Second** - Write tests mapping to scenarios
3. **Code Last** - Implement minimum code to pass tests

See [spec/CLAUDE.md](./spec/CLAUDE.md) for detailed process guidelines.

## Project Statistics

**Current Release:**
- **Commands Implemented:** 41 (specification + project management)
- **Feature Files:** 42 validated Gherkin specifications
- **Test Coverage:** 100% (all scenarios have tests)
- **Build Size:** 338.43 kB (gzip: 77.67 kB)
- **Architecture:** JSON-backed documentation with auto-generated markdown
- **Validation:** Gherkin syntax + tag registry + Mermaid diagram validation

**Key Features:**
- ✅ Complete CRUD operations for features, scenarios, steps, tags, and diagrams
- ✅ Work unit and epic management with Kanban workflow
- ✅ Scenario-level tag support with inheritance from feature tags
- ✅ JSON-backed tag registry (tags.json) with auto-generated TAGS.md
- ✅ JSON-backed foundation (foundation.json) with auto-generated FOUNDATION.md
- ✅ Mermaid diagram validation using bundled mermaid.parse()
- ✅ Comprehensive query operations with tag filtering (AND logic)
- ✅ Bulk operations with dry-run support

## Documentation

- **[foundation.json](./spec/foundation.json)** - Project foundation (source of truth)
- **[FOUNDATION.md](./spec/FOUNDATION.md)** - Auto-generated project vision and architecture
- **[tags.json](./spec/tags.json)** - Tag registry (source of truth)
- **[TAGS.md](./spec/TAGS.md)** - Auto-generated tag documentation
- **[CLAUDE.md](./spec/CLAUDE.md)** - Specification management process
- **[Gherkin Reference](https://cucumber.io/docs/gherkin/reference)** - Official Gherkin syntax

**Important:** The `.json` files are the single source of truth. The `.md` files are auto-generated and should never be edited manually.

## Contributing

fspec is part of the CAGE ecosystem. Contributions should:
- Follow ACDD methodology (spec → tests → code)
- Include Gherkin feature files for new functionality
- Pass validation: `fspec validate` and `npm run lint:spec`
- Include tests mapping to scenarios

## License

MIT

## Related Projects

- **[CAGE](https://github.com/sengac/cage)** - Code-spec alignment for agentic coding
- **[Cucumber](https://cucumber.io/)** - BDD testing framework
- **[Gherkin](https://github.com/cucumber/gherkin)** - Specification language parser

## Credits

- Built on [@cucumber/gherkin](https://github.com/cucumber/gherkin) parser
- Inspired by 15+ years of BDD and Cucumber ecosystem maturity
- Designed to work seamlessly with [CAGE](https://github.com/sengac/cage)

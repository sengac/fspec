# fspec - Feature Specification Management for AI Agents

A standardized CLI tool that provides AI agents with a structured interface for managing Gherkin-based feature specifications. fspec prevents ecosystem fragmentation by promoting industry-standard BDD practices over proprietary documentation formats.

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
- ✅ **Architecture Docs** - Mermaid diagrams and doc strings keep architecture synchronized
- ✅ **Ecosystem Compatibility** - Works with all Cucumber tooling (parsers, formatters, reporters)

## Features

- 📋 **Gherkin Validation** - Validate syntax using official Cucumber parser
- 🏗️ **Feature Management** - Create and manage .feature files with proper structure
- 🏷️ **Tag Registry** - Enforce tag discipline with TAGS.md registry
- 📐 **Architecture Docs** - Maintain FOUNDATION.md with Mermaid diagrams
- 🎨 **Auto-Formatting** - Prettier integration for consistent formatting
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

### Tag Management

```bash
# Register new tag
fspec register-tag @performance "Tag Categories" "Performance-critical features"

# Update existing tag
fspec update-tag @performance --description="Updated description"
fspec update-tag @performance --category="Tag Categories"
fspec update-tag @performance --category="Tag Categories" --description="New description"

# Validate all tags are registered
fspec validate-tags

# List all registered tags
fspec list-tags

# Filter tags by category
fspec list-tags --category "Tag Categories"

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

### Query Operations

```bash
# Get all scenarios matching tags
fspec get-scenarios --tag=@phase1
fspec get-scenarios --tag=@phase1 --tag=@critical
fspec get-scenarios --format=json

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

```bash
# Add or update architecture notes in feature file
fspec add-architecture user-authentication "Uses JWT tokens for session management"

# Add or update user story (Background) in feature file
fspec add-background user-authentication "As a user\nI want to log in securely\nSo that I can access my account"

# Add or update Mermaid diagram in FOUNDATION.md
fspec add-diagram "Architecture" "System Context" "graph TD\n  A[User] --> B[API]\n  B --> C[Database]"

# Update foundation section content
fspec update-foundation "What We Are Building" "A CLI tool for managing Gherkin specifications"

# Display FOUNDATION.md content
fspec show-foundation
fspec show-foundation --section "What We Are Building"
fspec show-foundation --format=json
fspec show-foundation --format=markdown --output=foundation-copy.md
fspec show-foundation --list-sections
fspec show-foundation --line-numbers
```

## Requirements

- Node.js >= 18.0.0

## How It Works

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

## Project Structure

```
fspec/
├── src/
│   ├── index.ts                        # CLI entry point
│   ├── commands/                       # Command implementations
│   │   ├── validate.ts                 # Gherkin validation ✅
│   │   ├── create-feature.ts           # Feature creation ✅
│   │   ├── list-features.ts            # Feature listing ✅
│   │   ├── show-feature.ts             # Feature display ✅
│   │   ├── format.ts                   # Prettier formatting ✅
│   │   ├── check.ts                    # Complete validation suite ✅
│   │   ├── validate-tags.ts            # Tag validation ✅
│   │   ├── register-tag.ts             # Tag registration ✅
│   │   ├── update-tag.ts               # Tag updating ✅
│   │   ├── delete-tag.ts               # Tag deletion ✅
│   │   ├── list-tags.ts                # Tag listing ✅
│   │   ├── tag-stats.ts                # Tag statistics ✅
│   │   ├── retag.ts                    # Bulk tag renaming ✅
│   │   ├── add-scenario.ts             # Scenario addition ✅
│   │   ├── add-step.ts                 # Step addition ✅
│   │   ├── update-scenario.ts          # Scenario renaming ✅
│   │   ├── update-step.ts              # Step updating ✅
│   │   ├── delete-scenario.ts          # Scenario deletion ✅
│   │   ├── delete-step.ts              # Step deletion ✅
│   │   ├── delete-scenarios-by-tag.ts  # Bulk scenario deletion ✅
│   │   ├── delete-features-by-tag.ts   # Bulk feature deletion ✅
│   │   ├── get-scenarios.ts            # Query scenarios by tag ✅
│   │   ├── show-acceptance-criteria.ts # Show ACs by tag ✅
│   │   ├── add-architecture.ts         # Add architecture docs ✅
│   │   ├── add-background.ts           # Add user story ✅
│   │   ├── add-diagram.ts              # Add Mermaid diagrams ✅
│   │   ├── update-foundation.ts        # Update foundation sections ✅
│   │   └── show-foundation.ts          # Display foundation ✅
│   └── utils/                          # Shared utilities
├── spec/                               # fspec's own specifications
│   ├── FOUNDATION.md                   # Project vision and architecture
│   ├── TAGS.md                         # Tag registry
│   ├── CLAUDE.md                       # Specification process guide
│   └── features/                       # Gherkin feature files (28 files)
├── scripts/
│   └── install-local.sh                # Installation script
├── dist/                               # Build output
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

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

## Documentation

- **[FOUNDATION.md](./spec/FOUNDATION.md)** - Project vision, architecture, and success criteria
- **[TAGS.md](./spec/TAGS.md)** - Tag registry and guidelines
- **[CLAUDE.md](./spec/CLAUDE.md)** - Specification management process
- **[Gherkin Reference](https://cucumber.io/docs/gherkin/reference)** - Official Gherkin syntax

## Current Status

### ✅ Phase 1: Core Validation & Feature Management (COMPLETE)
- ✅ Gherkin syntax validation with @cucumber/gherkin-parser
- ✅ Clear error messages with line numbers and suggestions
- ✅ Batch validation for all feature files
- ✅ Verbose mode for debugging
- ✅ Feature file creation with templates
- ✅ List features with tag filtering
- ✅ Display feature files in multiple formats
- ✅ Prettier formatting integration

**Commands:** `validate`, `create-feature`, `list-features`, `show-feature`, `format`

### ✅ Phase 2: Tag Registry & Management (COMPLETE)
- ✅ Tag validation against TAGS.md registry
- ✅ Register new tags with categories
- ✅ Update existing tags (category and/or description)
- ✅ Delete tags from registry with safety checks
- ✅ List registered tags with filtering
- ✅ Tag usage statistics and reporting
- ✅ Identify unused registered tags
- ✅ Detect unregistered tags in features
- ✅ Bulk rename tags across all files

**Commands:** `validate-tags`, `register-tag`, `update-tag`, `delete-tag`, `list-tags`, `tag-stats`, `retag`

### ✅ Phase 3: Advanced Feature Editing (COMPLETE)
- ✅ Add scenarios to existing features
- ✅ Add steps to existing scenarios
- ✅ Update scenario names
- ✅ Update step text and/or keywords
- ✅ Delete steps from scenarios
- ✅ Delete scenarios from features
- ✅ Preserve formatting and indentation
- ✅ Handle data tables and doc strings
- ✅ Validate after modifications

**Commands:** `add-scenario`, `add-step`, `update-scenario`, `update-step`, `delete-scenario`, `delete-step`

### ✅ Phase 4: CRUD Operations & Tag-Based Queries (COMPLETE)
- ✅ Query scenarios by tag(s) with AND logic
- ✅ Show acceptance criteria by tag with multiple formats (text, markdown, JSON)
- ✅ Export acceptance criteria to file
- ✅ Bulk delete scenarios by tag across multiple files
- ✅ Bulk delete feature files by tag
- ✅ Dry-run mode for previewing deletions
- ✅ Preserve feature structure during deletions
- ✅ Complete tag-based filtering foundation

**Commands:** `get-scenarios`, `show-acceptance-criteria`, `delete-scenarios`, `delete-features`

### ✅ Phase 5: Advanced CRUD & Bulk Operations (COMPLETE)
- ✅ Delete step from scenario
- ✅ Update scenario (rename)
- ✅ Update step (edit text/type)
- ✅ Delete tag from registry
- ✅ Bulk delete scenarios by tag
- ✅ Bulk delete features by tag
- ✅ Retag operations (rename tags across files)
- ✅ Comprehensive validation suite
- ✅ Dry-run support for destructive operations

**Commands:** `delete-step`, `update-step`, `update-scenario`, `delete-tag`, `delete-scenarios`, `delete-features`, `retag`, `check`

### ✅ Phase 6: Architecture Documentation (COMPLETE)
- ✅ Add/update architecture notes in feature files
- ✅ Add/update user stories (Background) in feature files
- ✅ Add/update Mermaid diagrams in FOUNDATION.md
- ✅ Update foundation sections
- ✅ Display foundation content with multiple formats
- ✅ Section-specific operations
- ✅ JSON output for programmatic access
- ✅ Diagram validation and formatting

**Commands:** `add-architecture`, `add-background`, `add-diagram`, `update-foundation`, `show-foundation`

### 🎯 All Core Features Complete!

**Summary:**
- **Total Commands:** 29 implemented
- **Total Tests:** 315 passing (100% pass rate)
- **Feature Files:** 28 validated specifications
- **Code Coverage:** All commands fully tested
- **Build Size:** 84.15 kB (gzip: 17.54 kB)

### 🔮 Future Enhancements (Optional)
- **JSON I/O Enhancement**: Consistent JSON input/output across all commands for easier AI agent integration
  - Accept JSON input for complex operations (multi-step scenarios, batch updates)
  - Standardize JSON output format across all commands
  - Machine-readable error responses in JSON format

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

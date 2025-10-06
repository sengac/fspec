# fspec - Feature Specification Management for AI Agents

A standardized CLI tool that provides AI agents with a structured interface for managing Gherkin-based feature specifications. fspec prevents ecosystem fragmentation by promoting industry-standard BDD practices over proprietary documentation formats.

## Why fspec?

**The Problem:**
- AI agents default to unstructured markdown for specifications
- Companies create proprietary spec formats (like spec-kit) ignoring 15+ years of BDD maturity
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

### Feature File Management (Coming Soon)

```bash
# Create new feature file
fspec create-feature "User Authentication"

# Add scenario
fspec add-scenario user-authentication "Login with valid credentials"

# List all features
fspec list-features

# Filter by tag
fspec list-features --tag=@phase1
```

### Tag Management (Coming Soon)

```bash
# Register new tag
fspec register-tag @performance "Technical Tags" "Performance-critical features"

# Validate all tags are registered
fspec validate-tags

# Show tag statistics
fspec tag-stats
```

### Architecture Documentation (Coming Soon)

```bash
# Add Mermaid diagram to FOUNDATION.md
fspec add-diagram "Architecture" "System Context" "<mermaid-code>"

# Update foundation section
fspec update-foundation "Problem Definition" "<content>"
```

### Formatting

```bash
# Format all feature files (uses Prettier)
npm run format:spec

# Check formatting
npm run lint:spec
```

## Requirements

- Node.js >= 18.0.0
- Prettier with prettier-plugin-gherkin (for formatting)

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
│   ├── index.ts                 # CLI entry point
│   ├── commands/                # Command implementations
│   │   ├── validate.ts          # Gherkin validation (✅ implemented)
│   │   ├── create-feature.ts    # Feature creation (TODO)
│   │   ├── list-features.ts     # Feature listing (TODO)
│   │   └── format.ts            # Prettier formatting (TODO)
│   └── utils/                   # Shared utilities
├── spec/                        # fspec's own specifications
│   ├── FOUNDATION.md            # Project vision and architecture
│   ├── TAGS.md                  # Tag registry
│   ├── CLAUDE.md                # Specification process guide
│   └── features/                # Gherkin feature files
│       └── gherkin-validation.feature
├── scripts/
│   └── install-local.sh         # Installation script
├── dist/                        # Build output
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
# Validate fspec's feature files
fspec validate spec/features/gherkin-validation.feature

# Format fspec's specs
npm run format:spec
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

**Phase 1: Core Validation & Feature Management**
- ✅ Gherkin syntax validation with @cucumber/gherkin-parser
- ✅ Clear error messages with line numbers and suggestions
- ✅ Batch validation for all feature files
- ✅ Verbose mode for debugging
- 🚧 Feature file creation (TODO)
- 🚧 Scenario/step management (TODO)
- 🚧 Tag validation (TODO)

**Phase 2: Tag Registry & FOUNDATION.md** (Planned)
- Tag management commands
- FOUNDATION.md Mermaid diagram insertion
- Tag statistics and reporting

**Phase 3: CAGE Integration & Advanced Features** (Planned)
- Optimized commands for CAGE hooks
- Batch operations
- Advanced querying

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

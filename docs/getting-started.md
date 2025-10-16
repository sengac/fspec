# Getting Started with fspec

## Quick Start

### 1. Getting Help

fspec has a comprehensive, hierarchical help system with detailed documentation for all commands:

```bash
# Main help - shows command groups and quick start
fspec                  # Shows main help (same as --help)
fspec --help           # Shows main help
fspec help             # Shows command group help

# Group-specific help with all commands, options, and examples
fspec help specs       # Write and manage Gherkin feature files
fspec help work        # Track work units through ACDD workflow
fspec help discovery   # Collaborative discovery with example mapping
fspec help metrics     # Track progress and quality
fspec help setup       # Configure project structure
fspec help hooks       # Lifecycle hooks for automation

# Command-specific help - detailed documentation for ANY command
fspec <command> --help
fspec validate --help           # Comprehensive help for validate command
fspec create-work-unit --help   # Comprehensive help for create-work-unit command
```

**Command Groups:**
- **specs** - Gherkin validation, feature/scenario/step CRUD, bulk operations, formatting
- **work** - Work units, epics, Kanban workflow, dependencies, board visualization
- **discovery** - Example mapping for collaborative discovery
- **metrics** - Progress tracking, estimation, reports
- **setup** - Tag registry, foundation docs, Mermaid diagrams

### 2. Validate Gherkin Syntax

```bash
# Validate all feature files
fspec validate

# Validate specific file
fspec validate spec/features/login.feature

# Verbose output
fspec validate --verbose
```

### 3. Create Your First Feature

```bash
# Create new feature file
fspec create-feature "User Authentication"

# Add scenario
fspec add-scenario user-authentication "Login with valid credentials"

# Add steps
fspec add-step user-authentication "Login with valid credentials" given "I am on the login page"
fspec add-step user-authentication "Login with valid credentials" when "I enter valid credentials"
fspec add-step user-authentication "Login with valid credentials" then "I should be logged in"

# Validate
fspec validate

# Format
fspec format
```

### 4. The ACDD Workflow

**Forward ACDD** (for new features):

1. **Discovery (Example Mapping)**
   ```bash
   fspec create-work-unit AUTH "User login"
   fspec update-work-unit-status AUTH-001 specifying
   fspec add-rule AUTH-001 "Password must be 8+ characters"
   fspec add-example AUTH-001 "User logs in with valid email"
   fspec add-question AUTH-001 "@human: Should we support OAuth?"
   ```

2. **Specification (Gherkin)**
   ```bash
   fspec generate-scenarios AUTH-001  # Auto-generate from example map
   fspec validate
   ```

3. **Testing Phase**
   ```bash
   fspec update-work-unit-status AUTH-001 testing
   # Write tests mapping to Gherkin scenarios BEFORE any code
   npm test  # Tests should fail
   ```

4. **Implementation Phase**
   ```bash
   fspec update-work-unit-status AUTH-001 implementing
   # Write minimum code to make tests pass
   npm test  # Tests should pass
   ```

5. **Validation & Done**
   ```bash
   fspec update-work-unit-status AUTH-001 validating
   npm run check
   fspec update-work-unit-status AUTH-001 done
   ```

**Reverse ACDD** (for existing codebases):

See [Reverse ACDD Guide](./reverse-acdd.md) for complete documentation on using `/rspec` in Claude Code.

## Next Steps

- [Complete Usage Guide](./usage.md) - Detailed command examples
- [Coverage Tracking](./coverage-tracking.md) - Link scenarios to tests and implementation
- [Tag Management](./tags.md) - Organize features with tags
- [Lifecycle Hooks](./hooks/configuration.md) - Automate your workflow
- [Project Management](./project-management.md) - Kanban workflow and work units

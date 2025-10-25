# CLI Reference: Quick Command Cheatsheet

**Quick reference for the most commonly used fspec commands.**

For complete documentation: [User Guide](./user-guide.md)

---

## Setup & Initialization

```bash
# Initialize new project (interactive agent selection)
fspec init

# Or specify agent directly
fspec init --agent=claude     # Claude Code
fspec init --agent=cursor     # Cursor
fspec init --agent=windsurf   # Windsurf
# ... 18 agents supported

# Bootstrap foundation from existing codebase
fspec bootstrap
```

**What `fspec init` creates:**
- Agent-specific slash commands and documentation
- `spec/fspec-config.json` - Runtime configuration for agent detection
- Project structure for features, work units, and tags

**Note:** The init command shows agent-specific activation instructions (e.g., "Run /fspec in Claude Code" vs "Open .cursor/commands/ in Cursor")

---

## Work Units

### Create Work Units

```bash
# Create story (user-facing feature)
fspec create-work-unit AUTH "User Login" --type story

# Create bug
fspec create-work-unit BUG "Session timeout broken" --type bug

# Create task (non-user-facing)
fspec create-work-unit TASK "Refactor auth middleware" --type task

# Create with epic
fspec create-work-unit AUTH "User Login" --type story --epic=USER-MGMT
```

### Update Work Units

```bash
# Update status (ACDD workflow)
fspec update-work-unit-status AUTH-001 specifying
fspec update-work-unit-status AUTH-001 testing
fspec update-work-unit-status AUTH-001 implementing
fspec update-work-unit-status AUTH-001 validating
fspec update-work-unit-status AUTH-001 done

# Set estimate (Fibonacci: 1,2,3,5,8,13)
fspec update-work-unit-estimate AUTH-001 5

# Add dependency
fspec add-dependency AUTH-002 --depends-on AUTH-001

# Delete work unit
fspec delete-work-unit AUTH-001
```

### View Work Units

```bash
# Kanban board
fspec board

# List all work units
fspec list-work-units

# Filter by status/type/epic
fspec list-work-units --status=implementing
fspec list-work-units --type=story
fspec list-work-units --epic=USER-MGMT

# Show details
fspec show-work-unit AUTH-001
```

---

## Example Mapping (Discovery)

### User Story

```bash
# Set user story (BEFORE generating scenarios)
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in securely" \
  --benefit "I can access my account"
```

### Rules (Blue Cards)

```bash
# Add business rule
fspec add-rule AUTH-001 "Password must be 8+ chars with 1 uppercase and 1 number"

# Remove rule
fspec remove-rule AUTH-001 0
```

### Examples (Green Cards)

```bash
# Add concrete example
fspec add-example AUTH-001 "User enters valid credentials and is authenticated"

# Remove example
fspec remove-example AUTH-001 0
```

### Questions (Red Cards)

```bash
# Add question
fspec add-question AUTH-001 "@human: Should we support password reset via email?"

# Answer question
fspec answer-question AUTH-001 0 --answer "Yes, with 1-hour expiry token"

# Remove question
fspec remove-question AUTH-001 0
```

### Attachments

```bash
# Add supporting file
fspec add-attachment AUTH-001 wireframe.png --description "Login page mockup"

# List attachments
fspec list-attachments AUTH-001

# Remove attachment
fspec remove-attachment AUTH-001 wireframe.png
```

---

## Feature Files (Specifications)

### Create & Generate

```bash
# Generate scenarios from example map
fspec generate-scenarios AUTH-001

# Generate with custom feature name
fspec generate-scenarios AUTH-001 --feature=admin-login

# Create empty feature file
fspec create-feature user-login
```

### Format & Validate

```bash
# Format feature files (auto-fix)
fspec format

# Format specific file
fspec format spec/features/user-login.feature

# Validate Gherkin syntax
fspec validate

# Validate specific file
fspec validate spec/features/user-login.feature
```

### List & Show

```bash
# List all features
fspec list-features

# Show feature details
fspec show-feature user-login

# Filter by tag
fspec list-features --tag=@phase1
```

---

## Coverage Tracking

```bash
# Link scenario to test and implementation
fspec link-coverage user-login \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 15-42

# Show coverage
fspec show-coverage user-login

# Show all coverage
fspec show-coverage
```

---

## Tags

### Manage Tags

```bash
# Register new tag
fspec register-tag phase2 phase "Phase 2 features"

# Unregister tag
fspec unregister-tag phase2

# List registered tags
fspec list-tags

# Show tag details
fspec show-tag phase1

# Validate tags in feature files
fspec validate-tags
```

### Tag Feature Files

```bash
# Add tag to feature file (manual edit)
# @phase1 @authentication @story @AUTH-001
Feature: User Login
```

---

## Git Checkpoints

### Manual Checkpoints

```bash
# Create checkpoint
fspec checkpoint AUTH-001 baseline

# List checkpoints
fspec list-checkpoints AUTH-001

# Restore checkpoint
fspec restore-checkpoint AUTH-001 baseline

# Delete checkpoint
fspec delete-checkpoint AUTH-001 baseline

# Clean up old checkpoints
fspec cleanup-checkpoints AUTH-001
```

### Automatic Checkpoints

```bash
# Auto-created before state transitions
fspec update-work-unit-status AUTH-001 testing
# Creates: fspec-checkpoint:AUTH-001:AUTH-001-auto-specifying:1234567890123

# Restore auto checkpoint
fspec restore-checkpoint AUTH-001 AUTH-001-auto-specifying
```

---

## Virtual Hooks

### Add Hooks

```bash
# Add blocking hook (failure prevents transition)
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking

# Add non-blocking hook (failure just logs)
fspec add-virtual-hook AUTH-001 pre-validating "npm run lint"

# Add with git context (only changed files)
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking

# Add with timeout (default: 300000ms)
fspec add-virtual-hook AUTH-001 post-testing "npm test" --blocking --timeout=120000
```

### Manage Hooks

```bash
# List hooks
fspec list-virtual-hooks AUTH-001

# Remove hook
fspec remove-virtual-hook AUTH-001 0

# Test hook execution
fspec test-virtual-hook AUTH-001 0

# Clean up hooks (when work unit done)
fspec cleanup-virtual-hooks AUTH-001
```

### Hook Events

```
pre-specifying           post-specifying
pre-testing              post-testing
pre-implementing         post-implementing
pre-validating           post-validating
```

---

## Epics

```bash
# Create epic
fspec create-epic "User Management" USER-MGMT "All user-related features"

# List epics
fspec list-epics

# Show epic details
fspec show-epic USER-MGMT

# Delete epic
fspec delete-epic USER-MGMT
```

---

## Validation & Quality

```bash
# Validate everything
fspec check

# Validate Gherkin syntax
fspec validate

# Validate tags
fspec validate-tags

# Check foundation
fspec check-foundation
```

---

## Metrics & Reports

```bash
# Summary report
fspec generate-summary-report

# Query metrics
fspec query-metrics

# Estimate accuracy
fspec query-estimate-accuracy

# Coverage statistics
fspec show-coverage
```

---

## Analysis & Querying

```bash
# Search scenarios across features
fspec search-scenarios --query="validation"
fspec search-scenarios --query="user.*login" --regex

# Search for function usage
fspec search-implementation --function=validateInput
fspec search-implementation --function=queryWorkUnits --show-work-units

# Compare implementations
fspec compare-implementations --tag=@cli
fspec compare-implementations --tag=@authentication --show-coverage

# Analyze test patterns
fspec show-test-patterns --tag=@high
fspec show-test-patterns --tag=@cli --include-coverage

# Query work units (basic)
fspec query-work-units --status=done --tag=@cli --format=table
```

---

## Reverse ACDD

```bash
# Interactive mode (recommended)
fspec reverse

# Strategy selection
fspec reverse --strategy=A  # Spec gap (have tests, need specs)
fspec reverse --strategy=B  # Test gap (have specs, need tests)
fspec reverse --strategy=C  # Coverage gap (link existing)
fspec reverse --strategy=D  # Full reverse (just code)

# Session management
fspec reverse --resume
fspec reverse --clear-session
```

---

## Common Workflows

### Create New Story (Full ACDD)

```bash
# 1. Create work unit
fspec create-work-unit AUTH "User Login" --type story
fspec update-work-unit-status AUTH-001 specifying

# 2. Example Mapping (Discovery)
fspec set-user-story AUTH-001 --role "user" --action "log in" --benefit "access account"
fspec add-rule AUTH-001 "Password 8+ chars with uppercase and number"
fspec add-example AUTH-001 "User enters valid credentials and is authenticated"
fspec answer-question AUTH-001 0 --answer "Yes"

# 3. Generate feature file
fspec generate-scenarios AUTH-001
fspec validate

# 4. Estimate (after specs complete)
fspec update-work-unit-estimate AUTH-001 5

# 5. Move to testing
fspec update-work-unit-status AUTH-001 testing

# 6. Write tests (map to scenarios)
# ... create test file with feature reference ...

# 7. Move to implementing
fspec update-work-unit-status AUTH-001 implementing

# 8. Write code (make tests pass)
# ... implement feature ...

# 9. Validate
fspec update-work-unit-status AUTH-001 validating
fspec check
npm test

# 10. Done
fspec update-work-unit-status AUTH-001 done
```

### Link Coverage

```bash
# After implementing
fspec link-coverage user-login \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 15-42

# Verify coverage
fspec show-coverage user-login
```

### Add Quality Gates

```bash
# Ensure tests run after implementing
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking

# Ensure linting before validating
fspec add-virtual-hook AUTH-001 pre-validating "npm run lint" --blocking

# Ensure type checking
fspec add-virtual-hook AUTH-001 pre-validating "npm run typecheck" --blocking
```

### Safe Experimentation

```bash
# Create baseline before experimenting
fspec checkpoint AUTH-001 baseline

# Try approach A
# ... make changes ...
# Doesn't work? Restore

fspec restore-checkpoint AUTH-001 baseline

# Try approach B
# ... make changes ...
# Works!

# Clean up old checkpoints
fspec cleanup-checkpoints AUTH-001
```

---

## Help

```bash
# General help
fspec --help

# Command help
fspec validate --help
fspec create-work-unit --help
fspec add-virtual-hook --help
```

---

## Tips

**Workflow Order (ACDD):**
1. Discovery (Example Mapping)
2. Specifications (Feature Files)
3. Testing (Write tests FIRST)
4. Implementation (Make tests pass)
5. Validation (Quality checks)
6. Done

**Estimation:**
- Use Fibonacci: 1, 2, 3, 5, 8, 13
- Estimate AFTER Example Mapping
- 13 points is upper limit (acceptable)
- 21+ points means split the story

**Tags:**
- Register tags before using: `fspec register-tag`
- Validate tags: `fspec validate-tags`
- Prefix format: `@category-value`

**Checkpoints:**
- Auto-created before state transitions
- Create manual checkpoints for experiments
- Clean up when work unit done

**Virtual Hooks:**
- Use `--blocking` for quality gates
- Use `--git-context` for efficiency
- Clean up when work unit done

---

**For detailed guides, see:**
- [Getting Started](./getting-started.md) - 5-minute quickstart
- [User Guide](./user-guide.md) - Complete command reference
- [ACDD Workflow](./acdd-workflow.md) - Understanding the process
- [Example Mapping](./example-mapping.md) - Discovery techniques
- [Work Units](./work-units.md) - Project management
- [Checkpoints](./checkpoints.md) - Safe experimentation
- [Virtual Hooks](./virtual-hooks.md) - Quality gates
- [Reverse ACDD](./reverse-acdd.md) - Document existing codebases

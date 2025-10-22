# User Guide: Complete Command Reference

## Overview

This guide covers all fspec commands organized by workflow stage. Remember: **you don't run these commands directly** - your AI agent does. This reference helps you understand what's happening behind the scenes.

**Quick navigation:**
- [Initialization](#initialization)
- [Foundation Discovery](#foundation-discovery)
- [Work Unit Management](#work-unit-management)
- [Example Mapping (Discovery)](#example-mapping-discovery)
- [Feature Files (Specifications)](#feature-files-specifications)
- [Coverage Tracking](#coverage-tracking)
- [Tags and Organization](#tags-and-organization)
- [Git Checkpoints](#git-checkpoints)
- [Virtual Hooks](#virtual-hooks)
- [Validation and Formatting](#validation-and-formatting)
- [Metrics and Reporting](#metrics-and-reporting)

---

## Initialization

### fspec init

Initialize fspec in your project with multi-agent support.

```bash
# Interactive menu to select AI agent(s)
fspec init

# Or specify agent directly
fspec init --agent=claude     # Claude Code
fspec init --agent=cursor     # Cursor
fspec init --agent=windsurf   # Windsurf
fspec init --agent=cline      # Cline (VS Code)
# ... supports 18 AI agents
```

**Creates:**
- **Agent-specific slash command** (e.g., `.claude/commands/fspec.md`, `.cursor/commands/fspec.md`)
- **Workflow documentation** (e.g., `spec/CLAUDE.md`, `spec/CURSOR.md`)
- **Project structure** (`spec/` directory, `work-units.json`, `tags.json`)

**When to use:** Once per project, before starting any work. Run again with different `--agent` to support multiple agents simultaneously.

---

## Foundation Discovery

### fspec discover-foundation

Bootstrap `spec/foundation.json` through AI-driven discovery.

```bash
# Create draft with placeholders
fspec discover-foundation

# Finalize after all fields filled
fspec discover-foundation --finalize
```

**Workflow:**
1. Creates `foundation.json.draft` with `[QUESTION:]` placeholders
2. AI fills fields one-by-one using `fspec update-foundation`
3. Finalize validates and generates `foundation.json` + `FOUNDATION.md`

**See also:** `fspec discover-foundation --help`

### fspec update-foundation

Update specific field in foundation draft.

```bash
fspec update-foundation --field project.name --value "fspec"
fspec update-foundation --field project.vision --value "CLI tool for ACDD"
```

**When to use:** During foundation discovery workflow.

### fspec add-persona / remove-persona

Manage user personas in foundation.

```bash
fspec add-persona "Developer" "Software engineer using fspec" --goal "Build quality software with AI"
fspec remove-persona "Developer"
```

### fspec add-capability / remove-capability

Manage system capabilities in foundation.

```bash
fspec add-capability "User Authentication" "Users can log in securely"
fspec remove-capability "User Authentication"
```

### fspec show-foundation

Display foundation documentation.

```bash
fspec show-foundation
fspec show-foundation --section "What We Are Building"
fspec show-foundation --format=json
```

---

## Work Unit Management

### fspec board

Display Kanban board showing work across all states.

```bash
fspec board
```

**Output:**
```
┌─────────┬────────────┬─────────┬───────────────┬───────────┬──────┐
│ Backlog │ Specifying │ Testing │ Implementing  │Validating │ Done │
├─────────┼────────────┼─────────┼───────────────┼───────────┼──────┤
│ AUTH-002│            │         │ AUTH-001      │           │      │
│ DASH-001│            │         │ User Login    │           │      │
│         │            │         │ (5 pts)       │           │      │
└─────────┴────────────┴─────────┴───────────────┴───────────┴──────┘
```

### fspec create-work-unit

Create new work unit (story, bug, or task).

```bash
fspec create-work-unit AUTH "User Login" --type story
fspec create-work-unit BUG "Session timeout broken" --type bug
fspec create-work-unit TASK "Refactor middleware" --type task
```

**Options:**
- `--type` - story (default), bug, or task
- `--description` - Detailed description
- `--epic` - Epic ID to associate with

### fspec update-work-unit-status

Move work unit through Kanban workflow.

```bash
fspec update-work-unit-status AUTH-001 specifying
fspec update-work-unit-status AUTH-001 testing
fspec update-work-unit-status AUTH-001 implementing
fspec update-work-unit-status AUTH-001 validating
fspec update-work-unit-status AUTH-001 done
fspec update-work-unit-status AUTH-001 blocked
```

**ACDD workflow enforced:**
- Cannot skip states
- Temporal ordering prevents retroactive changes
- Automatic checkpoints created (if changes exist)

### fspec update-work-unit-estimate

Set story points for estimation.

```bash
fspec update-work-unit-estimate AUTH-001 5
```

**Validation:**
- Cannot estimate stories/bugs without completed feature file
- Warns if estimate > 13 points (should break down)
- Uses Fibonacci scale: 1, 2, 3, 5, 8, 13

### fspec list-work-units

List work units with filters.

```bash
fspec list-work-units
fspec list-work-units --status=implementing
fspec list-work-units --type=story
fspec list-work-units --epic=user-management
```

### fspec show-work-unit

Display detailed work unit information.

```bash
fspec show-work-unit AUTH-001
fspec show-work-unit AUTH-001 --format=json
```

**Shows:**
- Status, type, estimate
- Example mapping (rules, examples, questions)
- Dependencies and epic relationship
- Attachments
- Virtual hooks

### fspec add-dependency / remove-dependency

Manage work unit dependencies.

```bash
fspec add-dependency AUTH-002 --depends-on AUTH-001
fspec remove-dependency AUTH-002 --depends-on AUTH-001
```

### fspec create-epic

Create epic to group related work units.

```bash
fspec create-epic "User Management" USER-MGMT "All user-related features"
```

### fspec delete-work-unit

Remove work unit from tracking.

```bash
fspec delete-work-unit AUTH-001
```

---

## Example Mapping (Discovery)

### fspec set-user-story

Set user story fields for work unit.

```bash
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in securely" \
  --benefit "I can access my account"
```

**CRITICAL:** Set this BEFORE generating scenarios to avoid placeholder text.

### fspec add-rule / remove-rule

Manage business rules (blue cards).

```bash
fspec add-rule AUTH-001 "Password must be at least 8 characters"
fspec remove-rule AUTH-001 0
```

### fspec add-example / remove-example

Manage concrete examples (green cards).

```bash
fspec add-example AUTH-001 "User logs in with valid credentials and is authenticated"
fspec remove-example AUTH-001 0
```

### fspec add-question / answer-question

Manage uncertainties (red cards).

```bash
fspec add-question AUTH-001 "@human: Should we support OAuth?"
fspec answer-question AUTH-001 0 --answer "Yes, Google and GitHub OAuth"
fspec remove-question AUTH-001 0
```

### fspec add-attachment / remove-attachment

Attach supporting files to work unit.

```bash
fspec add-attachment AUTH-001 diagrams/auth-flow.png --description "Authentication flow diagram"
fspec list-attachments AUTH-001
fspec remove-attachment AUTH-001 auth-flow.png
fspec remove-attachment AUTH-001 auth-flow.png --keep-file
```

**Supported files:** Images, diagrams, mockups, PDFs, Mermaid `.mmd` files

### fspec generate-scenarios

Generate Gherkin feature file from example map.

```bash
fspec generate-scenarios AUTH-001
fspec generate-scenarios AUTH-001 --feature=user-authentication
```

**Transforms:**
- Rules → Background context or scenario preconditions
- Examples → Concrete Given-When-Then scenarios
- User story → Background section

---

## Feature Files (Specifications)

### fspec create-feature

Create new Gherkin feature file.

```bash
fspec create-feature "User Authentication"
```

**Creates:**
- `spec/features/user-authentication.feature`
- `spec/features/user-authentication.feature.coverage`

### fspec add-scenario

Add scenario to existing feature.

```bash
fspec add-scenario user-authentication "Login with valid credentials"
```

### fspec add-step

Add step to existing scenario.

```bash
fspec add-step user-authentication "Login with valid credentials" given "I am on the login page"
fspec add-step user-authentication "Login with valid credentials" when "I enter valid credentials"
fspec add-step user-authentication "Login with valid credentials" then "I should be logged in"
```

### fspec update-scenario

Rename scenario.

```bash
fspec update-scenario user-authentication "Old Name" "New Name"
```

### fspec update-step

Update step text or keyword.

```bash
fspec update-step user-authentication "Login" "I am on the login page" --text "I navigate to the login page"
fspec update-step user-authentication "Login" "I am on the login page" --keyword When
```

### fspec delete-step / delete-scenario

Remove step or scenario.

```bash
fspec delete-step user-authentication "Login" "I am on the login page"
fspec delete-scenario user-authentication "Login with valid credentials"
```

### fspec list-features

List all feature files.

```bash
fspec list-features
fspec list-features --tag=@phase1
fspec list-features --format=json
```

### fspec show-feature

Display specific feature.

```bash
fspec show-feature user-authentication
fspec show-feature user-authentication --format=json
```

### fspec add-architecture / add-background

Add documentation to feature file.

```bash
fspec add-architecture user-authentication "Uses JWT tokens for session management"
fspec add-background user-authentication "As a user\nI want to log in\nSo that I can access my account"
```

---

## Coverage Tracking

### fspec generate-coverage

Create coverage files for all features.

```bash
fspec generate-coverage
fspec generate-coverage --dry-run
```

**What it does:**
- Creates `.coverage` files for features without them
- Updates existing coverage files with missing scenarios
- Does NOT overwrite existing mappings

### fspec link-coverage

Link scenario to test and implementation.

```bash
# Link test to scenario
fspec link-coverage user-authentication \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --test-lines 45-62

# Link implementation to test mapping
fspec link-coverage user-authentication \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 10-24

# Link both at once
fspec link-coverage user-authentication \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --test-lines 45-62 \
  --impl-file src/auth/login.ts \
  --impl-lines 10-24
```

### fspec show-coverage

Display coverage for feature.

```bash
fspec show-coverage user-authentication
fspec show-coverage user-authentication --format=json
fspec show-coverage  # All features
```

**Output:**
```
Coverage for user-authentication:

✅ Login with valid credentials (FULLY COVERED)
   Test: src/__tests__/auth.test.ts:45-62
   Implementation: src/auth/login.ts:10-24

❌ Login with invalid credentials (NO COVERAGE)
   No test mappings

⚠️  Login with OAuth (PARTIAL COVERAGE)
   Test: src/__tests__/auth.test.ts:70-85
   Implementation: (not linked)

Coverage: 60% (3/5 scenarios)
```

### fspec audit-coverage

Validate coverage file references.

```bash
fspec audit-coverage user-authentication
```

**Checks:**
- Test files exist
- Implementation files exist
- Line numbers are valid

---

## Tags and Organization

### fspec register-tag

Add tag to registry.

```bash
fspec register-tag @authentication --category component --description "Authentication-related features"
fspec register-tag @phase1 --category phase
fspec register-tag @critical --category priority
```

### fspec unregister-tag

Remove tag from registry.

```bash
fspec unregister-tag @deprecated
```

### fspec list-tags

List all registered tags.

```bash
fspec list-tags
fspec list-tags --category=component
```

### fspec add-tag-to-feature / remove-tag-from-feature

Manage feature-level tags.

```bash
fspec add-tag-to-feature spec/features/user-authentication.feature @phase1
fspec remove-tag-from-feature spec/features/user-authentication.feature @wip
```

### fspec tag-stats

Show tag usage statistics.

```bash
fspec tag-stats
```

**Output:**
```
Tag Statistics:

@phase1: 12 features
@authentication: 5 features
@critical: 3 features
@wip: 1 feature
```

---

## Git Checkpoints

### fspec checkpoint

Create manual checkpoint.

```bash
fspec checkpoint AUTH-001 baseline
fspec checkpoint AUTH-001 before-refactor
```

### fspec list-checkpoints

List checkpoints for work unit.

```bash
fspec list-checkpoints AUTH-001
```

### fspec restore-checkpoint

Restore to previous checkpoint.

```bash
fspec restore-checkpoint AUTH-001 baseline
```

**Prompts if working directory dirty:**
1. Commit changes first (low risk)
2. Stash changes and restore (medium risk)
3. Force restore with merge (high risk)

### fspec cleanup-checkpoints

Clean up old checkpoints.

```bash
fspec cleanup-checkpoints AUTH-001 --keep-last 5
```

**See:** [Git Checkpoints Guide](./checkpoints.md) for detailed usage.

---

## Virtual Hooks

### fspec add-virtual-hook

Add work unit-scoped quality gate.

```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
```

**Events:**
- `pre-specifying`, `post-specifying`
- `pre-testing`, `post-testing`
- `pre-implementing`, `post-implementing`
- `pre-validating`, `post-validating`

**Flags:**
- `--blocking` - Failure prevents progression
- `--git-context` - Auto-generate script that processes changed files

### fspec list-virtual-hooks

List virtual hooks for work unit.

```bash
fspec list-virtual-hooks AUTH-001
```

### fspec remove-virtual-hook

Remove specific virtual hook.

```bash
fspec remove-virtual-hook AUTH-001 eslint
```

### fspec clear-virtual-hooks

Remove all virtual hooks from work unit.

```bash
fspec clear-virtual-hooks AUTH-001
```

### fspec copy-virtual-hooks

Copy hooks between work units.

```bash
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint
```

**See:** [Virtual Hooks Guide](./virtual-hooks.md) for detailed usage.

---

## Validation and Formatting

### fspec validate

Validate Gherkin syntax.

```bash
fspec validate
fspec validate spec/features/user-authentication.feature
```

**Uses:** Official @cucumber/gherkin parser

### fspec validate-tags

Validate all tags are registered.

```bash
fspec validate-tags
```

**Checks:**
- All tags in feature files exist in `spec/tags.json`
- Required tags present (@phase, @component, work unit ID)

### fspec format

Format feature files using custom AST-based formatter.

```bash
fspec format
fspec format spec/features/user-authentication.feature
```

**Guarantees:**
- Consistent indentation (2 spaces)
- Proper spacing around keywords
- Preserves doc strings and data tables
- Maintains tag formatting

### fspec check

Run all validation checks.

```bash
fspec check
fspec check --verbose
```

**Runs:**
- `fspec validate` (Gherkin syntax)
- `fspec validate-tags` (tag compliance)
- Auto-formats if needed

---

## Metrics and Reporting

### fspec generate-summary-report

Generate comprehensive project report.

```bash
fspec generate-summary-report
```

**Includes:**
- Work unit counts by status and type
- Story points completed vs remaining
- Coverage statistics
- Tag usage
- Dependency graph
- Epic progress

### fspec query-metrics

Query specific metrics.

```bash
fspec query-metrics
fspec query-metrics --format=json
```

### fspec query-estimate-accuracy

Check estimation accuracy over time.

```bash
fspec query-estimate-accuracy
```

**Shows:**
- Estimated vs actual story points
- Velocity trends
- Estimation patterns

---

## Reverse ACDD

### fspec reverse

Interactive reverse ACDD for existing codebases.

```bash
fspec reverse
fspec reverse --strategy=A  # Spec gap (generate from tests)
fspec reverse --strategy=B  # Test gap (generate from code)
fspec reverse --strategy=C  # Coverage gap (link tests to code)
fspec reverse --strategy=D  # Full reverse ACDD
```

**Interactive mode:**
- Analyzes codebase structure
- Suggests strategy based on gaps
- Guides through decision tree
- Saves session state

**See:** [Reverse ACDD Guide](./reverse-acdd.md) for detailed usage.

---

## Help Commands

### fspec --help

Show general help.

```bash
fspec --help
```

### fspec help <category>

Get detailed help for command categories.

```bash
fspec help specs      # Gherkin feature file commands
fspec help work       # Kanban workflow commands
fspec help discovery  # Example mapping commands
fspec help metrics    # Progress tracking commands
fspec help setup      # Configuration commands
fspec help hooks      # Lifecycle hook commands
```

### fspec <command> --help

Get detailed help for specific command.

```bash
fspec create-work-unit --help
fspec add-virtual-hook --help
fspec generate-scenarios --help
```

**Every command has:**
- Complete usage syntax
- AI-optimized sections (WHEN TO USE, PREREQUISITES, etc.)
- Multiple examples
- Related commands
- Best practices

---

## Quick Reference by Workflow Stage

### 1. Initialization
```bash
fspec init
fspec discover-foundation
```

### 2. Create Work
```bash
fspec create-work-unit AUTH "User Login" --type story
fspec update-work-unit-status AUTH-001 specifying
```

### 3. Discovery
```bash
fspec set-user-story AUTH-001 --role "user" --action "log in" --benefit "access account"
fspec add-rule AUTH-001 "Password min 8 chars"
fspec add-example AUTH-001 "User logs in with valid credentials"
fspec generate-scenarios AUTH-001
```

### 4. Testing
```bash
fspec update-work-unit-status AUTH-001 testing
# AI writes tests
fspec link-coverage user-authentication --scenario "Login" --test-file ... --test-lines ...
```

### 5. Implementation
```bash
fspec update-work-unit-status AUTH-001 implementing
# AI writes code
fspec link-coverage user-authentication --scenario "Login" --test-file ... --impl-file ...
```

### 6. Validation
```bash
fspec update-work-unit-status AUTH-001 validating
fspec validate
fspec check
```

### 7. Completion
```bash
fspec update-work-unit-status AUTH-001 done
fspec board
```

---

**For more details, see:**
- [Getting Started](./getting-started.md)
- [ACDD Workflow](./acdd-workflow.md)
- [Example Mapping](./example-mapping.md)
- [Work Units](./work-units.md)
- [Checkpoints](./checkpoints.md)
- [Virtual Hooks](./virtual-hooks.md)
- [CLI Reference](./cli-reference.md)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**Stop fixing AI chaos. Start shipping quality.**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm](https://img.shields.io/npm/v/@sengac/fspec)](https://www.npmjs.com/package/@sengac/fspec)

---

## What is fspec?

**fspec makes AI agents follow discipline.** It's the missing workflow layer that turns AI coding chaos into structured, testable, traceable software.

Think **Git for specifications** + **Kanban for AI agents** + **TDD enforcement** all in one.

---

## Quick Start

```bash
# Install
npm install -g @sengac/fspec

# Initialize in your project (interactive agent selection)
cd /path/to/your/project
fspec init

# Or specify your AI agent directly
fspec init --agent=claude    # Claude Code
fspec init --agent=cursor    # Cursor
fspec init --agent=windsurf  # Windsurf
# ... and 15 more agents

# Configure test and quality check tools (REQUIRED for platform-agnostic workflows)
fspec configure-tools --test-command "npm test" \
  --quality-commands "npm run format" "npx tsc --noEmit"

# Switch agents (auto-detects and prompts for confirmation)
fspec init --agent=cursor    # Switches from claude to cursor

# Remove agent initialization files
fspec remove-init-files

# Tell your AI agent to create a feature
/fspec Create a story for user authentication
```

**Version Sync (Automatic):**

When you upgrade fspec (e.g., `npm install -g @sengac/fspec@0.7.0`), the next time you run `/fspec`:
- **Version check runs first** - Compares embedded version in slash command file to package.json
- **On mismatch** - Updates both `.claude/commands/fspec.md` (or equivalent) AND `spec/CLAUDE.md`, shows agent-specific restart message, exits
- **On match** - Silent pass-through, AI continues loading normally
- **Result:** Always up-to-date documentation without manual reinstall

**What happens next:**

1. **Discovery** - AI asks clarifying questions using Example Mapping
   ```
   AI: "Should we support OAuth in addition to email/password?"
   You: "Yes, Google and GitHub OAuth"
   AI: "What password requirements?"
   You: "Min 8 chars, 1 uppercase, 1 number"
   ```

2. **Specification** - AI generates validated Gherkin scenarios
   ```gherkin
   Feature: User Authentication
     Scenario: Login with email and password
       Given I am on the login page
       When I enter valid credentials
       Then I should be logged in
   ```

3. **Testing** - AI writes tests that map to scenarios (must fail first)
4. **Implementation** - AI writes minimal code to pass tests
5. **Validation** - AI runs all quality checks, coverage validation

**Result:** Clean, tested, documented feature. No skipped steps. No chaos.

---

## Core Features

### 🎯 ACDD Workflow Enforcement

Strict Kanban workflow: `backlog → specifying → testing → implementing → validating → done`

- Can't write code before tests
- Can't write tests before specs
- Can't skip discovery
- Temporal ordering prevents retroactive state walking

```bash
$ fspec board

┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│BACKLOG   │SPECIFYING│TESTING   │IMPLEMENTI│VALIDATING│DONE      │BLOCKED   │
├──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┤
│AUTH-002  │AUTH-001  │UI-003    │API-005   │AUTH-004  │LOGIN-001 │          │
│UI-004    │          │          │          │          │AUTH-003  │          │
│API-006   │          │          │          │          │UI-001    │          │
│          │          │          │          │          │UI-002    │          │
│          │          │          │          │          │API-001   │          │
│          │          │          │          │          │... 45 mo │          │
├────────────────────────────────────────────────────────────────────────────┤
│13 points in progress, 127 points completed                                 │
└────────────────────────────────────────────────────────────────────────────┘
```

### 🤝 Example Mapping for Discovery

Collaborative conversation between you and AI:

- **Rules** (blue cards) - Business logic and constraints
- **Examples** (green cards) - Concrete scenarios
- **Questions** (red cards) - Uncertainties to resolve
- **Attachments** - Diagrams, mockups, documents

AI asks questions. You provide answers. Specs emerge from shared understanding.

### 💾 Git Checkpoints

Safe experimentation with automatic and manual save points:

```
Create baseline → Try approach A → Doesn't work? → Restore baseline → Try approach B
```

- Automatic checkpoints before workflow transitions
- Manual checkpoints for experimentation
- Re-restorable (same checkpoint multiple times)
- Conflict resolution assistance

### ⚡ Virtual Hooks

Work unit-scoped quality gates that enforce standards:

```bash
# Run tests after implementing
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking

# Lint only changed files (git context)
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
```

- Automatic script generation
- Git context integration
- Blocking or non-blocking
- Ephemeral (removed when work done)

### 🔄 Reverse ACDD

Document existing codebases by reverse engineering:

```
/fspec Run fspec reverse
```

Interactive strategy planner:
- **Strategy A**: Generate specs from tests (spec gap analysis)
- **Strategy B**: Generate tests from code (test gap analysis)
- **Strategy C**: Link existing tests to code (coverage gap)
- **Strategy D**: Full reverse ACDD (specs + tests + coverage)

### 🔗 Coverage Tracking

Full traceability from acceptance criteria to implementation:

```
Scenario → Test File (lines 45-62) → Implementation (lines 10-24)
          ↓ Step Validation
    // @step Given I am on the login page
    // @step When I enter valid credentials
    // @step Then I should be logged in
```

- **Scenario-to-test-to-implementation mappings** - Track every line of code
- **Step validation (NEW)** - Cucumber-style step matching in test comments
- **Coverage percentage reporting** - Know exactly what's tested
- **Gap detection** - Find untested scenarios
- **Audit commands** - Verify traceability integrity
- **Parameterized step matching** - Handles `{int}`, `{string}` placeholders

### 🔍 Advanced Search & Comparison

Find patterns, compare implementations, ensure consistency:

```bash
# Search scenarios across all features
fspec search-scenarios --query="validation"

# Find function usage across work units
fspec search-implementation --function=validateInput --show-work-units

# Compare implementation approaches for similar features
fspec compare-implementations --tag=@cli --show-coverage

# Analyze testing patterns for consistency
fspec show-test-patterns --tag=@authentication --include-coverage
```

- Cross-feature scenario search (literal or regex)
- Function usage analysis across codebase
- Side-by-side implementation comparison
- Testing pattern analysis and consistency checking
- Identify architectural inconsistencies early

---

## Documentation

- 📘 **[Getting Started](./docs/getting-started.md)** - 5-minute quickstart
- 📖 **[User Guide](./docs/user-guide.md)** - Comprehensive usage
- 🎯 **[ACDD Workflow](./docs/acdd-workflow.md)** - Understanding the process
- 🤝 **[Example Mapping](./docs/example-mapping.md)** - Discovery techniques
- 📊 **[Work Units](./docs/work-units.md)** - Project management
- 🔗 **[Coverage Tracking](./docs/coverage-tracking.md)** - Traceability
- 🔄 **[Reverse ACDD](./docs/reverse-acdd.md)** - Existing codebases
- 💾 **[Git Checkpoints](./docs/checkpoints.md)** - Safe experimentation
- ⚡ **[Virtual Hooks](./docs/virtual-hooks.md)** - Quality gates
- 🏷️ **[Tags](./docs/tags.md)** - Organization system
- 🔧 **[CLI Reference](./docs/cli-reference.md)** - Command cheatsheet

**Pro tip:** All commands have comprehensive `--help` output:
```bash
fspec <command> --help
fspec help specs      # Gherkin commands
fspec help work       # Kanban commands
fspec help discovery  # Example mapping commands
```

---

**[Visit fspec.dev](https://fspec.dev)** | **[GitHub](https://github.com/sengac/fspec)** | **[npm](https://www.npmjs.com/package/@sengac/fspec)**

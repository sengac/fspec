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

### The Problem

Your AI agent writes code fast. Too fast. You end up with:

- ❌ Code before tests, tests before specs (or no specs at all)
- ❌ Context loss between sessions ("wait, what were we building?")
- ❌ Implementing the wrong thing (AI guessed instead of asking)
- ❌ Broken tests, missing coverage, drifting architecture docs
- ❌ No way to track what's done, what's blocked, or why decisions were made

**Flat TODO lists don't help.** They show "done" or "not done" but no workflow state, no dependencies, no traceability.

### The Solution

**fspec enforces the right order:**

```
Discovery → Specifications → Tests → Implementation → Validation → Done
```

Can't skip steps. Can't go backward without intention. Can't write code before tests. **The AI agent must follow ACDD** (Acceptance Criteria Driven Development).

**And you get persistent state:**
- Kanban board showing work in progress
- Example mapping with rules, examples, and questions
- Gherkin specs validated by Cucumber's official parser
- Coverage tracking linking scenarios → tests → implementation
- Git checkpoints for safe experimentation
- Virtual hooks for work-unit-specific quality gates

---

## See It in Action

**30-second demo:**

```bash
# Install
npm install -g @sengac/fspec

# Initialize in your project
cd /path/to/your/project
fspec init

# Tell Claude Code to create a feature
/fspec Create a story for user authentication
```

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

## Key Benefits

### For You (The Developer)

- ✅ **AI agents that actually follow TDD/BDD** - No more code-first disasters
- ✅ **Persistent queryable state** - Survives context resets, session switches
- ✅ **Stop repeating yourself** - AI reads the board, knows what's in flight
- ✅ **Safe experimentation** - Git checkpoints let AI try multiple approaches
- ✅ **Quality gates that actually run** - Virtual hooks enforce linting, testing, validation

### For Your Team

- ✅ **Shared understanding** - Example Mapping captures decisions and rationale
- ✅ **Living documentation** - Specs stay synchronized with code automatically
- ✅ **Full traceability** - Every scenario links to tests and implementation
- ✅ **Reverse ACDD for existing code** - Document what you already built
- ✅ **Workflow enforcement** - Kanban prevents cutting corners

### For Your Project

- ✅ **Validated Gherkin** - Official @cucumber/gherkin parser, no syntax errors
- ✅ **Tag discipline** - JSON-backed registry prevents tag chaos
- ✅ **Foundation documentation** - AI discovers project vision, capabilities, personas
- ✅ **Work unit management** - Track epics, dependencies, estimates, status
- ✅ **Coverage tracking** - Know exactly what's tested and what's not

---

## Quick Start

### 1. Install

```bash
npm install -g @sengac/fspec
```

### 2. Initialize

```bash
cd /path/to/your/project
fspec init
```

This creates:
- `.claude/commands/fspec.md` - Forward ACDD for new features
- `spec/CLAUDE.md` - Workflow guidelines for AI agents

### 3. Build Features with AI

**In Claude Code, tell it to create a work unit:**

```
/fspec Create a story for user login
/fspec Create a bug for session timeout not working
/fspec Create a task to refactor auth middleware
```

**Or just check the board:**

```
/fspec
```

The AI agent will:
- Guide you through discovery (Example Mapping)
- Generate validated Gherkin specifications
- Write tests that map to scenarios
- Implement code to pass tests
- Run quality checks and validation

**You don't run fspec commands directly.** The AI agent handles the workflow.

---

## Core Features

### 🎯 ACDD Workflow Enforcement

Strict Kanban workflow: `backlog → specifying → testing → implementing → validating → done`

- Can't write code before tests
- Can't write tests before specs
- Can't skip discovery
- Temporal ordering prevents retroactive state walking

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
```

- Scenario-to-test-to-implementation mappings
- Coverage percentage reporting
- Gap detection (untested scenarios)
- Audit commands for validation

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

## What You Get

✅ **AI agents that follow TDD/BDD religiously**
✅ **Specifications that stay in sync with code**
✅ **Full traceability from idea to implementation**
✅ **Persistent state that survives context resets**
✅ **Quality gates that actually enforce standards**
✅ **Living documentation that never drifts**
✅ **Safe experimentation with git checkpoints**
✅ **Kanban workflow preventing shortcuts**

**Stop fixing AI chaos. Start shipping quality.**

---

## Works With Any AI Agent

While designed for Claude Code, fspec works with:
- GitHub Copilot
- Cursor
- Windsurf
- Any AI agent that can run CLI commands

Just map the `/fspec` command to your agent's command system.

---

## Architecture

**JSON-backed documentation** for machine-readable state:

```
spec/
├── work-units.json              # Work unit state (single source of truth)
├── tags.json                    # Tag registry
├── foundation.json              # Project foundation
├── features/*.feature           # Gherkin specifications
└── features/*.feature.coverage  # Coverage mappings
```

**Design principles:**
- JSON is authoritative, Markdown is documentation
- Auto-generation prevents drift
- Queryable state enables AI agents to reason about project
- Coverage files enable reverse ACDD

---

## License

MIT

---

**[Visit fspec.dev](https://fspec.dev)** | **[GitHub](https://github.com/sengac/fspec)** | **[npm](https://www.npmjs.com/package/@sengac/fspec)**

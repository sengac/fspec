<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**A Spec-Driven Development tool for AI Agents**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## The Problem: AI Agents Need Structure

AI coding agents (like Claude Code, GitHub Copilot) excel at writing code but struggle to build quality software reliably. Without structure, they:

- ❌ **Lose context** between sessions - relying on conversation history instead of queryable state
- ❌ **Skip discovery** - jumping straight to code without understanding requirements
- ❌ **Violate ACDD** - writing code before tests, tests before specs, or skipping phases entirely
- ❌ **Build the wrong thing** - implementing what they think is needed, not what you actually need
- ❌ **Create specification chaos** - malformed Gherkin, inconsistent tags, drifting architecture docs

Flat TODO lists don't help - they show "done" or "not done" but no workflow state, dependencies, or relationships.

## The Solution: Integrated Specification + Project Management

**fspec** provides persistent queryable state with enforced Kanban workflow and collaborative discovery:

- ✅ **Kanban Workflow** - Enforces ACDD: `backlog → specifying → testing → implementing → validating → done`. Cannot skip phases.
- ✅ **Work Units** - Persistent project state (not TODO lists) with status, dependencies, epic relationships, example mapping
- ✅ **Example Mapping** - AI asks clarifying questions, human provides answers - structured discovery before coding
- ✅ **Queryable State** - AI runs `fspec list-work-units --status=specifying` to see what's in flight - doesn't rely on conversation context
- ✅ **Validated Gherkin** - Official @cucumber/gherkin-parser ensures specs are always valid
- ✅ **Tag Discipline** - JSON-backed registry prevents tag chaos
- ✅ **Coverage Tracking** - Link scenarios to test files and implementation code for full traceability
- ✅ **Visual Board** - `fspec board` displays Kanban board showing work across all states

## Why ACDD? (Acceptance Criteria Driven Development)

**ACDD builds on Specification by Example and BDD by enforcing ORDER:**

1. **Acceptance Criteria FIRST** (specs in validated Gherkin)
2. **Tests SECOND** (mapping to scenarios, must fail first)
3. **Code LAST** (minimum implementation to pass tests)

**The Challenge:** AI agents naturally violate ACDD workflow without tooling enforcement.

**The fspec Solution:** Kanban workflow + persistent state + collaborative discovery = Reliable ACDD.

## How It Works

**You don't run fspec commands directly.** Instead, you give high-level requests to your AI agent (like Claude Code), and the agent uses fspec to manage the workflow.

**Important:** You must tell the AI agent to create a work unit and specify the type (story, bug, or task). Otherwise, the agent won't know to create one and kick off the ACDD workflow.

### Work Unit Types

- **Story** - New user-facing feature or functionality
- **Bug** - Something that's broken and needs fixing
- **Task** - Non-user-facing work (refactoring, infrastructure, documentation)

### For New Features (Forward ACDD)

**Tell the agent to create a story:**
```
You: /fspec Create a story for user login feature

Claude: I'll create a story work unit for user login and manage this
        through ACDD workflow:
1. Creating work unit AUTH-001 (type: story) for "User login"
2. Moving to discovery phase - let me ask clarifying questions...
   - Should we support OAuth in addition to email/password?
   - What password requirements do you need?
3. After discovery, I'll generate validated Gherkin specs
4. Then write tests that map to scenarios
5. Finally implement code to pass those tests
```

**Or create a bug:**
```
You: /fspec Create a bug for login session timeout not working

Claude: I'll create a bug work unit for the session timeout issue...
```

**Or create a task:**
```
You: /fspec Create a task to refactor authentication middleware

Claude: I'll create a task work unit for refactoring the middleware...
```

**Or just run `/fspec` alone:**
```
You: /fspec

Claude: Let me check the fspec board...

        Current board shows:
        - 3 items in backlog
        - 1 item in implementing (AUTH-001: User login)
        - 2 items done

        What would you like to work on?
        1. Continue with AUTH-001 (currently implementing)
        2. Start a new work unit
        3. Review the backlog
```

### For Existing Code (Reverse ACDD)
```
You: /rspec Analyze the authentication system in src/auth/

Claude: I'll reverse engineer the existing code:
1. Analyzing routes, API endpoints, and business logic
2. Creating feature files with inferred acceptance criteria
3. Grouping into epics (user-management, security, etc.)
4. Generating test skeletons (structure only, not implemented)
5. Linking existing code to scenarios for traceability
```

**The AI agent handles all fspec commands** - you just provide the high-level intent and work unit type.

## Get Started

### 1. Install fspec

**Via npm (recommended):**

```bash
npm install -g @sengac/fspec
```

**Or from source:**

```bash
git clone https://github.com/sengac/fspec.git
cd fspec
npm install && npm run build && npm run install:local
```

### 2. Initialize in Your Project

```bash
cd /path/to/your/project
fspec init
```

This installs two command files in `.claude/commands/`:
- `/fspec` - For building new features with forward ACDD
- `/rspec` - For reverse engineering existing code

**Works with any AI agent:** While designed for Claude Code, you can use the generated `fspec.md` and `rspec.md` command files with other AI agents by mapping them to your agent's command system. If using another agent, rename `spec/CLAUDE.md` to `spec/AGENTS.md` for clarity.

### 3. Start Building with AI

In Claude Code, **be specific about work unit type** (story, bug, or task):

**For new features (stories):**
```
/fspec Create a story for user authentication feature
/fspec Create a story to add password reset functionality
/fspec Create a story for API rate limiting
```

**For bugs:**
```
/fspec Create a bug for broken session timeout
/fspec Create a bug where password validation allows weak passwords
```

**For tasks:**
```
/fspec Create a task to refactor authentication middleware
/fspec Create a task to update API documentation
```

**Or just run `/fspec` to see the board:**
```
/fspec
```

**For existing code:**
```
/rspec Analyze the entire codebase
/rspec Document the payment processing system
/rspec Create specs for src/api/routes.ts
```

The AI agent will guide you through discovery, ask clarifying questions, generate specs, and enforce ACDD workflow.

## Features

- 📊 **Kanban Workflow** - 7-state workflow with visual board
- 🤝 **Example Mapping** - Collaborative discovery with rules, examples, questions, attachments
- 🔄 **Work Unit Management** - Track work through Kanban with dependencies and epics
- 🔁 **Reverse ACDD** - Reverse engineer existing codebases via `/rspec` command
- 📋 **Gherkin Validation** - Official Cucumber parser ensures valid syntax
- 🔗 **Coverage Tracking** - Link scenarios → tests → implementation (critical for reverse ACDD)
- 🏷️ **JSON-Backed Tag Registry** - Single source of truth with auto-generated docs
- 🎨 **Auto-Formatting** - Custom AST-based formatter for Gherkin files
- 🪝 **Lifecycle Hooks** - Execute custom scripts at command events (quality gates, automation)
- 🤖 **AI Agent Friendly** - Designed for Claude Code integration with persistent queryable state

## Documentation

- 📘 **[Getting Started](./docs/getting-started.md)** - Learn the ACDD workflow
- 📦 **[Installation](./docs/installation.md)** - Setup and requirements
- 📖 **[Usage Guide](./docs/usage.md)** - Complete command reference (for AI agents)
- 🏷️ **[Tag Management](./docs/tags.md)** - Organize features with tags
- 🔗 **[Coverage Tracking](./docs/coverage-tracking.md)** - Link specs to tests and code
- 📊 **[Project Management](./docs/project-management.md)** - Kanban workflow and work units
- 🔁 **[Reverse ACDD](./docs/reverse-acdd.md)** - Reverse engineer existing codebases
- 🪝 **[Lifecycle Hooks](./docs/hooks/configuration.md)** - Automate your workflow

## Who Benefits?

**Developers Using AI Coding Agents**
- Reliable ACDD workflow with persistent state
- Confidence AI is building the right thing

**Teams Practicing BDD/ACDD**
- AI agents that follow methodology rigorously
- Enforced workflow, validated Gherkin, structured discovery

**Product Owners & Stakeholders**
- Clear visibility through Kanban board
- Collaborative discovery ensures right features built

**BDD/Cucumber Ecosystem**
- Promotes standard Gherkin over proprietary formats
- Works with existing Cucumber tooling

## Architecture

fspec uses **JSON-backed documentation** for machine-readable state:

- `spec/work-units.json` - Work unit state (single source of truth)
- `spec/tags.json` - Tag registry (auto-generates TAGS.md)
- `spec/foundation.json` - Project foundation (auto-generates FOUNDATION.md)
- `spec/features/*.feature` - Gherkin specifications
- `spec/features/*.feature.coverage` - Scenario-to-test-to-implementation mappings

## Development

```bash
npm install
npm run build
npm test
npm run dev  # Watch mode
```

## License

MIT

---

**[Visit fspec.dev](https://fspec.dev)** for more information and examples.

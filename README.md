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

## Quick Example

```bash
# AI queries persistent state (not conversation context)
$ fspec show-work-unit AUTH-001
Work Unit: AUTH-001
Status: specifying
Epic: user-management
Rules:
  1. Password must be 8+ characters
  2. Email must be valid format
Examples:
  1. User logs in with valid email user@example.com
  2. Login fails with password "short"
Questions:
  1. Should we support OAuth 2.0? (@human)

# Human answers question
$ fspec answer-question AUTH-001 0 --answer "Yes, via Google OAuth" --add-to rule

# AI generates validated Gherkin from example map
$ fspec generate-scenarios AUTH-001
✓ Created spec/features/user-authentication.feature

# Cannot skip phases - workflow enforced
$ fspec update-work-unit-status AUTH-001 implementing
✗ Error: Cannot move to implementing without entering testing state first

# AI follows proper ACDD workflow
$ fspec update-work-unit-status AUTH-001 testing
✓ Moved to testing - write tests BEFORE implementation
```

## Features

- 📊 **Kanban Workflow** - 7-state workflow with visual board (`fspec board`)
- 🤝 **Example Mapping** - Collaborative discovery with rules, examples, questions, attachments
- 🔄 **Work Unit Management** - Track work through Kanban with dependencies and epics
- 🔁 **Reverse ACDD** - Reverse engineer existing codebases via `/rspec` command in Claude Code
- 📋 **Gherkin Validation** - Official Cucumber parser ensures valid syntax
- 🔗 **Coverage Tracking** - Link scenarios → tests → implementation (critical for reverse ACDD)
- 🏷️ **JSON-Backed Tag Registry** - Single source of truth with auto-generated docs
- 🎨 **Auto-Formatting** - Custom AST-based formatter for Gherkin files
- 🪝 **Lifecycle Hooks** - Execute custom scripts at command events (quality gates, automation)
- 🤖 **AI Agent Friendly** - Designed for Claude Code integration with `/fspec` and `/rspec` commands

## Get Started

### Installation

```bash
npm install -g fspec
fspec init  # Installs /fspec and /rspec commands for Claude Code
```

### Quick Start

```bash
# Validate Gherkin syntax
fspec validate

# Create your first feature
fspec create-feature "User Authentication"

# See the Kanban board
fspec board

# Get comprehensive help
fspec help
fspec help work     # Kanban workflow commands
fspec help discovery # Example mapping commands
```

## Documentation

- 📘 **[Getting Started](./docs/getting-started.md)** - Learn the ACDD workflow
- 📦 **[Installation](./docs/installation.md)** - Setup and requirements
- 📖 **[Usage Guide](./docs/usage.md)** - Complete command reference
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

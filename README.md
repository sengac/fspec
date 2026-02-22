<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**The spec-driven coding agent.**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm](https://img.shields.io/npm/v/@sengac/fspec)](https://www.npmjs.com/package/@sengac/fspec)

---

## What is fspec?

fspec is an AI coding agent that writes specifications, tests, and code—in that order. You describe what you want. fspec asks clarifying questions, generates Gherkin specs, writes failing tests, then implements the code to make them pass.

You don't write the specs. You don't write the tests. You answer questions and review the output. fspec does the hard work.

---

## Quick Start

```bash
npm install -g @sengac/fspec
cd /path/to/your/project
fspec
```

This opens the interactive platform with a Kanban board and AI conversations.

![Interactive Kanban](interactive-kanban.png)

---

## First Run: Getting Started

When you first run `fspec` on a new project, the board is empty—no work units yet. Here's how to begin:

### 1. Start an AI Agent

Press **`/`** (or **Shift+Right**) to start a new AI conversation. A dialog appears:

```
Start New Agent?
Begin a fresh AI conversation, not linked to any task.

Mode:  Normal  / Isolated
```

- **Normal** — Agent works directly in your project
- **Isolated** — Agent works in a git worktree (safe for experimental changes)

Press **Enter** on "Yes" to launch the agent.

### 2. Use It However You Want

**fspec doesn't force any workflow.** The AI agent is a full-featured coding assistant. You can:

- Ask it to write code, refactor, debug, or explain things
- Have it review PRs, write documentation, or answer questions
- Use it exactly like any other AI coding tool

The ACDD workflow is available when you want it, not required. fspec provides the tools—you decide how to use them.

### 3. Foundation Discovery (When Using ACDD)

If you want to use the spec-driven workflow, start with **Foundation Discovery**. For new projects without `spec/foundation.json`, tell the AI:

```
"Let's set up fspec for this project"
"Run fspec discover-foundation"
```

The AI guides you through creating your project's requirements document:

- Analyzes your codebase
- Asks about project vision, personas, and capabilities
- Builds `foundation.json` field by field
- Finalizes with `fspec discover-foundation --finalize`

This is a one-time setup that establishes project context for the ACDD workflow.

### 4. Create Work Units

Once foundation exists (or skip it for quick tasks), tell the AI what you want:

```
"Create a story for user authentication"
"I need to add a payment processing feature"
"There's a bug where login fails on mobile"
```

The AI creates work units (stories, bugs, or tasks) and adds them to your backlog.

### 5. Work the Kanban Board

Now you have cards on the board! The workflow is:

```
BACKLOG → SPECIFYING → TESTING → IMPLEMENTING → VALIDATING → DONE
```

- Press **Enter** on any card to work on it with the AI
- The AI moves cards through stages automatically
- Each stage has specific goals (see "How It Works" below)

Or ignore the board entirely and just chat with the agent—it's your choice.

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Start new AI agent |
| **Shift+Right** | Navigate to next session (or create new) |
| **Shift+Left** | Navigate to previous session (or back to board) |
| **Enter** | Open selected work unit with AI |
| **↑ ↓ ← →** | Navigate board |
| **C** | View git checkpoints |
| **F** | View changed files |
| **D** | View FOUNDATION.md |
| **Esc** | Exit / Go back |

---

## How It Works

1. **You describe what you want** — A feature, a bug fix, a task
2. **fspec asks questions** — Clarifies edge cases, rules, and expectations
3. **fspec writes specs** — Generates Gherkin scenarios from your answers
4. **fspec writes tests** — Failing tests that prove the spec isn't implemented
5. **fspec writes code** — Just enough to make the tests pass
6. **Coverage tracks everything** — Every line of code links back to a requirement

This is **Acceptance Criteria Driven Development (ACDD)**. The agent drives. You steer.

---

## Using with External Agents

fspec also works as a tool for Claude Code, Cursor, Codex, or any AI agent:

```bash
cd /path/to/your/project
fspec init
```

This installs agent-specific documentation and slash commands. Then tell your agent:

```
"Run fspec bootstrap"
"Create a story for user authentication"
"Show me the board"
```

The agent learns fspec's workflow and manages your specs automatically.

---

## Key Capabilities

- **Example Mapping** — Agent discovers rules, examples, and edge cases by asking you questions
- **Gherkin generation** — Specs written automatically from your answers
- **Test-first development** — Tests before code, always
- **Kanban workflow** — Track work from backlog to done
- **Git checkpoints** — Automatic save points for safe experimentation
- **Coverage tracking** — Link code to requirements
- **Multiple sessions** — Run concurrent AI conversations in background
- **Isolated sessions** — Work in git worktrees for safe experimentation

---

## Dogfooding

fspec was built using fspec. **432 feature files** with complete Gherkin specifications. What would take a QA team 9-12 months took weeks with spec-driven AI development.

Browse the specs: [spec/features](https://github.com/sengac/fspec/tree/main/spec/features)

---

## Links

**[fspec.dev](https://fspec.dev)** · **[GitHub](https://github.com/sengac/fspec)** · **[npm](https://www.npmjs.com/package/@sengac/fspec)**

---

## Professional Services

Legacy codebase? Untested? Undocumented? [SENGAC](https://sengac.com) transforms systems into fully-specified, AI-tested platforms using fspec.

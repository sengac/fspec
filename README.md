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
fspec init
fspec
```

This opens the interactive platform with a Kanban board, AI conversations, and live spec validation.

![Interactive Kanban](interactive-kanban.png)

Press **Enter** on any work unit to start coding with the agent.

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

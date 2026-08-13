<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**The Spec-Driven, Multi-Agent Harness**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## What is fspec?

**fspec** (Factory Spec) is infrastructure for running a software factory—multiple AI agents working jobs in parallel, driven by specifications, managed on a Kanban board.

This isn't another agent harness. It's a **coding factory**.

### How fspec Differs from Agent Harnesses

Many agent harnesses support multiple agents and basic planning. The difference is **how work gets broken down and executed**.

In a typical agent harness, you describe a task and the agent writes code. Planning is informal, code is produced directly, and there's no structural guarantee that the output matches your intent. You can add formal planning via external frameworks (BMAD, GSD, etc.), but those frameworks are bolted on — they don't enforce a spec-first, test-first pipeline or tie every line of code back to a verifiable requirement.

fspec uses **Acceptance Criteria Driven Development (ACDD)** — a disciplined pipeline where every feature must pass through specification, testing, and implementation in order:

1. **Features on a board** — Work is broken into user-facing capabilities, each tracked as a work unit on a Kanban board
2. **Specification first** — Before any code, the agent writes Gherkin scenarios capturing exactly what the feature should do, asking clarifying questions to uncover edge cases
3. **Tests before code** — Failing tests are written from the spec, proving the agent understands the requirements
4. **Minimal implementation** — Just enough code to make the tests pass, nothing more

This discipline produces code that is **robust** (every behavior is tested), **isolated** (each feature is independently specified and tested), and **behavior-driven** (the spec is the source of truth, not the code).

| | Agent Harness | fspec (ACDD) |
|---|---|---|
| **Work breakdown** | Free-form prompts or tasks | Features as user capabilities on a Kanban board |
| **Spec → code** | Code written directly from prompts | Gherkin spec → failing tests → minimal implementation |
| **Quality guarantee** | Depends on agent judgment | Every acceptance criterion has a failing test before code is written |
| **Traceability** | None or informal | Every line of code links back to a Gherkin scenario |
| **Scope control** | Agents tend to over-implement | Tests dictate exactly what gets implemented |
| **Concurrency** | Optional, unstructured | Structured — each agent pulls a specified work unit from the board |

In a factory, you don't stand at every machine. You design the product, write the specifications, and let the production line run. You check output quality. You fix bottlenecks. But the machines do the work.

### Why Kanban?

In the 1940s, Toyota engineer Taiichi Ohno faced a problem: American car factories outproduced Japanese ones by a factor of ten. His solution was **Kanban**—a visual system where cards represent work items flowing through production stages. Each station pulls work when ready. Nothing gets built without a card. Nothing moves forward until quality checks pass.

Toyota's production system revolutionized manufacturing. The Kanban board became the heartbeat of the factory floor.

fspec applies the same principle to software production. Work units are jobs. The Kanban board is your production floor. AI agents are workstations. Specifications are blueprints. Tests are quality control. The factory runs whether you're watching or not.

### The Dark Factory

The term comes from [Dan Shapiro's framework](https://www.danshapiro.com/blog/2026/01/the-five-levels-from-spicy-autocomplete-to-the-software-factory/) mapping AI-assisted coding to five levels of autonomy—borrowing from self-driving cars. Most developers operate at Levels 2-3: pair programming with AI, reviewing AI-generated code. fspec is built for **Levels 4 and 5**, where specifications become the primary human input and AI agents autonomously produce working software.

The "dark factory" references the [Fanuc factory in Japan](https://en.wikipedia.org/wiki/Lights_out_(manufacturing))—a robot factory staffed by robots, running with the lights off because no humans are present. In software, it means: specs go in, code comes out. The factory runs in the dark.

fspec makes this possible through **Acceptance Criteria Driven Development (ACDD)**: you describe what you want, the AI asks clarifying questions, writes Gherkin scenarios capturing your intent, generates failing tests, then writes just enough code to pass. Every line traces back to a requirement. The specification *is* the source of truth.

---

## Supported Providers

fspec works with any AI provider that supports tool calling. See [docs/PROVIDERS.md](docs/PROVIDERS.md) for the full list and configuration.

> **⚠️ Subscription Tokens**: Some tokens (like `CLAUDE_CODE_OAUTH_TOKEN`) come from subscription services rather than pay-per-use APIs. Check your provider's terms of service before using subscription tokens with third-party tools.

---

## Quick Start

### Install

```bash
# macOS / Linux — build and install from source:
./scripts/install.sh

# Or build manually:
./scripts/build.sh --package
cp dist/fspec-*.tar.gz ~/.local/bin/
```

### Run

```bash
cd /path/to/your/project
fspec
```

This opens the factory floor—your Kanban board with AI workstations ready to take jobs.

![Interactive Kanban](interactive-kanban.png)

> **Building from source?** See [BUILD.md](docs/BUILD.md) for complete build instructions,
> cross-compilation, and the `release-slim` profile rationale.

---

---

## First Run: Starting the Factory

When you first run `fspec`, the production floor is empty—no jobs queued. Here's how to get the factory running:

### 1. Spin Up an Agent

Press **`/`** (or **Shift+Right**) to start a new AI workstation. A dialog appears:

```
Start New Agent?
Begin a fresh AI conversation, not linked to any task.

Mode:  Normal  / Isolated
```

- **Normal** — Agent works directly in your project
- **Isolated** — Agent works in a git worktree (safe for experimental changes)

Press **Enter** on "Yes" to bring the workstation online.

### 2. Use It However You Want

**fspec doesn't force any workflow.** Each AI agent is a full-featured coding assistant. You can:

- Ask it to write code, refactor, debug, or explain things
- Have it review PRs, write documentation, or answer questions
- Use it exactly like any other AI coding tool

The factory workflow is available when you want it, not required. fspec provides the infrastructure—you decide how to run your production line.

### 3. Foundation Discovery (Setting Up the Factory)

To run the full factory workflow, start with **Foundation Discovery**. This establishes your product blueprint. For new projects without `spec/foundation.json`, tell the AI:

```
"Let's set up fspec for this project"
"Run fspec discover-foundation"
```

The AI guides you through creating your project's requirements document:

- Analyzes your codebase
- Asks about project vision, personas, and capabilities
- Builds `foundation.json` field by field
- Finalizes with `fspec discover-foundation --finalize`

This is a one-time setup that establishes the blueprint for all future production.

### 4. Queue Jobs

Once the foundation exists (or skip it for quick tasks), tell the AI what you want to build:

```
"Create a story for user authentication"
"I need to add a payment processing feature"
"There's a bug where login fails on mobile"
```

The AI creates work units (stories, bugs, or tasks) and queues them in the backlog. These are your production jobs.

### 5. Run the Production Line

Now you have jobs on the board! Work flows through the factory:

```
BACKLOG → SPECIFYING → TESTING → IMPLEMENTING → VALIDATING → DONE
```

- Press **Enter** on any job to assign it to an agent
- Agents move jobs through stages automatically
- Each stage has quality gates (see "How the Factory Works" below)

Multiple agents can work different jobs simultaneously—one implements a feature while another fixes a bug. This is parallel production.

Or ignore the board entirely and just chat with an agent—the factory infrastructure is there when you need it.

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Spin up new AI agent |
| **Shift+Right** | Navigate to next session (or create new) |
| **Shift+Left** | Navigate to previous session (or back to board) |
| **Enter** | Assign selected job to an agent |
| **↑ ↓ ← →** | Navigate board |
| **C** | View git checkpoints |
| **F** | View changed files |
| **D** | View FOUNDATION.md |
| **Esc** | Exit / Go back |

---

## How the Factory Works

1. **You specify what to build** — A feature, a bug fix, a task
2. **The agent asks questions** — Clarifies edge cases, rules, and expectations
3. **The agent writes specs** — Generates Gherkin scenarios from your answers
4. **The agent writes tests** — Failing tests that prove the spec isn't implemented
5. **The agent writes code** — Just enough to make the tests pass
6. **Quality control** — Every line of code links back to a requirement

This is **Acceptance Criteria Driven Development (ACDD)**. Specifications are blueprints. Tests are quality control. Code is the product. The factory produces software that provably meets requirements.

---

## Using with External Agents

fspec works as tooling for Claude Code, Cursor, Codex, or any AI agent. See [docs/EXTERNAL.md](docs/EXTERNAL.md) for setup instructions.

---

## Security

fspec agents have full access to your file system, network, and shell. See [docs/SECURITY.md](docs/SECURITY.md) for sandboxing recommendations and ExitBox integration.

---

## Command & File Blocklist

Block, allow, or prompt for approval on specific commands and file access patterns. See [docs/BLOCKLIST.md](docs/BLOCKLIST.md) for configuration details.

---

## Key Capabilities

- **Parallel production** — Multiple agents working different jobs simultaneously
- **Example Mapping** — Agents discover rules, examples, and edge cases by asking questions
- **Gherkin generation** — Specs written automatically from your answers
- **Test-first development** — Tests before code, always
- **Kanban workflow** — Toyota-style production flow from backlog to done
- **Git checkpoints** — Automatic save points for safe experimentation
- **Coverage tracking** — Link code to requirements
- **Isolated sessions** — Work in git worktrees for safe experimentation

---

## Scaling Work

AgentManager, SessionSearch, and DeepSearch enable parallel work across multiple agents. See [docs/SCALING.md](docs/SCALING.md) for details.

---

## Telegram Bridge

Monitor and interact with your factory from your phone via Telegram. See [docs/TELEGRAM.md](docs/TELEGRAM.md) for setup instructions.

---

## Isolated Sessions & Worktrees

Work in git worktrees for safe experimentation and parallel development. See [docs/WORKTREES.md](docs/WORKTREES.md) for details.

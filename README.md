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

In a typical agent harness, you describe a task and the agent writes code. Planning is informal, code is produced directly, and there's no structural guarantee that the output matches your intent.

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

fspec works with any AI provider that supports tool calling. Set your API key and start the factory:

| Provider | Environment Variable |
|----------|---------------------|
| **Anthropic** | `ANTHROPIC_API_KEY` or `CLAUDE_CODE_OAUTH_TOKEN` (run `claude setup-token`) |
| **Google Gemini** | `GOOGLE_GENERATIVE_AI_API_KEY` |
| **OpenAI** | `OPENAI_API_KEY` |
| **Codex** | OAuth (automatic) |
| **xAI** | `XAI_API_KEY` |
| **DeepSeek** | `DEEPSEEK_API_KEY` |
| **Z.AI** | `ZAI_API_KEY` |
| **Mistral** | `MISTRAL_API_KEY` |
| **Groq** | `GROQ_API_KEY` |
| **OpenRouter** | `OPENROUTER_API_KEY` |
| **Together AI** | `TOGETHER_API_KEY` |
| **Azure OpenAI** | `AZURE_OPENAI_API_KEY` |

**OpenAI-compatible APIs** — Ollama, vLLM, LM Studio, and any server implementing the OpenAI API format work via the OpenAI provider with `OPENAI_API_KEY`.

Configure providers with `/provider` in any session.

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

## Rust Binary Architecture

The `fspec` binary is a pure-Rust application built in the `rust/` directory.
It provides the TUI, WebSocket server, and ACDD command surface as a single
self-contained executable — no Node.js runtime required.

### Three Operating Modes

The binary supports three modes via clap subcommands:

| Mode | Command | Description |
|------|---------|-------------|
| **Combined** (default) | `fspec` | TUI + always-on WebSocket server in one process |
| **Daemon** | `fspec daemon` | Headless WebSocket server only (suitable for systemd / launchd) |
| **Client** | `fspec client` | Frontend-only; connects to a running daemon via WebSocket |

### Workspace Structure

The Rust codebase is organized as a Cargo workspace with 21 crates:

| Crate | Purpose |
|-------|---------|
| `codelet-fspec` | **Main binary** — CLI entry point, clap subcommands, mode selection |
| `codelet-fspec-core` | Command implementations — pure-Rust port of all fspec CLI commands |
| `codelet-fspec-tui` | Terminal UI — ratatui-based Kanban board, session views, agent interaction |
| `codelet-agent-loop` | LLM agent loop — drains input, dispatches to LlmProvider, emits streaming output |
| `codelet-sessions` | Session management — NAPI-free SessionManager + BackgroundSession |
| `codelet-core` | Persistence, compaction, lifecycle hooks, token tracking, scheduler |
| `codelet-common` | Shared utilities — data directory, file locking, logging, config |
| `codelet-providers` | LLM provider integrations — Anthropic, OpenAI, Gemini, Codex, etc. |
| `codelet-rpc` | RPC framework — tarpc-based in-process and WebSocket transports |
| `codelet-rpc-server` | WebSocket RPC server — headless daemon mode |
| `codelet-rpc-types` | Shared RPC types and message definitions |
| `codelet-rpc-embedded` | Embedded transport — in-process tarpc channel |
| `codelet-tools` | AI tool implementations — Read, Write, Bash, Grep, AstGrep, etc. |
| `codelet-graph` | Knowledge graph — AST indexing, concept relationships |
| `codelet-git` | Git operations — checkpoints, worktrees, branch management |
| `codelet-cli` | CLI utilities — context gathering, interactive helpers, terminal output |
| `codelet-tui` | TUI components — widgets, layouts, state management |
| `codelet-attachment-viewer` | Axum HTTP server for serving project attachments |
| `codelet-fspec-json-error` | JSON error formatting — human-friendly diagnostics |
| `codelet-test-helpers` | Shared test utilities for integration tests |
| `codelet-napi` | Node.js NAPI bindings — thin adapter for legacy JS integration |

### Command Architecture

All fspec CLI commands are implemented in `codelet-fspec-core`. Each command
exposes a single entry point:

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

This function serves as the **single source of truth** for both:

1. **The LLM-facing dispatcher** — called by the agent loop when the AI agent invokes an fspec tool
2. **The standalone CLI** — called by the `fspec` binary when invoked from the shell

This "two front doors, one source of truth" pattern ensures business logic is
never duplicated between the agent-facing and shell-facing interfaces.

### Distribution

The standalone `fspec` binary is a self-contained executable:
- **No Node.js runtime** needed
- **No `.node` files** or NAPI bindings
- **~150 MB** distribution size (using `release-slim` profile)

Build and run directly:

```bash
cd rust
cargo build --profile release-slim -p codelet-fspec
./target/release-slim/fspec --version
```

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

fspec also works as tooling for Claude Code, Cursor, Codex, or any AI agent:

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

The agent learns the factory workflow and manages production automatically.

---

## ⚠️ Security: Running in a Sandbox

**fspec agents have full access to your file system, network, and shell.** They can read, write, and execute anything your user account can. This is by design—agents need these capabilities to write code, run tests, and manage your project.

However, this means a compromised or misbehaving agent could:
- Read sensitive files (SSH keys, credentials, other projects)
- Make network requests to arbitrary endpoints
- Execute destructive commands

### Recommended: Use ExitBox

[ExitBox](https://github.com/cloud-exit/exitbox) runs AI agents in isolated containers with defense-in-depth security:

- **Network firewall** — Agents can only reach allowlisted domains
- **File isolation** — Only your project directory is mounted
- **Capability restrictions** — No raw sockets, no privilege escalation
- **Credential protection** — SSH keys and cloud credentials are not exposed

### Quick Setup

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/setup-sandbox.ps1 | iex
```

### What Gets Restricted

| Resource | Without Sandbox | With ExitBox |
|----------|-----------------|--------------|
| File system | Full access | Only `/workspace` (your project) |
| Network | Unrestricted | Allowlisted domains only |
| SSH keys | Accessible | Hidden (unless `--full-git-support`) |
| Other projects | Accessible | Isolated |
| System commands | Full shell | Restricted capabilities |

### When to Skip the Sandbox

If you're running fspec on throwaway VMs, CI environments, or fully trust the agent, you can run directly. The sandbox adds a small amount of overhead and complexity.

For local development on your primary machine, **the sandbox is strongly recommended**.

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

## Scaling Work: AgentManager, SessionSearch & DeepSearch

Three tools work together to scale beyond single-threaded development:

### AgentManager — Parallel Worker Sessions

Spawn subordinate AI agents with full tool access. Each worker inherits the supervisor's model and runs independently:

- **spawn** — Create a new worker session with an optional role (e.g., "security reviewer")
- **message** — Send tasks to workers, with optional context references from other sessions
- **list / get_status** — Monitor worker progress
- **await_idle** — Block until workers finish (instead of polling)
- **close** — Terminate workers when done

```
AgentManager(action='spawn', role='Security reviewer')
# → { session_id: 'abc-123' }

AgentManager(action='message', session_id='abc-123',
  message='Review src/auth/ for vulnerabilities')

AgentManager(action='await_idle', session_id='abc-123')
# Blocks until the worker finishes

AgentManager(action='close', session_id='abc-123')
```

### SessionSearch — Cross-Session Memory

Search and view conversation history across all sessions. Workers use SessionSearch to PULL context from their supervisor:

- **recent** — List recent sessions for discovery
- **search** — Keyword search with regex across all content (user inputs, responses, tool calls)
- **show** — Load a specific session's conversation

```
SessionSearch(action='recent', count=5)
# → List of recent sessions with timestamps

SessionSearch(action='search', query='authentication', last_hours=24)
# → Matches with surrounding context

SessionSearch(action='show', session_id='abc-123', max_turns=20)
# → Conversation history for drill-down
```

### DeepSearch — Ephemeral Research Sub-Agents

Spawn a read-only sub-agent that explores a scoped corpus (code files or session history) and returns a synthesized answer:

```
DeepSearch(query='How is authentication handled?', scope='src/auth/')
# → Sub-agent explores the directory and returns findings

DeepSearch(query='What was decided about the database schema?')
# → Searches session history only (no code scope)
```

### How They Work Together

| Tool | Use When | Persistence |
|------|----------|-------------|
| **SessionSearch** | Recall decisions, pull context from another agent | Reads existing data |
| **DeepSearch** | Answer research questions requiring many file reads | Ephemeral (no persistence) |
| **AgentManager** | Parallel workers doing real work — writing code, running tests | Full session (searchable) |

**Typical pattern:**

1. Supervisor spawns workers via AgentManager
2. Workers use SessionSearch to pull context from the supervisor
3. Workers use DeepSearch for codebase research
4. Supervisor uses `await_idle` to wait for results
5. Workers close when done

This enables factory-scale parallelism: one agent implements a feature while another reviews security, all sharing context through SessionSearch.

---

## Telegram Bridge

Monitor and interact with your factory from your phone via Telegram. See [docs/TELEGRAM.md](docs/TELEGRAM.md) for setup instructions.

---

## Isolated Sessions & Worktrees

Work in git worktrees for safe experimentation and parallel development. See [docs/WORKTREES.md](docs/WORKTREES.md) for details.

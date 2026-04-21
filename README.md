<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**The Spec-Driven, Multi-Agent Coding Factory**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm](https://img.shields.io/npm/v/@sengac/fspec)](https://www.npmjs.com/package/@sengac/fspec)

---

## What is fspec?

**fspec** (Factory Spec) is infrastructure for running a software factory—multiple AI agents working jobs in parallel, driven by specifications, managed on a Kanban board.

This isn't another coding assistant. It's a **coding factory**.

### Coding Agent vs. Coding Factory

A **coding agent** is a single AI that helps you write code. You chat with it, it writes some code, you review it, repeat. One conversation, one task, one developer. It's pair programming with a robot.

A **coding factory** is fundamentally different:

| | Coding Agent | Coding Factory |
|---|---|---|
| **Concurrency** | One conversation at a time | Multiple agents working simultaneously |
| **Input** | Informal chat prompts | Structured specifications |
| **Workflow** | Ad-hoc | Systematic (backlog → done) |
| **Traceability** | None | Every line links to a requirement |
| **Overnight work** | Not practical | Agents run while you sleep |
| **Scale** | One developer's productivity | Factory-level throughput |

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

```bash
npm install -g @sengac/fspec
cd /path/to/your/project
fspec
```

This opens the factory floor—your Kanban board with AI workstations ready to take jobs.

![Interactive Kanban](interactive-kanban.png)

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

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/setup-sandbox.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/setup-sandbox.ps1 | iex
```

**Or manually:**
```bash
# 1. Install ExitBox
mkdir -p ~/.local/bin
curl -fsSL https://github.com/Cloud-Exit/ExitBox/releases/latest/download/exitbox-$(uname -s | tr A-Z a-z)-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/') -o ~/.local/bin/exitbox
chmod +x ~/.local/bin/exitbox

# 2. Run setup wizard
exitbox setup
# Select "node" in development profiles

# 3. Run factory in sandbox
cd /path/to/your/project
exitbox run claude

# 4. Inside container, install fspec
npm install -g @sengac/fspec
fspec
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

fspec includes a blocklist system that can block, allow, or prompt for approval on specific commands and file access patterns. This provides fine-grained control over what agents can do—without requiring a full sandbox.

### Configuration Files

| Location | Purpose |
|----------|---------|
| `~/.fspec/blocklist.json` | System-wide rules (apply to all projects) |
| `.fspec/blocklist.json` | Project-specific rules (override system rules) |

### Config Structure

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "git-checkout-block",
      "pattern": "^git\\s+checkout\\b",
      "action": "block",
      "reason": "git checkout is deprecated",
      "guidance": "Use git switch instead"
    },
    {
      "id": "ssh-config-prompt",
      "pattern": "\\.ssh",
      "action": "prompt",
      "reason": "SSH directory may contain sensitive keys"
    },
    {
      "id": "allow-node-modules-rm",
      "pattern": "^rm\\s+-rf\\s+\\./node_modules\\b",
      "action": "allow",
      "reason": ""
    }
  ]
}
```

### Actions

| Action | Behavior |
|--------|----------|
| **block** | Immediately reject. The AI receives the `reason` and `guidance` as an error message. |
| **prompt** | Pause and ask the user. Shows a triple-choice dialog: **Allow Once**, **Allow Session**, or **Deny**. |
| **allow** | Explicitly permit. Used to override a more general blocking rule. |

### How Rules Are Evaluated

1. **Project rules first** — `.fspec/blocklist.json` rules are checked before system rules
2. **First match wins** — Evaluation stops at the first matching pattern
3. **Allow overrides block** — A project `allow` rule can override a system `block` rule

This means you can have a system-wide rule blocking `rm -rf` but allow it specifically for `./node_modules` in a project config.

### Pattern Syntax

Patterns use **regex**. Common patterns:

```json
"^git\\s+checkout\\b"     // Command starts with "git checkout"
"\\.env"                   // Path contains ".env"
"^rm\\s+-rf\\b"           // Command starts with "rm -rf"
"~/.ssh"                   // Path contains "~/.ssh"
```

### What Gets Checked

- **Bash tool** — Command string is checked before execution
- **Read/Write/Edit tools** — File path is checked before access

### Session Allowances

When a user selects **Allow Session** on a prompt, that pattern is remembered for the current session. The agent can access matching resources without re-prompting until the TUI is restarted.

### Example: Protecting Sensitive Files

System config (`~/.fspec/blocklist.json`):
```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "ssh-prompt",
      "pattern": "\\.ssh",
      "action": "prompt",
      "reason": "SSH keys are sensitive credentials"
    },
    {
      "id": "env-prompt", 
      "pattern": "\\.env",
      "action": "prompt",
      "reason": "Environment files may contain secrets"
    },
    {
      "id": "aws-prompt",
      "pattern": "\\.aws",
      "action": "prompt", 
      "reason": "AWS credentials directory"
    }
  ]
}
```

### Example: Enforcing Tool Usage

Block agents from using shell commands when proper tools exist:

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "cat-block",
      "pattern": "^cat\\s+",
      "action": "block",
      "reason": "Use the Read tool for file reading, not Bash",
      "guidance": "The Read tool provides proper encoding and line numbers"
    },
    {
      "id": "echo-redirect-block",
      "pattern": "echo.*>",
      "action": "block",
      "reason": "Use the Write tool for file writing, not Bash",
      "guidance": "The Write tool handles encoding and creates parent directories"
    }
  ]
}
```

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
- **Watcher sessions** — Supervisor agents that review production in real-time

---

## Telegram Bridge

Monitor and interact with your factory from your phone. The Bridge tool connects any session to external WebSocket endpoints, with a built-in Telegram integration.

### Setup

1. **Create a Telegram bot** — Message [@BotFather](https://t.me/botfather), send `/newbot`, get your token

2. **Configure the bridge** — Create `bridge/.env`:
   ```bash
   TELEGRAM_BOT_TOKEN=your_token_here
   TELEGRAM_ALLOWED_USER_IDS=123456789   # Your Telegram user ID (optional but recommended)
   ```

3. **Start the endpoint**:
   ```bash
   npm run bridge:telegram
   ```

4. **Message your bot** — Send any message to link your chat

5. **Connect the agent** — Tell it:
   ```
   Connect to the Telegram bridge at ws://localhost:8181
   ```

Now all agent output streams to Telegram. Send messages back to provide input. Run the factory overnight and check production from bed.

### Security: User Whitelist

By default, anyone who finds your bot can interact with it. Set `TELEGRAM_ALLOWED_USER_IDS` to restrict access:

```bash
# Single user
TELEGRAM_ALLOWED_USER_IDS=123456789

# Multiple users (comma-separated)
TELEGRAM_ALLOWED_USER_IDS=123456789,987654321
```

To find your Telegram user ID, message [@userinfobot](https://t.me/userinfobot) or check the bridge console output when you send a message.

### Multiple Bridges

Connect to multiple endpoints simultaneously—Telegram, Slack, Discord, or any WebSocket server. Each bridge receives the same stream.

### Mobile App (Coming Soon)

A dedicated mobile app for iOS and Android is in development at [github.com/sengac/fspec.app](https://github.com/sengac/fspec.app). It connects to fspec via the Bridge protocol, providing a native interface for monitoring production, sending input, and managing jobs from your phone.

---

## WebMCP Chrome Extension

The fspec Browser Agent Chrome Extension bridges your browser to AI agents via the [Model Context Protocol (MCP)](https://modelcontextprotocol.io). It exposes browser control tools and discovers [WebMCP](https://developer.chrome.com/blog/webmcp-epp) tools registered by websites—all accessible through a standard MCP connection.

### What It Does

Once connected, your AI agent can:

- **Control the browser** — Navigate tabs, take screenshots, click elements, fill forms, execute JavaScript
- **Discover website tools** — Websites using Chrome's [WebMCP API](https://developer.chrome.com/blog/webmcp-epp) (`navigator.modelContext.registerTool()`) automatically appear as callable tools
- **Receive browser events** — Tab navigation, page loads, tab creation/closure arrive as real-time MCP notifications via SSE

### Architecture

```
AI Agent (fspec)
  ↕ ConnectMCP (HTTP)
Native Messaging Host (Node.js, port 19876)
  ↕ stdin/stdout (Chrome native messaging protocol)
Service Worker (Chrome extension)
  ↕ chrome.runtime / chrome.tabs
Content Script (isolated world relay)
  ↕ window.postMessage
Main-World Script (WebMCP tool discovery & invocation)
```

### Prerequisites

- **Node.js** 18+ (for the native messaging host)
- **Google Chrome** 120+ (for browser control tools)
- **Google Chrome 146+** with WebMCP flag enabled (only needed for WebMCP website tool discovery)

### Installation

#### 1. Build the extension

```bash
cd extension
npm install
npm run build
```

This produces the loadable extension in the `extension/` directory (with built files in `extension/dist/`).

#### 2. Load the extension in Chrome

1. Open Chrome and navigate to `chrome://extensions/`
2. Enable **Developer mode** using the toggle in the top-right corner
3. Click **Load unpacked**
4. Select the `extension/` directory from the fspec repository
5. Note the **extension ID** shown on the extension card (e.g., `abcdefghijklmnopqrstuvwxyz`)

#### 3. Register the native messaging host

The native messaging host is a Node.js process that Chrome launches automatically when the extension connects. You need to register it once so Chrome knows where to find it:

```bash
node extension/host/native-host.mjs --register --extension-id <your-extension-id>
```

Replace `<your-extension-id>` with the ID from step 2.

This writes a `com.fspec.browser.agent.json` manifest to Chrome's native messaging host directory:

| Platform | Manifest Location |
|----------|-------------------|
| **macOS** | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` |
| **Linux** | `~/.config/google-chrome/NativeMessagingHosts/` |
| **Windows** | `%LOCALAPPDATA%\Google\Chrome\User Data\NativeMessagingHosts\` |

#### 4. Enable WebMCP (optional, for website tool discovery)

If you want AI agents to discover tools registered by websites via `navigator.modelContext`:

1. Navigate to `chrome://flags`
2. Search for **WebMCP for testing**
3. Set it to **Enabled**
4. Relaunch Chrome

This is only needed for WebMCP tool discovery. All native browser control tools work without this flag.

### Connecting from fspec

Once installed, tell your AI agent:

```
Connect to the Chrome extension at http://localhost:19876/mcp
```

Or the agent can call the ConnectMCP tool directly:

```
ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")
```

The extension popup (click the extension icon) shows the current status: server status, port, connected clients, and available tools grouped by source.

### Available Tools

The extension provides 11 native browser control tools out of the box:

| Tool | Description |
|------|-------------|
| `browser_navigate` | Navigate a tab to a URL |
| `browser_screenshot` | Capture a screenshot of a tab |
| `browser_list_tabs` | List all open tabs with IDs, URLs, and titles |
| `browser_execute_script` | Execute JavaScript in a tab |
| `browser_switch_tab` | Activate a tab and focus its window |
| `browser_close_tab` | Close a tab |
| `browser_get_page_content` | Get page content as text or HTML |
| `browser_click_element` | Click an element by CSS selector |
| `browser_fill_form` | Fill a form field by CSS selector |
| `browser_go_back` | Navigate back in browser history |
| `browser_go_forward` | Navigate forward in browser history |

WebMCP tools from websites appear dynamically with the naming pattern `webmcp__<hostname>__<toolName>` (e.g., `webmcp__travel-demo.bandarra.me__searchFlights`).

### Browser Event Notifications

When connected via SSE, the agent receives real-time notifications:

| Event | Method | Params |
|-------|--------|--------|
| Page navigation | `notifications/browser/navigation` | `tabId`, `url`, `title` |
| Page loaded | `notifications/browser/load_complete` | `tabId`, `url`, `title` |
| Tab created | `notifications/browser/tab_created` | `tabId`, `url` |
| Tab closed | `notifications/browser/tab_closed` | `tabId` |
| Tool list changed | `notifications/tools/list_changed` | *(none)* |

### Custom Port

The native host defaults to port 19876. To use a different port:

```bash
node extension/host/native-host.mjs --port 8080
```

---

## Watcher Sessions

Watchers are supervisor agents that observe production in real-time and automatically interject with feedback. Think of them as quality inspectors on the factory floor.

### Use Cases

- **Security Reviewer** — Watches for SQL injection, XSS, authentication issues
- **Test Enforcer** — Ensures tests are written before implementation  
- **Architecture Advisor** — Suggests patterns and flags structural problems
- **Documentation Checker** — Ensures code changes include doc updates

### Creating a Watcher

Type `/watcher` in any session to open the watcher overlay. Press **N** to create a new template:

- **Name** — Role name like "Security Reviewer"
- **Authority** — Peer (suggestions) or Supervisor (directives)
- **Model** — Which AI model to use
- **Brief** — Instructions for what to watch for
- **Auto-inject** — Whether to automatically send feedback to the production agent

### How It Works

1. Watcher observes production agent output in real-time
2. At breakpoints (tool results, turn completion), watcher evaluates what it saw
3. If the watching brief is triggered, watcher decides to interject or continue
4. **Auto-inject ON**: Feedback automatically appears in production session as a purple message
5. **Auto-inject OFF**: You review the feedback and manually inject if desired

### Split View

When viewing a watcher session, the screen splits:

- **Left pane** — Production session (read-only, dimmed)
- **Right pane** — Watcher conversation (interactive)
- **←/→ arrows** — Switch between panes
- **Tab** — Select specific turns to discuss

### Templates

Watcher configurations are saved as reusable templates in `~/.fspec/watcher-templates.json`. Spawn instances quickly with `/watcher spawn <slug>` or press Enter on any template in the overlay.

---

## Isolated Sessions & Worktrees

When you start a new agent, you can choose between **Normal** and **Isolated** mode:

- **Normal** — Agent works directly in your project directory
- **Isolated** — Agent works in a git worktree (separate directory, same repository)

### Why Isolated Sessions?

Isolated sessions provide safe experimentation without risking your main codebase:

- **Parallel development** — Multiple agents can work on different features simultaneously
- **Safe experimentation** — Changes are contained in a separate worktree
- **Easy rollback** — Discard changes without affecting the main project
- **Clean merges** — Apply changes back only when you're satisfied

### How It Works

1. **Create an isolated session** — Press `/` and select "Isolated" mode
2. **Work normally** — The agent sees the worktree as its project root
3. **Review changes** — All files are modified in the isolated worktree
4. **Merge or discard** — When finished, merge changes back or discard them

### Merging Changes

To merge isolated session changes back to the main project:

```
/merge-worktree
```

This command:

1. **Checks for conflicts** — Detects if files changed in both session and main project
2. **Applies changes** — Copies modifications, additions, and deletions to main worktree
3. **Shows summary** — Displays files modified, added, and deleted
4. **Closes session** — Returns you to the board view

### Conflict Handling

If conflicts are detected:

- The merge is **not applied**
- Conflict details are shown in the chat
- The session remains **active** so you can resolve conflicts
- After resolving, run `/merge-worktree` again

### Discarding Changes

If you decide not to keep isolated changes:

```
/sessions
```

This opens the session manager where you can:

- View all isolated sessions
- Inspect changes before deciding
- Discard sessions you don't want

Discarding removes the worktree and all uncommitted changes—**no changes are applied to the main project**.

### When to Use Isolated Sessions

| Use Case | Recommended Mode |
|----------|------------------|
| Quick bug fix | Normal |
| Experimental feature | Isolated |
| Multiple agents working simultaneously | Isolated |
| Refactoring with uncertain outcomes | Isolated |
| Production hotfix | Normal |
| Code review/analysis | Normal |

### Technical Details

- Worktrees are created in `.fspec/worktrees/<session-id>/`
- Each worktree shares the same git history as the main repository
- Sessions are tracked in `~/.fspec/git-sessions/`
- Orphaned worktrees (from crashed sessions) are cleaned up automatically

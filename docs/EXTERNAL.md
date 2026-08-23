# Using fspec with External Agents

fspec works as tooling for Claude Code, Cursor, Codex, or any AI agent that can run shell commands. Instead of replacing your agent, fspec teaches it a disciplined workflow: the agent drives fspec's **CLI commands** to manage work units, write Gherkin specs, and follow the ACDD pipeline (specify → test → implement → validate).

## Setup

From your project root, run:

```bash
fspec init
```

On a terminal this opens an interactive selector (agents already detected in your project are pre-marked). For headless/scripted use, pass the agent explicitly:

```bash
fspec init --agent=claude
fspec init --agent=cursor --agent=claude   # multiple agents
```

### What `init` installs

For each selected agent, `init` writes two things:

1. **An agent documentation file** under `spec/` — e.g. `spec/CLAUDE.md` for Claude Code, `spec/AGENTS.md` for Codex. This is the full fspec workflow guide (work unit management, ACDD phases, Gherkin conventions, CLI command reference) tailored to that agent.
2. **A slash command** that bootstraps the agent's context:
   - Claude Code → `.claude/commands/fspec.md`
   - Codex / Codex CLI → `~/.codex/prompts/fspec.md` (installed once, per user)
   - Other agents → their own command directory (e.g. `.cursor/commands/fspec.md`)

It also records the agent id in `spec/fspec-config.json` (existing keys are preserved).

The slash command is the activation point. When the agent runs it, it:

1. runs `fspec --sync-version` to confirm the binary is in sync,
2. runs **`fspec bootstrap`** and loads its complete output into context.

`fspec bootstrap` emits the entire ACDD workflow documentation — the same content as the `spec/<AGENT>.md` file, with your project's configured test and quality-check commands substituted in (from `spec/fspec-config.json`). Once the agent has run bootstrap, it knows every fspec CLI command and the full workflow, and manages production from the shell.

## Example: Claude Code

```bash
cd /path/to/your/project
fspec init --agent=claude
```

This installs `spec/CLAUDE.md` and `.claude/commands/fspec.md`. Then, inside Claude Code:

```
/fspec
```

Claude runs `fspec bootstrap`, loads the workflow into context, and is ready. From there you just talk to it:

```
"Create a story for user authentication"
"Show me the board"
"Pick the next job from the backlog and start on it"
```

Claude drives the CLI itself — `fspec list-work-units`, `fspec create-story`, `fspec update-work-unit-status`, `fspec generate-scenarios`, and so on.

## Example: Codex

```bash
cd /path/to/your/project
fspec init --agent=codex
```

This installs `spec/AGENTS.md` and `~/.codex/prompts/fspec.md`. Then, inside Codex:

```
/prompts:fspec
```

Codex runs `fspec bootstrap` and loads the workflow. The same commands as above then work:

```
"Create a bug for the login failure on mobile"
"Move AUTH-001 to specifying and write the scenarios"
```

## Other agents

`fspec init` supports 19 agents: Claude Code, Cursor, Cline, Aider, Windsurf, GitHub Copilot, Gemini CLI, Qwen Code, Kilo Code, Roo Code, CodeBuddy, Amazon Q, Auggie, OpenCode, Codex, Factory Droid, Crush, Codex CLI, and Antigravity. Run `fspec init` without flags on a terminal to pick from the list.

Every agent follows the same pattern: `init` installs its doc file + slash command, the agent runs the slash command (which triggers `fspec bootstrap`), and then it uses the fspec CLI for all project management and specification work.

## Configuring test commands

The installed docs and bootstrap output contain `<test-command>` and `<quality-check-commands>` placeholders. Set them in `spec/fspec-config.json` and the agent will use your project's real commands:

```json
{
  "tools": {
    "test": { "command": "cargo test" },
    "qualityCheck": { "commands": ["cargo clippy", "cargo fmt --check"] }
  }
}
```

You can also set these interactively with `fspec configure-tools`. Re-run `fspec init` after changing the config to refresh the installed docs.

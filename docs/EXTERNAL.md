# Using fspec with External Agents

fspec works as tooling for Claude Code, Cursor, Codex, or any AI agent that can run shell commands.

## Setup

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

# Installation Guide

**Important:** fspec is designed to be used by AI agents, not run directly by users. You give high-level commands to your AI agent (like `/fspec` or `/rspec` in Claude Code), and the agent uses fspec to manage the workflow.

## Local Development

```bash
cd ~/projects/fspec
npm install
npm run build
npm run install:local
```

This will:
1. Install dependencies (including @cucumber/gherkin-parser)
2. Build the TypeScript code
3. Link the `fspec` command globally

## AI Agent Integration

fspec provides command files that enable AI-driven ACDD workflows:

```bash
# Initialize fspec in your project
fspec init
```

This creates command files in `.claude/commands/`:
- `fspec.md` - Forward ACDD command for building new features
- `rspec.md` - Reverse ACDD command for existing codebases
- `../spec/CLAUDE.md` - Specification management guidelines

**Using with AI Agents:**
- **Claude Code:** Use `/fspec` and `/rspec` slash commands directly
- **Other AI agents:** Map the generated `fspec.md` and `rspec.md` files to your agent's command system. Rename `spec/CLAUDE.md` to `spec/AGENTS.md` for clarity.

## Requirements

- Node.js >= 18.0.0

## Uninstall

```bash
npm unlink -g fspec
```

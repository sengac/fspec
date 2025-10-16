# Installation Guide

**Important:** fspec is designed to be used by AI agents, not run directly by users. You give high-level commands to your AI agent (like `/fspec` or `/rspec` in Claude Code), and the agent uses fspec to manage the workflow.

## Installation

### Via npm (Recommended)

```bash
npm install -g @sengac/fspec
```

This installs the latest stable version globally.

### From Source (For Development)

```bash
git clone https://github.com/sengac/fspec.git
cd fspec
npm install
npm run build
npm run install:local
```

This will:
1. Clone the repository
2. Install dependencies (including @cucumber/gherkin-parser)
3. Build the TypeScript code
4. Link the `fspec` command globally for development

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

**If installed via npm:**
```bash
npm uninstall -g @sengac/fspec
```

**If installed from source:**
```bash
npm unlink -g @sengac/fspec
```

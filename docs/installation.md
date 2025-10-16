# Installation Guide

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

## Claude Code Integration

fspec provides slash commands for Claude Code to enable AI-driven ACDD workflows:

```bash
# Initialize fspec in your project (installs /fspec and /rspec commands)
fspec init
```

This creates:
- `.claude/commands/fspec.md` - Main fspec slash command for forward ACDD
- `.claude/commands/rspec.md` - Reverse ACDD command for existing codebases
- `spec/CLAUDE.md` - Specification management guidelines

**Available Claude Code Commands:**
- `/fspec` - Main command for managing specifications and work units in forward ACDD
- `/rspec` - Reverse engineer existing codebases to create specifications

## Requirements

- Node.js >= 18.0.0

## Uninstall

```bash
npm unlink -g fspec
```

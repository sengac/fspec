<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**Stop fixing AI chaos. Start shipping quality.**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm](https://img.shields.io/npm/v/@sengac/fspec)](https://www.npmjs.com/package/@sengac/fspec)

---

## What is fspec?

**fspec makes AI agents follow discipline.** It's the missing workflow layer that turns AI coding chaos into structured, testable, traceable software.

Think **Git for specifications** + **Kanban for AI agents** + **TDD enforcement** all in one.

---

## Quick Start

1. **Install fspec globally**
   ```bash
   npm install -g @sengac/fspec
   ```

2. **Go to your project directory**
   ```bash
   cd /path/to/your/project
   ```

3. **Initialize fspec**
   ```bash
   fspec init
   ```

4. **Run your AI agent** (e.g., Claude Code, Codex)

5. **Bootstrap fspec context**
   - Use a bootstrapping command: `/fspec` (Claude Code) or `/prompts:fspec` (Codex)
   - Or tell your agent: "Run fspec bootstrap"

6. **Talk naturally with fspec**
   ```
   "I want to create a bug to fix this issue"
   "Create a checkpoint for this work"
   "Show me the kanban board"
   "Generate scenarios from the example map"
   ```

7. **Keep context fresh**
   - Over time, your agent's context gets cluttered
   - Clear your agent's context and bootstrap fspec again
   - Repeat as needed to maintain clean workflow

---

## Bug Reporting & Support

Found a bug? Just tell your AI agent:

```
"Report a bug to GitHub"
```

Your agent knows how to use `fspec report-bug-to-github` to automatically gather context and create the issue.

**Manual reporting:** [Create an issue](https://github.com/sengac/fspec/issues/new)

---

## Documentation

- 📘 **[Getting Started](./docs/getting-started.md)** - 5-minute quickstart
- 📖 **[User Guide](./docs/user-guide.md)** - Comprehensive usage
- 🎯 **[ACDD Workflow](./docs/acdd-workflow.md)** - Understanding the process
- 🤝 **[Example Mapping](./docs/example-mapping.md)** - Discovery techniques
- 📊 **[Work Units](./docs/work-units.md)** - Project management
- 🔗 **[Coverage Tracking](./docs/coverage-tracking.md)** - Traceability
- 🔄 **[Reverse ACDD](./docs/reverse-acdd.md)** - Existing codebases
- 💾 **[Git Checkpoints](./docs/checkpoints.md)** - Safe experimentation
- ⚡ **[Virtual Hooks](./docs/virtual-hooks.md)** - Quality gates
- 🏷️ **[Tags](./docs/tags.md)** - Organization system
- 🔧 **[CLI Reference](./docs/cli-reference.md)** - Command cheatsheet

**Pro tip:** All commands have comprehensive `--help` output:
```bash
fspec <command> --help
fspec help specs      # Gherkin commands
fspec help work       # Kanban commands
fspec help discovery  # Example mapping commands
```

---

**[Visit fspec.dev](https://fspec.dev)** | **[GitHub](https://github.com/sengac/fspec)** | **[npm](https://www.npmjs.com/package/@sengac/fspec)**

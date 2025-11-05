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

7. **Launch the interactive kanban**
   ```bash
   fspec
   ```
   Watch live changes, view work unit details, manage checkpoints, and navigate your kanban board with an intuitive TUI.

   ![Interactive Kanban](interactive-kanban.png)

8. **Keep context fresh**
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

## Checkpoints

Experiment fearlessly with built-in Git checkpoints. Tell your agent to **"create a checkpoint"** to save your current state, then view and restore checkpoints from the interactive kanban by pressing the **C** key.

![Checkpoint View](checkpoints.png)

---

## Attachment Viewer

Attach markdown documents with mermaid diagrams to work units for rich documentation **before** starting Example Mapping. Perfect for researching complex topics and creating visual designs.

**Example workflow:**
```
"Create a story about adding Event Storming. I want you to research this
topic on the web and create a markdown document with mermaid diagrams
and attach it to this story using fspec."
```

View attachments in the interactive kanban by pressing the **A** key, with full markdown and mermaid diagram rendering.

![Attachment Viewer - Document](attachment-viewer-1.png)

![Attachment Viewer - Diagram](attachment-viewer-2.png)

---

## Documentation

- 📘 **[Getting Started](https://fspec.dev/getting-started/quickstart/)** - 5-minute quickstart
- 📖 **[User Guide](https://fspec.dev/getting-started/introduction/)** - Comprehensive usage
- 🎯 **[ACDD Workflow](https://fspec.dev/concepts/acdd/)** - Understanding the process
- 🤝 **[Example Mapping](https://fspec.dev/concepts/example-mapping/)** - Discovery techniques
- 📊 **[Work Units](https://fspec.dev/concepts/kanban/)** - Project management
- 🔗 **[Coverage Tracking](https://fspec.dev/docs/coverage-tracking/)** - Traceability
- 🔄 **[Reverse ACDD](https://fspec.dev/docs/reverse-acdd/)** - Existing codebases
- 💾 **[Git Checkpoints](https://fspec.dev/docs/checkpoints/)** - Safe experimentation
- ⚡ **[Virtual Hooks](https://fspec.dev/docs/virtual-hooks/)** - Quality gates
- 🏷️ **[Tags](https://fspec.dev/commands/specs/)** - Organization system
- 🔧 **[CLI Reference](https://fspec.dev/reference/cli/)** - Command cheatsheet

**Pro tip:** All commands have comprehensive `--help` output:
```bash
fspec <command> --help
fspec help specs      # Gherkin commands
fspec help work       # Kanban commands
fspec help discovery  # Example mapping commands
```

---

**[Visit fspec.dev](https://fspec.dev)** | **[GitHub](https://github.com/sengac/fspec)** | **[npm](https://www.npmjs.com/package/@sengac/fspec)**

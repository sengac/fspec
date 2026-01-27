<picture>
  <source media="(prefers-color-scheme: dark)" srcset="fspec-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="fspec-logo-light.svg">
  <img alt="fspec" src="fspec-logo-light.svg" width="248">
</picture>

**The complete multi-agent AI coding platform with integrated spec-driven development.**

[![Website](https://img.shields.io/badge/Website-fspec.dev-blue)](https://fspec.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm](https://img.shields.io/npm/v/@sengac/fspec)](https://www.npmjs.com/package/@sengac/fspec)

---

## Why

AI agents lack the infrastructure that professional developers take for granted. No way to easily force AI to follow your acceptance criteria or ask questions about things it doesn't understand. AI confabulates without quality examples and doesn't ask when it needs to know what it doesn't know. No TDD guardrails. No or poorly implemented checkpoint systems for safe experimentation. No Kanban boards for tracking workflow state. No specification management systems with mermaid diagram viewers and markdown documentation. No coverage tracking to link code back to business rules. AI agents are coding in the dark, and you're left babysitting instead of building.

## What

fspec is a **comprehensive multi-agent AI coding platform** that unifies development workflows through disciplined spec-driven development. It combines:

- **🤖 Integrated AI Agent**: Full Rust-based coding agent built directly into the terminal interface
- **📋 Interactive Kanban**: Live project boards with real-time updates and dependency tracking  
- **🎯 ACDD Workflow**: Acceptance Criteria Driven Development enforcing test-first discipline
- **📝 Gherkin Management**: Complete BDD specification system with 430+ feature coverage
- **🔄 Git Checkpoints**: Automatic and manual checkpointing with conflict-aware restoration
- **📊 Coverage Tracking**: Full traceability from requirements to code implementation
- **🎨 Rich Terminal UI**: Split-pane conversations, file diffs, attachment viewers, diagram rendering
- **🔍 AST Code Analysis**: Deep codebase understanding across 16+ programming languages
- **🌐 Web Integration**: Chrome automation, PDF processing, research tools
- **⚡ Background Sessions**: Multiple concurrent AI conversations with session management

## How

fspec transforms the entire development lifecycle into a unified, AI-native experience where specifications drive development and AI agents work within professional guardrails.

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

4. **Launch the interactive platform**

   ```bash
   fspec
   ```

   This opens the **full interactive experience** with:
   - Live Kanban board for work unit management
   - Integrated AI agent conversations
   - File diff viewer and attachment system  
   - Checkpoint management interface
   - Real-time spec validation

   ![Interactive Kanban](interactive-kanban.png)

5. **Start a conversation with your AI agent**
   - Press **Enter** on any work unit to start an AI conversation
   - The agent is automatically contextualized with the work unit details
   - Use **Shift+←/→** to switch between multiple concurrent sessions
   - Press **ESC** to detach sessions and run them in the background

6. **Alternative: Use with external AI agents** (e.g., Claude Code, Cursor, Codex)
   - Bootstrap fspec context with `/fspec` or tell your agent: "Run fspec bootstrap"
   - Talk naturally: *"Create a story for user authentication"* or *"Show me the kanban board"*

---

## Core Platform Features

### 🤖 **Integrated AI Agent**

Built-in Rust-powered AI agent with advanced capabilities:
- **Multiple Providers**: OpenAI, Anthropic, Groq, Ollama, and 15+ more
- **AST Code Search**: Understanding code structure across languages
- **Web Research**: Chrome automation for gathering requirements
- **PDF Processing**: Extract and analyze documentation
- **Tool Integration**: File operations, Git management, test execution
- **Session Management**: Background execution with context preservation

### 📋 **Interactive Project Management**

Professional Kanban workflow with AI integration:
- **Live Board Updates**: Real-time synchronization across all interfaces
- **Work Unit Types**: Stories, tasks, bugs with full lifecycle tracking
- **Dependency Management**: Visualize and manage work dependencies
- **Epic Organization**: Group related work with progress tracking
- **Estimation & Metrics**: Story points, velocity, accuracy tracking

### 🎯 **Spec-Driven Development (ACDD)**

Disciplined workflow ensuring quality:
- **Acceptance Criteria Driven Development**: Test-first enforcement
- **Gherkin Specifications**: Full BDD with Given/When/Then scenarios
- **Example Mapping**: Collaborative discovery with rules/examples/questions
- **Event Storming**: Domain modeling for complex business contexts
- **Coverage Tracking**: Link every line of code to business requirements

### 🔄 **Git Checkpoint System**

Experiment fearlessly with professional backup:
- **Auto Checkpoints**: Created before every workflow transition
- **Manual Checkpoints**: Save experimental states
- **Conflict Resolution**: Smart restoration with merge handling
- **Cleanup Management**: Automatic pruning of old checkpoints

### 🎨 **Rich Terminal Interface**

Professional development environment in your terminal:
- **Split Sessions**: Parent/watcher conversation views
- **File Diff Viewer**: Visual code changes with syntax highlighting
- **Attachment System**: Markdown documents with Mermaid diagram rendering
- **Virtual Scrolling**: Handle large conversations efficiently
- **Keyboard Navigation**: Vim-like keybindings for power users

### 🔍 **Enterprise Analytics**

Deep insights into development workflow:
- **Bottleneck Analysis**: Identify workflow constraints
- **Estimation Accuracy**: Track and improve story point accuracy  
- **Coverage Reports**: Requirement-to-code traceability
- **Quality Metrics**: Test coverage, specification alignment
- **Dependency Statistics**: Understand work interdependencies

---

## Session Management

fspec provides unprecedented flexibility in AI conversation management:

**🔗 Session-to-Story Linking:**
- AI conversations automatically link to work units
- Work units with conversations show a 🟢 indicator
- Resume exact conversations when returning to work units
- Use `/detach` to start fresh or `/resume` to switch contexts

**⚡ Multiple Concurrent Sessions:**
- Run multiple AI agents simultaneously in background
- **Shift+←/→** instantly switch between active sessions
- Detached sessions continue executing while you work elsewhere
- Background task completion with buffered output

**📱 Background Execution:**
- Press **ESC → Detach** to leave sessions running in background
- **Space+ESC** returns to board while keeping sessions attached
- View all background sessions with `/resume`
- Like tmux/screen but for AI agent conversations

---

## Advanced Capabilities

### 🌐 **Web Integration**
- Automated web research for requirement gathering
- Chrome DevTools integration for testing
- Screenshot capture for documentation
- Link validation and content extraction

### 📊 **Coverage Analysis**
- Link Gherkin scenarios to test files to implementation
- Requirement traceability matrices
- Orphaned code detection
- Business rule compliance tracking

### 🔧 **Hook System**
- Pre/post workflow hooks for automation
- Custom validation and quality gates
- Integration with CI/CD pipelines
- Conditional execution based on tags/estimates

### 🎨 **Attachment Viewer**
Comprehensive documentation system:
- **Markdown Rendering**: Full GitHub-flavored markdown support
- **Mermaid Diagrams**: Interactive diagram viewing
- **Research Integration**: AI agents can create and attach documentation
- **Version History**: Track document changes over time

![Attachment Viewer - Dialog](attachment1.png)
![Attachment Viewer - Diagram](attachment2.png)

---

## Checkpoint Management

Professional backup and experimentation system:

![Checkpoint View](checkpoints.png)

- **Automatic Checkpointing**: Before every workflow state transition
- **Manual Snapshots**: Save experimental work states  
- **Branching Strategies**: Multiple checkpoint branches per work unit
- **Conflict Resolution**: Smart merging when restoring checkpoints
- **Cleanup Automation**: Configurable retention policies

---

## 🦴 DOGFOODING 🍖

**We practice what we preach.** fspec was built entirely using fspec itself, resulting in **432 comprehensive feature files** with complete Gherkin specifications, full test coverage, and end-to-end traceability.

**Traditional approach:** A dedicated QA and business analyst team would need **9-12 months** to produce this level of documentation and specification coverage.

**fspec approach:** We achieved this in **weeks** using AI agents working within ACDD discipline, with the platform enforcing quality and traceability at every step.

**See for yourself:** Browse the [complete feature file collection](https://github.com/sengac/fspec/tree/main/spec/features) and witness what AI-driven, spec-first development looks like at scale.

---

## Bug Reporting & Support

**With integrated AI agent:**
Just tell your agent: *"Report a bug to GitHub"* - it knows how to use `fspec report-bug-to-github` to gather context and create issues.

**With external AI agents:**
Tell your agent: *"Report a bug to GitHub using fspec"*

**Manual reporting:** [Create an issue](https://github.com/sengac/fspec/issues/new)

---

## Professional Services

**Your codebase is costing you millions.** Untested. Undocumented. Impossible to maintain. SENGAC specializes in transforming legacy systems into modern, AI-tested, fully-specified platforms using fspec.

We don't just migrate—we rebuild with:
- ✅ Complete test coverage
- ✅ Living documentation  
- ✅ End-to-end traceability
- ✅ AI agent integration
- ✅ Professional workflow discipline

**Stop throwing money at technical debt.** [Contact SENGAC](https://sengac.com) to modernize your development infrastructure.

---

**[Visit fspec.dev](https://fspec.dev)** | **[GitHub](https://github.com/sengac/fspec)** | **[npm](https://www.npmjs.com/package/@sengac/fspec)**
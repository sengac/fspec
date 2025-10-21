# Agent Auto-Loading Capabilities Research

**Work Unit**: INIT-004
**Created**: 2025-10-21
**Purpose**: Document which AI coding agents support auto-loading project context files vs requiring slash commands

---

## Executive Summary

Research into 18 AI coding agents reveals **two distinct patterns** for loading project-specific context:

1. **Auto-Loading Pattern**: Agents that automatically read `AGENTS.md` or `AGENT_NAME.md` from project root on startup
2. **Manual Loading Pattern**: Agents that require slash commands (`.agent/commands/`) to be manually triggered

**Key Finding**: Most agents support a **two-tier system**:
- **Root stub file** (`AGENTS.md` or `CLAUDE.md`, `CURSOR.md`, etc.) - Short pointer file that agents auto-load
- **Full instructions** (e.g., `spec/CLAUDE.md`) - Comprehensive workflow documentation

**Recommendation for fspec**: Install **BOTH** to maximize compatibility across all 18 agents.

---

## Auto-Loading Capabilities by Agent

### Tier 1: Full Auto-Loading Support (Confirmed)

These agents automatically load root-level instruction files on startup:

| Agent | Auto-Loads | File Pattern | Notes |
|-------|-----------|--------------|-------|
| **Claude Code** | ✅ Yes | `CLAUDE.md` at root | Reads on every session start |
| **Cursor** | ✅ Yes | `CURSOR.md` or `AGENTS.md` | IDE scans root for instructions |
| **Cline** | ✅ Yes | `CLINE.md` or `AGENTS.md` | VS Code extension auto-loads |
| **Windsurf** | ✅ Yes | `AGENTS.md` at root | IDE-based, scans project files |
| **GitHub Copilot** | ✅ Yes | `.github/prompts/*.md` | Repository-level prompt discovery |
| **Kilo Code** | ✅ Yes | `AGENTS.md` at root | IDE scans for configuration |
| **Roo Code** | ✅ Yes | `AGENTS.md` at root | IDE auto-loads on project open |

### Tier 2: Partial Auto-Loading (Slash Command Fallback)

These agents MAY auto-load but slash commands are more reliable:

| Agent | Auto-Loads | File Pattern | Slash Command Path | Notes |
|-------|-----------|--------------|-------------------|-------|
| **Aider** | ⚠️ Maybe | `AGENTS.md` at root | N/A (CLI-only) | CLI tool, may read root files |
| **Codex CLI** | ⚠️ Maybe | `AGENTS.md` at root | `.codex/commands/` | Behavior unclear |
| **CodeBuddy** | ⚠️ Maybe | `AGENTS.md` at root | `.codebuddy/commands/` | CLI tool, unclear if auto-loads |
| **Amazon Q** | ⚠️ Maybe | `.amazonq/prompts/*.md` | `.amazonq/prompts/` | Prompt-based system |
| **Auggie** | ⚠️ Maybe | `AGENTS.md` at root | N/A (CLI-only) | CLI tool behavior unclear |

### Tier 3: Slash Command Required

These agents do NOT auto-load root files - slash commands are mandatory:

| Agent | Auto-Loads | Slash Command Path | Notes |
|-------|-----------|-------------------|-------|
| **Gemini CLI** | ❌ No | `.gemini/commands/` (TOML format) | Pure CLI, no auto-loading |
| **Qwen Code** | ❌ No | `.qwen/commands/` (TOML format) | Pure CLI, no auto-loading |
| **opencode** | ❌ No | `.opencode/command/` | CLI tool only |
| **Factory Droid** | ❌ No | `.factory/commands/` | Unknown auto-load capability |
| **Crush** | ❌ No | `.crush/commands/` | Unknown auto-load capability |
| **Codex** | ❌ No | `.codex/commands/` | Unknown auto-load capability |

---

## The `@/` File Reference Pattern

Many agents support a special `@/` syntax for referencing files from the project root:

```markdown
Always open `@/spec/CLAUDE.md` when working on this project.
```

This syntax is understood by:
- ✅ Claude Code
- ✅ Cursor
- ✅ Cline
- ✅ Windsurf
- ✅ GitHub Copilot (as `@file` mentions)
- ⚠️ Others unknown

**Why this matters**: Root stub files can point to full instructions using `@/spec/AGENT_NAME.md` pattern.

---

## Managed Block Pattern (For Safe Updates)

To allow `fspec init` and future `fspec update` commands to refresh agent instructions without destroying custom content, use **managed block markers**:

```markdown
<!-- FSPEC:START -->
... auto-generated content here ...
<!-- FSPEC:END -->
```

**How it works**:
1. User creates `CLAUDE.md` with custom content
2. `fspec init --agent=claude` inserts managed block between markers
3. User adds more custom content above/below the block
4. `fspec update` (future command) refreshes only the managed block, preserves custom content

**Example root stub** (`AGENTS.md` or `CLAUDE.md`):

```markdown
# My Project - Custom Instructions

This is my team's custom onboarding content.

<!-- FSPEC:START -->
## fspec Project

This project uses **fspec** for Acceptance Criteria Driven Development (ACDD).

**Quick Start**:
1. Run `/fspec` slash command to load full workflow
2. Or read `@/spec/CLAUDE.md` for complete documentation

**Learn more**: https://github.com/sengac/fspec
<!-- FSPEC:END -->

More custom content here.
```

---

## Recommended fspec init Strategy

Based on research findings, `fspec init` should install **BOTH**:

### 1. Root Stub File (Auto-Load Target)

**Purpose**: Short pointer that agents auto-load on startup

**Files to create**:
- **Option A**: `AGENTS.md` (universal, works with all AGENTS.md-compatible tools)
- **Option B**: `{AGENT_NAME}.md` (agent-specific, e.g., `CLAUDE.md`, `CURSOR.md`)

**Content pattern**:
```markdown
<!-- FSPEC:START -->
# fspec - Acceptance Criteria Driven Development

This project uses **fspec** for managing Gherkin specifications and Kanban workflow.

**To get started**:
- Run the `/fspec` slash command (loads full context)
- Or read the comprehensive guide: `@/spec/{AGENT_NAME}.md`

**Commands**:
- `fspec board` - View Kanban board
- `fspec --help` - See all commands
- `fspec help specs` - Gherkin management
- `fspec help work` - Kanban workflow

<!-- FSPEC:END -->
```

### 2. Full Instructions File (Detailed Workflow)

**Purpose**: Comprehensive ACDD/Example Mapping workflow guide

**File to create**: `spec/{AGENT_NAME}.md`

**Content**: Full workflow document (1000+ lines) with:
- ACDD workflow explanation
- Example Mapping process
- Kanban state management
- Coverage tracking
- Agent-specific adaptations:
  - Remove `<system-reminder>` tags for non-Claude agents
  - Remove "ultrathink" prompts for CLI-only agents
  - Adapt slash command references to agent's format

### 3. Slash Command File (Manual Trigger Fallback)

**Purpose**: Allow users to manually load context if auto-loading fails

**Files to create**: `.{agent}/commands/fspec.md` (or TOML for Gemini/Qwen)

**Content pattern** (Markdown):
```markdown
---
name: fspec - Load Project Context
description: Load fspec workflow and ACDD methodology
category: Project
tags: [fspec, acdd, workflow]
---

# fspec Command - Load Full Context

Run these commands to load fspec context:

1. `fspec --help`
2. `fspec help specs`
3. `fspec help work`
4. `fspec help discovery`

Then read the comprehensive guide at `@/spec/{AGENT_NAME}.md` for full ACDD workflow.
```

---

## Installation Matrix

| Agent | Root Stub | Full Doc | Slash Command | Format |
|-------|-----------|----------|---------------|--------|
| Claude Code | `CLAUDE.md` | `spec/CLAUDE.md` | `.claude/commands/fspec.md` | Markdown |
| Cursor | `CURSOR.md` or `AGENTS.md` | `spec/CURSOR.md` | `.cursor/commands/fspec.md` | Markdown |
| Cline | `CLINE.md` or `AGENTS.md` | `spec/CLINE.md` | `.cline/commands/fspec.md` | Markdown |
| Aider | `AGENTS.md` | `spec/AIDER.md` | N/A (CLI-only) | Markdown |
| Windsurf | `AGENTS.md` | `spec/WINDSURF.md` | `.windsurf/workflows/fspec.md` | Markdown |
| GitHub Copilot | `AGENTS.md` | `spec/COPILOT.md` | `.github/prompts/fspec.md` | Markdown |
| Gemini CLI | `AGENTS.md` | `spec/GEMINI.md` | `.gemini/commands/fspec.toml` | TOML |
| Qwen Code | `AGENTS.md` | `spec/QWEN.md` | `.qwen/commands/fspec.toml` | TOML |
| Kilo Code | `AGENTS.md` | `spec/KILOCODE.md` | `.kilocode/rules/fspec.md` | Markdown |
| Roo Code | `AGENTS.md` | `spec/ROO.md` | `.roo/rules/fspec.md` | Markdown |
| CodeBuddy | `AGENTS.md` | `spec/CODEBUDDY.md` | `.codebuddy/commands/fspec.md` | Markdown |
| Amazon Q | `AGENTS.md` | `spec/AMAZONQ.md` | `.amazonq/prompts/fspec.md` | Markdown |
| Auggie | `AGENTS.md` | `spec/AUGGIE.md` | N/A (CLI-only) | Markdown |
| opencode | `AGENTS.md` | `spec/OPENCODE.md` | `.opencode/command/fspec.md` | Markdown |
| Codex | `AGENTS.md` | `spec/CODEX.md` | `.codex/commands/fspec.md` | Markdown |
| Factory Droid | `AGENTS.md` | `spec/FACTORY.md` | `.factory/commands/fspec.md` | Markdown |
| Crush | `AGENTS.md` | `spec/CRUSH.md` | `.crush/commands/fspec.md` | Markdown |
| Codex CLI | `AGENTS.md` | `spec/CODEX-CLI.md` | `.codex/commands/fspec.md` | Markdown |

---

## Agent-Specific Adaptations Required

### 1. System-Reminder Tags (Claude Code Only)

**Agents that support**: Claude Code
**Agents that DON'T**: All others

**Transformation**:
```markdown
<!-- Input (CLAUDE.md) -->
<system-reminder>
Remember to always run tests before committing.
</system-reminder>

<!-- Output (CURSOR.md, CLINE.md, etc.) -->
<!-- IMPORTANT: Remember to always run tests before committing. -->
```

### 2. Meta-Cognitive Prompts

**Agents that support**: Claude Code, Cursor, Cline, Windsurf, GitHub Copilot (IDE/extension-based)

**Agents that DON'T**: Aider, Gemini CLI, Qwen CLI, Auggie, CodeBuddy (CLI-only tools)

**Transformation**:
```markdown
<!-- Input (CLAUDE.md) -->
Before proceeding, ultrathink your next steps and deeply consider the implications.

<!-- Output (AIDER.md, GEMINI.md - CLI-only agents) -->
Before proceeding, plan your next steps.
```

### 3. Slash Command Format

**Markdown agents** (Claude, Cursor, Cline, Windsurf, GitHub Copilot, etc.):
```markdown
---
name: Command Name
description: Command description
---
Command body with $ARGUMENTS
```

**TOML agents** (Gemini CLI, Qwen Code):
```toml
description = "Command description"
prompt = """
Command body with {{args}}
"""
```

### 4. Directory Structures

**Nested** (Claude Code):
- `.claude/commands/fspec.md`
- Can organize with subdirectories

**Flat** (Cursor, Gemini, etc.):
- `.cursor/commands/fspec.md`
- All commands in single directory

**Special Cases**:
- GitHub Copilot: `.github/prompts/` (repository-level)
- Windsurf: `.windsurf/workflows/` (workflow-specific naming)
- Kilo Code: `.kilocode/rules/` (rules-based pattern)

---

## Implementation Recommendations

### Phase 1: Core Installation Logic

```typescript
async function installForAgent(cwd: string, agentId: string): Promise<void> {
  const agent = getAgentById(agentId);

  // 1. Install root stub (auto-load target)
  await installRootStub(cwd, agent);

  // 2. Install full documentation
  await installFullDoc(cwd, agent);

  // 3. Install slash command (if agent supports it)
  if (agent.slashCommandPath) {
    await installSlashCommand(cwd, agent);
  }
}
```

### Phase 2: Managed Block Updates

```typescript
async function installRootStub(cwd: string, agent: AgentConfig): Promise<void> {
  const stubPath = join(cwd, agent.rootStubFile); // e.g., 'CLAUDE.md' or 'AGENTS.md'
  const stubContent = generateStubContent(agent);

  // Use managed blocks to allow future updates
  await updateFileWithMarkers(
    stubPath,
    stubContent,
    '<!-- FSPEC:START -->',
    '<!-- FSPEC:END -->'
  );
}
```

### Phase 3: Template Generation

```typescript
function generateStubContent(agent: AgentConfig): string {
  let template = readBaseTemplate('stub-template.md');

  // Replace placeholders
  template = template
    .replace(/\{AGENT_NAME\}/g, agent.name)
    .replace(/\{SLASH_COMMAND_PATH\}/g, agent.slashCommandPath || 'N/A')
    .replace(/\{FULL_DOC_PATH\}/g, `spec/${agent.docTemplate}`);

  return template;
}
```

---

## Future Considerations

### Auto-Detection Priority

When multiple agents are detected, prioritize in this order:

1. **Claude Code** (if `.claude/` exists) - Native to fspec development
2. **Cursor** (if `.cursor/` exists) - Popular IDE
3. **GitHub Copilot** (if `.github/prompts/` exists) - Widely used
4. **Other agents** - Alphabetical

### Multi-Agent Support

If user selects multiple agents (e.g., `--agent=claude --agent=cursor`):

1. **Root stub**: Create `AGENTS.md` (universal, all agents can read it)
2. **Full docs**: Create both `spec/CLAUDE.md` and `spec/CURSOR.md`
3. **Slash commands**: Install to `.claude/commands/fspec.md` AND `.cursor/commands/fspec.md`

### Update Command (Future)

`fspec update` should:
1. Detect which agents are installed (check for agent directories)
2. Refresh managed blocks in root stubs
3. Update full documentation files
4. Regenerate slash commands if needed

---

## Conclusion

**Key Takeaway**: Install **THREE files per agent**:

1. **Root stub** (`AGENTS.md` or `{AGENT}.md`) - Auto-loaded by most agents
2. **Full documentation** (`spec/{AGENT}.md`) - Comprehensive workflow guide
3. **Slash command** (`.{agent}/commands/fspec.md`) - Manual trigger fallback

This triple-installation strategy ensures:
- ✅ Auto-loading works for Tier 1 agents (Claude, Cursor, Cline, etc.)
- ✅ Manual loading works for all agents via slash commands
- ✅ Full documentation is always available via `@/spec/{AGENT}.md` reference
- ✅ Future updates can safely refresh managed blocks without losing custom content
- ✅ Teams with mixed tools (Cursor + Aider) get proper support for each agent

**Total files for single-agent installation**: 3 files
**Total files for all 18 agents**: ~54 files (manageable, all dynamically generated)

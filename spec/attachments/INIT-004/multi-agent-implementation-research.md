# Multi-Agent Support Implementation Research

## Executive Summary

This document provides technical research on implementing multi-agent support for fspec based on analysis of existing spec-driven development tools that have successfully implemented agent-agnostic architectures. The goal is to enable fspec to support any AI coding agent while maintaining a consistent user experience.

---

## Supported Agents Analysis

Based on research of existing implementations, here are the agents that should be supported with their directory conventions and characteristics:

### Tier 1: CLI-Based Agents (with slash command support)

| Agent | Directory Path | Format | CLI Tool | Slash Command Support | System-Reminder Support |
|-------|---------------|--------|----------|----------------------|------------------------|
| **Claude Code** | `.claude/commands/` | Markdown with YAML frontmatter | `claude` | ✅ Yes (namespaced: `/fspec/command`) | ✅ Yes (native `<system-reminder>` tags) |
| **Cursor** | `.cursor/commands/` | Markdown with YAML frontmatter | `cursor-agent` | ✅ Yes (flat: `/fspec-command`) | ❌ No |
| **Cline** | `.cline/commands/` | Markdown | N/A (VSCode extension) | ✅ Yes | ❌ No |
| **Auggie CLI** | `.augment/rules/` | Markdown | `auggie` | ✅ Yes | ❌ No |
| **CodeBuddy CLI** | `.codebuddy/commands/` | Markdown | `codebuddy` | ✅ Yes | ❌ No |
| **OpenCode** | `.opencode/command/` | Markdown | `opencode` | ✅ Yes | ❌ No |
| **Codex CLI** | `.codex/commands/` | Markdown | `codex` | ✅ Yes | ❌ No |
| **Amazon Q Developer CLI** | `.amazonq/prompts/` | Markdown | `q` | ⚠️  Partial (no custom args) | ❌ No |

### Tier 2: IDE-Based Agents

| Agent | Directory Path | Format | IDE Integration | Slash Command Support | System-Reminder Support |
|-------|---------------|--------|-----------------|----------------------|------------------------|
| **Windsurf** | `.windsurf/workflows/` | Markdown | Windsurf IDE | ✅ Yes | ❌ No |
| **Kilo Code** | `.kilocode/rules/` | Markdown | Kilo Code IDE | ✅ Yes | ❌ No |
| **Roo Code** | `.roo/rules/` | Markdown | Roo Code IDE | ✅ Yes | ❌ No |
| **GitHub Copilot** | `.github/prompts/` | Markdown | VS Code/JetBrains | ⚠️  Partial | ❌ No |
| **Factory Droid** | `.factory/rules/` | Markdown | Factory IDE | ✅ Yes | ❌ No |
| **Crush** | `.crush/commands/` | Markdown | Crush IDE | ✅ Yes | ❌ No |

### Tier 3: TOML-Based Agents

| Agent | Directory Path | Format | CLI Tool | Slash Command Support |
|-------|---------------|--------|----------|----------------------|
| **Gemini CLI** | `.gemini/commands/` | TOML | `gemini` | ✅ Yes (`{{args}}` placeholder) |
| **Qwen Code** | `.qwen/commands/` | TOML | `qwen` | ✅ Yes (`{{args}}` placeholder) |

### Tier 4: Fallback (AGENTS.md pattern)

| Agent | Directory Path | Format | Notes |
|-------|---------------|--------|-------|
| **Generic (AGENTS.md)** | `./` | Markdown | Works with Amp, VS Code, and any tool that reads AGENTS.md |

**Total Agent Support: 18 agents**

---

## Key Implementation Patterns

### 1. Agent Registry Architecture

Successful implementations use a **central agent registry** pattern that maps agent IDs to configuration:

```typescript
interface AgentConfig {
  name: string;              // Display name (e.g., "Claude Code")
  id: string;                // Unique ID (e.g., "claude")
  docTemplate: string;       // Documentation template filename (e.g., "CLAUDE.md")
  slashCommandPath: string;  // Directory for slash commands (e.g., ".claude/commands/")
  format: 'markdown' | 'toml'; // Command file format
  supportsSystemReminders: boolean;  // Whether agent supports <system-reminder> tags
  requiresCLI: boolean;      // Whether agent requires CLI tool installation
  cliTool?: string;          // CLI executable name (e.g., "claude", "cursor-agent")
  installUrl?: string;       // Installation documentation URL
  argumentPlaceholder: string; // Argument syntax (e.g., "$ARGUMENTS", "{{args}}")
}
```

**Key Design Principle**: Use the actual CLI executable name as the agent ID (e.g., `cursor-agent` not `cursor`) to avoid special-case mappings throughout the codebase.

### 2. Configurator Pattern (Strategy Pattern)

Implement agent-specific behavior using the **Strategy Pattern** with a base class and agent-specific subclasses:

```typescript
abstract class AgentConfigurator {
  abstract readonly agentId: string;
  abstract readonly isAvailable: boolean;

  // Get targets for this agent (e.g., slash command files, doc templates)
  abstract getTargets(): ConfigTarget[];

  // Generate all configuration files for this agent
  async generateAll(projectPath: string): Promise<string[]> {
    // Create slash commands, documentation, etc.
  }

  // Update existing configuration files (for fspec update command)
  async updateExisting(projectPath: string): Promise<string[]> {
    // Only update files that already exist
  }

  // Get file path for specific config (e.g., slash command)
  protected abstract getRelativePath(configId: string): string;

  // Get frontmatter for markdown files (YAML metadata)
  protected abstract getFrontmatter(configId: string): string | undefined;

  // Get command body (shared across agents)
  protected getBody(configId: string): string {
    return TemplateManager.getCommandBody(configId);
  }
}
```

**Agent-Specific Implementations**:
```typescript
class ClaudeCodeConfigurator extends AgentConfigurator {
  readonly agentId = 'claude';
  readonly isAvailable = true;

  protected getRelativePath(configId: string): string {
    return `.claude/commands/fspec/${configId}.md`;
  }

  protected getFrontmatter(configId: string): string {
    return `---
name: fspec: ${configId}
description: ${getDescription(configId)}
category: fspec
tags: [fspec, ${configId}]
---`;
  }
}

class CursorConfigurator extends AgentConfigurator {
  readonly agentId = 'cursor';

  protected getRelativePath(configId: string): string {
    return `.cursor/commands/fspec-${configId}.md`;  // Flat naming
  }

  protected getFrontmatter(configId: string): string {
    return `---
name: /fspec-${configId}
id: fspec-${configId}
category: fspec
---`;
  }
}
```

### 3. Marker-Based Update System

Use HTML comment markers to enable **idempotent updates** of generated files:

```markdown
---
name: fspec: validate
description: Validate Gherkin syntax and tags
category: fspec
---
<!-- FSPEC:START -->
# fspec validate

Run Gherkin syntax validation on all feature files.

## Usage
fspec validate [file]

<!-- FSPEC:END -->
```

**Update Logic**:
1. Check if file exists
2. If exists: Replace content between `<!-- FSPEC:START -->` and `<!-- FSPEC:END -->`
3. If not exists: Generate complete file with frontmatter + markers + body
4. **Never modify frontmatter during updates** - only regenerate on init

**Critical**: Markers MUST wrap only the body content, never the YAML frontmatter, to prevent parse errors.

### 4. Template Management System

Centralize command bodies in a **Template Manager** to avoid duplication:

```typescript
class TemplateManager {
  // Generate command body from CLAUDE.md or agent-specific templates
  static getCommandBody(commandId: string, agentId: string): string {
    // Strip system-reminder tags for non-Claude agents
    const rawBody = this.loadTemplate(commandId);
    if (agentId !== 'claude') {
      return this.stripSystemReminders(rawBody);
    }
    return rawBody;
  }

  // Strip <system-reminder> tags but preserve content
  static stripSystemReminders(content: string): string {
    return content
      .replace(/<system-reminder>\n/g, '')
      .replace(/\n<\/system-reminder>/g, '')
      .replace(/<system-reminder>/g, '')
      .replace(/<\/system-reminder>/g, '');
  }

  // Get agent-specific documentation template
  static getAgentDocTemplate(agentId: string): string {
    const template = this.loadDocTemplate(agentId);
    if (agentId !== 'claude') {
      return this.stripSystemReminders(template);
    }
    return template;
  }
}
```

### 5. Auto-Detection Strategy

Implement **auto-detection with explicit override** pattern:

```typescript
async function detectAgent(projectPath: string): Promise<string | null> {
  // Check for agent-specific directories
  const checks = [
    { dir: '.claude/commands', agent: 'claude' },
    { dir: '.cursor/commands', agent: 'cursor' },
    { dir: '.cline/commands', agent: 'cline' },
    { dir: '.windsurf/workflows', agent: 'windsurf' },
    // ... etc
  ];

  for (const check of checks) {
    if (await directoryExists(join(projectPath, check.dir))) {
      return check.agent;
    }
  }

  // Check environment variables
  if (process.env.AI_AGENT) {
    return process.env.AI_AGENT;
  }

  return null;  // No agent detected, prompt user
}

// Usage in init command
async function init(options: InitOptions) {
  let agentId = options.agent;

  if (!agentId) {
    // Try auto-detection
    agentId = await detectAgent(options.cwd);
    if (agentId) {
      console.log(`Detected: ${getAgentName(agentId)}`);
    } else {
      // No detection, use default or prompt
      agentId = 'claude';  // Default
    }
  }

  // User can override with --agent flag
  const configurator = AgentRegistry.get(agentId);
  await configurator.generateAll(options.cwd);
}
```

### 6. Multi-Agent Support

Support installing for **multiple agents simultaneously**:

```bash
# Install for multiple agents
fspec init --agent=claude --agent=cursor --agent=windsurf
```

**Implementation**:
```typescript
async function initMultipleAgents(agentIds: string[], projectPath: string) {
  const installed: string[] = [];

  for (const agentId of agentIds) {
    const configurator = AgentRegistry.get(agentId);
    if (!configurator) {
      console.warn(`Unknown agent: ${agentId}`);
      continue;
    }

    const files = await configurator.generateAll(projectPath);
    installed.push(agentId);

    // Generate agent-specific documentation
    const docTemplate = TemplateManager.getAgentDocTemplate(agentId);
    const docPath = `spec/${agentId.toUpperCase()}.md`;
    await writeFile(join(projectPath, docPath), docTemplate);
  }

  console.log(`Configured ${installed.length} agents: ${installed.join(', ')}`);
}
```

---

## Command Naming Conventions

Different agents have different slash command naming patterns:

### Claude Code (Namespaced)
- **Pattern**: `/fspec/command`
- **Examples**: `/fspec/validate`, `/fspec/format`, `/fspec/init`
- **Path**: `.claude/commands/fspec/validate.md`
- **Supports**: Categories, tags, nested directories

### Cursor (Flat with Prefix)
- **Pattern**: `/fspec-command`
- **Examples**: `/fspec-validate`, `/fspec-format`, `/fspec-init`
- **Path**: `.cursor/commands/fspec-validate.md`
- **Supports**: Categories, IDs, flat structure only

### Other Agents
- **Pattern**: `/fspec-command` (flat, prefixed)
- Most agents follow Cursor's flat naming pattern
- Some use different directory structures but similar naming

---

## File Format Variations

### Markdown Format (Most Agents)

**With Frontmatter** (Claude, Cursor, Cline, Windsurf, Kilo, Roo, etc.):
```markdown
---
name: fspec: validate
description: Validate Gherkin syntax
category: fspec
tags: [fspec, validation]
---
<!-- FSPEC:START -->
# fspec validate

Command documentation here...
<!-- FSPEC:END -->
```

**Without Frontmatter** (OpenCode, some agents):
```markdown
<!-- FSPEC:START -->
# fspec validate

Command documentation here...
<!-- FSPEC:END -->
```

### TOML Format (Gemini, Qwen)

```toml
description = "Validate Gherkin syntax and tags"

prompt = """
# FSPEC:START
# fspec validate

Run Gherkin syntax validation on all feature files.

## Usage
fspec validate [file]

Arguments are available via {{args}} placeholder.
# FSPEC:END
"""
```

**Key Difference**: TOML agents use `{{args}}` instead of `$ARGUMENTS` for argument placeholders.

---

## System-Reminder Handling

**Problem**: `<system-reminder>` tags are Claude Code-specific and don't work with other agents.

**Solutions Observed**:

### Option 1: Strip Tags, Preserve Content
```typescript
function stripSystemReminders(content: string): string {
  // Remove tags but keep the instructional content
  return content
    .replace(/<system-reminder>\n/g, '')
    .replace(/\n<\/system-reminder>/g, '');
}
```

**Result**: Instructional content becomes plain text, visible to all agents.

### Option 2: Convert to Agent-Specific Patterns

**Claude Code**:
```markdown
<system-reminder>
CRITICAL: Use Example Mapping before writing scenarios.
</system-reminder>
```

**Other Agents** (Plain Markdown):
```markdown
**IMPORTANT**: Use Example Mapping before writing scenarios.
```

**Cursor/Windsurf** (Comments):
```markdown
<!-- IMPORTANT: Use Example Mapping before writing scenarios. -->
```

### Option 3: Agent-Specific Templates

Maintain separate template variations:
- `templates/claude/CLAUDE.md` - With system-reminders
- `templates/cursor/CURSOR.md` - With bold/italic emphasis
- `templates/cline/CLINE.md` - With comment-based guidance

---

## Directory Structure Best Practices

### Agent-Specific Directories
```
project/
├── .claude/
│   └── commands/
│       └── fspec/
│           ├── validate.md
│           ├── format.md
│           └── init.md
├── .cursor/
│   └── commands/
│       ├── fspec-validate.md
│       ├── fspec-format.md
│       └── fspec-init.md
├── .cline/
│   └── commands/
│       ├── fspec-validate.md
│       └── ...
├── .windsurf/
│   └── workflows/
│       ├── fspec-validate.md
│       └── ...
├── spec/
│   ├── CLAUDE.md          # Agent-specific docs
│   ├── CURSOR.md
│   ├── CLINE.md
│   ├── WINDSURF.md
│   ├── features/
│   ├── work-units.json
│   └── foundation.json
```

### Shared Templates Directory (fspec internal)
```
fspec/
├── templates/
│   ├── agents/
│   │   ├── claude/
│   │   │   ├── CLAUDE.md
│   │   │   └── commands/
│   │   │       ├── validate.md
│   │   │       └── format.md
│   │   ├── cursor/
│   │   │   ├── CURSOR.md
│   │   │   └── commands/
│   │   ├── cline/
│   │   └── ...
│   └── shared/
│       └── command-bodies/  # Shared command logic
```

---

## Argument Placeholder Patterns

Different agents use different syntax for passing arguments to commands:

| Agent Type | Placeholder | Example |
|-----------|-------------|---------|
| Markdown-based (Claude, Cursor, Cline, Windsurf, etc.) | `$ARGUMENTS` | `fspec validate $ARGUMENTS` |
| TOML-based (Gemini, Qwen) | `{{args}}` | `fspec validate {{args}}` |
| Script placeholder (all) | `{SCRIPT}` | Replaced with actual script path |
| Agent placeholder (all) | `__AGENT__` | Replaced with agent name |

**Template Example with Placeholders**:
```markdown
# fspec validate

Validate Gherkin syntax for feature files.

## Usage
fspec validate $ARGUMENTS

This command is running in __AGENT__.
```

**After substitution** (for Cursor):
```markdown
# fspec validate

Validate Gherkin syntax for feature files.

## Usage
fspec validate $ARGUMENTS

This command is running in Cursor.
```

---

## Implementation Roadmap for fspec

### Phase 1: Core Infrastructure
1. **Create Agent Registry**
   - Define `AgentConfig` interface
   - Implement agent registry at `src/agents/registry.ts`
   - Register all 18 agents with their configurations

2. **Implement Configurator Pattern**
   - Create base `AgentConfigurator` class
   - Implement agent-specific configurators for each agent
   - Register configurators in `AgentConfiguratorRegistry`

3. **Template Management System**
   - Create `TemplateManager` class
   - Implement system-reminder stripping logic
   - Store templates in `templates/agents/{agent-id}/`

### Phase 2: Init Command Refactoring
1. **Add --agent Flag**
   - Modify `init` command to accept `--agent` flag (can be specified multiple times)
   - Implement auto-detection logic
   - Add validation against supported agents list

2. **Agent-Specific File Generation**
   - Generate slash commands using configurators
   - Copy agent-specific documentation templates
   - Create appropriate directory structures

3. **Multi-Agent Support**
   - Allow multiple `--agent` flags
   - Generate configs for all specified agents

### Phase 3: Update Command Support
1. **Marker-Based Updates**
   - Implement marker detection and replacement
   - Only update existing files (don't create new ones)
   - Preserve frontmatter during updates

2. **Per-File Update Logging**
   - Show which files were updated
   - Report any skipped files (missing)

### Phase 4: Testing & Validation
1. **Golden Snapshot Tests**
   - Test generated files for each agent
   - Verify frontmatter correctness
   - Validate marker placement

2. **Integration Tests**
   - Test multi-agent initialization
   - Test auto-detection logic
   - Test update behavior

### Phase 5: Documentation
1. **Update README**
   - Document all 18 supported agents
   - Provide installation examples
   - Show multi-agent usage

2. **Agent-Specific Guides**
   - Create setup guide for each major agent
   - Document agent-specific quirks

---

## Key Lessons from Existing Implementations

### 1. Use Actual CLI Tool Names
❌ **Wrong**: Use `cursor` as agent ID when CLI tool is `cursor-agent`
✅ **Correct**: Use `cursor-agent` as agent ID to match actual executable

**Why**: Eliminates special-case mappings throughout the codebase for tool checking.

### 2. Centralize Templates
❌ **Wrong**: Duplicate command bodies for each agent
✅ **Correct**: Define command bodies once, apply agent-specific wrappers

**Why**: Reduces maintenance burden and ensures consistency.

### 3. Marker Placement Discipline
❌ **Wrong**: Place markers inside YAML frontmatter
✅ **Correct**: Frontmatter first, then markers wrapping body

**Why**: Prevents YAML parse errors.

### 4. Idempotent Operations
❌ **Wrong**: Overwrite files completely on update
✅ **Correct**: Only update content within markers

**Why**: Preserves user customizations in frontmatter.

### 5. Explicit vs Implicit Agent Selection
❌ **Wrong**: Always require `--agent` flag
✅ **Correct**: Auto-detect with explicit override option

**Why**: Better UX - auto-detects for most users, explicit for edge cases.

### 6. Directory Creation Responsibility
The configurator (not the user) should handle creating nested directories like `.claude/commands/fspec/`.

### 7. Version-Specific Quirks
Some agents may not support frontmatter in certain versions. Fall back to Markdown-only format when frontmatter is unsupported.

---

## Agent-Specific Quirks & Limitations

### Amazon Q Developer CLI
- **Limitation**: Does not support custom arguments for slash commands
- **Workaround**: Commands must be hardcoded without argument placeholders
- **Status**: Partial support

### GitHub Copilot
- **Limitation**: Slash commands not fully exposed in all IDEs
- **Workaround**: Use `.github/prompts/` directory for prompt templates
- **Status**: Partial support

### Gemini & Qwen (TOML-based)
- **Format**: TOML instead of Markdown
- **Placeholder**: `{{args}}` instead of `$ARGUMENTS`
- **Quirk**: Different frontmatter structure

### Windsurf, Kilo Code, Roo Code (IDE-based)
- **Directory**: Use `workflows/` or `rules/` instead of `commands/`
- **Detection**: Cannot auto-detect via CLI tool (no CLI)
- **Workaround**: Detect via presence of IDE-specific directories

---

## Security & Privacy Considerations

### No API Keys Required
All agents work with user-provided API keys - fspec never handles or stores credentials.

### Local-First Architecture
- All configuration files stored locally in project
- No external API calls for agent configuration
- Agent detection happens entirely on local filesystem

### User Control
- Explicit `--agent` flag gives users full control
- Auto-detection can be overridden
- Multiple agents can coexist safely

---

## Conclusion

Implementing multi-agent support for fspec requires:

1. **Agent Registry**: Central configuration mapping for 18+ agents
2. **Configurator Pattern**: Strategy pattern for agent-specific behavior
3. **Template Management**: Centralized, DRY template system with system-reminder handling
4. **Marker-Based Updates**: Idempotent file updates preserving user customizations
5. **Auto-Detection**: Smart detection with explicit override
6. **Multi-Agent Support**: Simultaneous configuration for multiple agents

The architecture should prioritize:
- **Extensibility**: Easy to add new agents
- **Maintainability**: Minimal code duplication
- **User Experience**: Auto-detection with explicit control
- **Consistency**: Uniform behavior across all agents while respecting their conventions

This approach will make fspec **agent-agnostic** while providing **agent-optimized** experiences.

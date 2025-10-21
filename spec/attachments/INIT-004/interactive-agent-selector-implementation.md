# Interactive Agent Selector Implementation for fspec init

**Work Unit**: INIT-004
**Created**: 2025-10-21
**Author**: Claude (based on cage project patterns)

---

## Overview

This document provides a comprehensive implementation guide for transforming `fspec init` from a simple command with `--agent` flags into an **interactive ink/react-based agent selector** with arrow key navigation, multi-select support, and auto-detection.

The implementation follows patterns from the **cage project** (`~/projects/cage`), specifically:
- `useKeyboardNavigation.tsx` - keyboard input handling
- `MainMenu.tsx` - arrow navigation and selection UI
- `init.tsx` - initialization command structure

---

## Key Design Decisions

### 1. Interactive vs CLI Flags (Both Supported)

**Interactive Mode** (default):
```bash
fspec init
# Shows interactive agent selector with up/down arrows, space to select, enter to confirm
```

**CLI Mode** (backwards compatible):
```bash
fspec init --agent=cursor --agent=aider
# Non-interactive, directly installs specified agents
```

### 2. Auto-Detection with Confirmation

If agent directories are detected (`.claude/`, `.cursor/`, `.continue/`):
```
✓ Auto-detected: Claude Code (.claude/commands/ found)

  Would you like to install fspec for Claude Code?

  [*] Claude Code (auto-detected)
  [ ] Cursor
  [ ] Cline
  ...

  Space: Toggle selection | Enter: Confirm | ?: Help
```

### 3. Multi-Select Support

Users can select **multiple agents** for teams with mixed tools:
```
  [ ] Claude Code
  [*] Cursor          ← selected
  [ ] Cline
  [*] Aider           ← selected
  [ ] Windsurf

  2 agents selected
```

---

## Implementation Architecture

### File Structure

```
src/
├── commands/
│   └── init.ts                    # Main init command (updated)
├── components/
│   └── AgentSelector.tsx          # NEW: Interactive agent selector UI
├── hooks/
│   ├── useAgentSelection.tsx      # NEW: Agent selection logic hook
│   └── useSafeInput.tsx           # NEW: Safe input handling (from cage)
├── utils/
│   ├── agentDetection.ts          # NEW: Auto-detect agents
│   └── agentRegistry.ts           # NEW: Agent configuration registry
└── types/
    └── agents.ts                  # NEW: Agent types
```

---

## Step 1: Agent Registry (Data Structure)

**File**: `src/utils/agentRegistry.ts`

```typescript
export interface AgentConfig {
  id: string;
  name: string;
  description: string;
  slashCommandPath: string;
  slashCommandFormat: 'markdown' | 'toml';
  supportsSystemReminders: boolean;
  supportsMetaCognition: boolean;
  docTemplate: string; // e.g., 'CURSOR.md'
  detectionPaths: string[]; // e.g., ['.cursor/commands/', '.cursor/']
  available: boolean;
  category: 'ide' | 'cli' | 'extension';
}

export const AGENT_REGISTRY: AgentConfig[] = [
  {
    id: 'claude',
    name: 'Claude Code',
    description: 'Anthropic CLI (nested slash commands, system-reminders)',
    slashCommandPath: '.claude/commands/',
    slashCommandFormat: 'markdown',
    supportsSystemReminders: true,
    supportsMetaCognition: true,
    docTemplate: 'CLAUDE.md',
    detectionPaths: ['.claude/commands/', '.claude/'],
    available: true,
    category: 'cli',
  },
  {
    id: 'cursor',
    name: 'Cursor',
    description: 'IDE with AI assistant (flat slash commands)',
    slashCommandPath: '.cursor/commands/',
    slashCommandFormat: 'markdown',
    supportsSystemReminders: false,
    supportsMetaCognition: true,
    docTemplate: 'CURSOR.md',
    detectionPaths: ['.cursor/commands/', '.cursor/'],
    available: true,
    category: 'ide',
  },
  {
    id: 'cline',
    name: 'Cline',
    description: 'VS Code extension (no system-reminders)',
    slashCommandPath: '.cline/commands/',
    slashCommandFormat: 'markdown',
    supportsSystemReminders: false,
    supportsMetaCognition: true,
    docTemplate: 'CLINE.md',
    detectionPaths: ['.cline/', '.continue/'],
    available: true,
    category: 'extension',
  },
  {
    id: 'aider',
    name: 'Aider',
    description: 'CLI agent (no meta-cognitive prompts)',
    slashCommandPath: '.aider/',
    slashCommandFormat: 'markdown',
    supportsSystemReminders: false,
    supportsMetaCognition: false,
    docTemplate: 'AIDER.md',
    detectionPaths: [],
    available: true,
    category: 'cli',
  },
  // ... 14 more agents (windsurf, cody, roo-code, opencode, etc.)
];

export function getAgentById(id: string): AgentConfig | undefined {
  return AGENT_REGISTRY.find(agent => agent.id === id);
}

export function getAvailableAgents(): AgentConfig[] {
  return AGENT_REGISTRY.filter(agent => agent.available);
}
```

---

## Step 2: Agent Detection

**File**: `src/utils/agentDetection.ts`

```typescript
import { existsSync } from 'fs';
import { join } from 'path';
import { AGENT_REGISTRY, type AgentConfig } from './agentRegistry';

export interface DetectedAgent {
  agent: AgentConfig;
  detectedPath: string;
}

export function detectAgents(cwd: string): DetectedAgent[] {
  const detected: DetectedAgent[] = [];

  for (const agent of AGENT_REGISTRY) {
    for (const detectionPath of agent.detectionPaths) {
      const fullPath = join(cwd, detectionPath);
      if (existsSync(fullPath)) {
        detected.push({
          agent,
          detectedPath,
        });
        break; // Only record once per agent
      }
    }
  }

  return detected;
}

export function hasAnyAgentInstalled(cwd: string): boolean {
  return detectAgents(cwd).length > 0;
}
```

---

## Step 3: Agent Selection Hook

**File**: `src/hooks/useAgentSelection.tsx`

```typescript
import { useState, useCallback } from 'react';
import type { AgentConfig } from '../utils/agentRegistry';

export interface AgentSelectionState {
  selectedIndex: number;
  selectedAgents: Set<string>; // Set of agent IDs
  confirmed: boolean;
}

export function useAgentSelection(
  agents: AgentConfig[],
  preSelectedIds: string[] = []
) {
  const [state, setState] = useState<AgentSelectionState>({
    selectedIndex: 0,
    selectedAgents: new Set(preSelectedIds),
    confirmed: false,
  });

  const moveUp = useCallback(() => {
    setState(prev => ({
      ...prev,
      selectedIndex: prev.selectedIndex === 0
        ? agents.length - 1
        : prev.selectedIndex - 1,
    }));
  }, [agents.length]);

  const moveDown = useCallback(() => {
    setState(prev => ({
      ...prev,
      selectedIndex: (prev.selectedIndex + 1) % agents.length,
    }));
  }, [agents.length]);

  const toggleSelection = useCallback(() => {
    const agentId = agents[state.selectedIndex]?.id;
    if (!agentId) return;

    setState(prev => {
      const newSelected = new Set(prev.selectedAgents);
      if (newSelected.has(agentId)) {
        newSelected.delete(agentId);
      } else {
        newSelected.add(agentId);
      }
      return { ...prev, selectedAgents: newSelected };
    });
  }, [agents, state.selectedIndex]);

  const confirm = useCallback(() => {
    setState(prev => ({ ...prev, confirmed: true }));
  }, []);

  const isSelected = useCallback(
    (agentId: string) => state.selectedAgents.has(agentId),
    [state.selectedAgents]
  );

  return {
    selectedIndex: state.selectedIndex,
    selectedAgents: Array.from(state.selectedAgents),
    confirmed: state.confirmed,
    moveUp,
    moveDown,
    toggleSelection,
    confirm,
    isSelected,
  };
}
```

---

## Step 4: Safe Input Hook (from cage)

**File**: `src/hooks/useSafeInput.tsx`

```typescript
import { useInput } from 'ink';
import { useEffect, useRef } from 'react';

/**
 * Safe input hook that prevents duplicate handlers
 * Adapted from cage project
 */
export function useSafeInput(
  handler: (input: string, key: any) => void
): void {
  const handlerRef = useRef(handler);

  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useInput((input, key) => {
    handlerRef.current(input, key);
  });
}
```

---

## Step 5: Interactive Agent Selector Component

**File**: `src/components/AgentSelector.tsx`

```typescript
import React from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput';
import { useAgentSelection } from '../hooks/useAgentSelection';
import type { AgentConfig } from '../utils/agentRegistry';
import type { DetectedAgent } from '../utils/agentDetection';
import figures from 'figures';
import chalk from 'chalk';

interface AgentSelectorProps {
  agents: AgentConfig[];
  detectedAgents: DetectedAgent[];
  onConfirm: (selectedAgentIds: string[]) => void;
  onCancel?: () => void;
}

export const AgentSelector: React.FC<AgentSelectorProps> = ({
  agents,
  detectedAgents,
  onConfirm,
  onCancel,
}) => {
  // Pre-select detected agents
  const preSelectedIds = detectedAgents.map(d => d.agent.id);

  const {
    selectedIndex,
    selectedAgents,
    confirmed,
    moveUp,
    moveDown,
    toggleSelection,
    confirm,
    isSelected,
  } = useAgentSelection(agents, preSelectedIds);

  // Handle keyboard input
  useSafeInput((input, key) => {
    if (confirmed) return; // Ignore input after confirmation

    if (key.upArrow || input === 'k') {
      moveUp();
    } else if (key.downArrow || input === 'j') {
      moveDown();
    } else if (input === ' ') {
      toggleSelection();
    } else if (key.return) {
      if (selectedAgents.length > 0) {
        confirm();
        onConfirm(selectedAgents);
      }
    } else if (key.escape || input === 'q') {
      onCancel?.();
    } else if (input === '?') {
      // TODO: Show help
    }
  });

  // Render detection notice
  const renderDetectionNotice = () => {
    if (detectedAgents.length === 0) return null;

    return (
      <Box flexDirection="column" marginBottom={1}>
        <Text color="green">
          {figures.tick} Auto-detected: {detectedAgents.map(d => d.agent.name).join(', ')}
        </Text>
        <Text color="gray" dimColor>
          Pre-selected below. Space to toggle, Enter to confirm.
        </Text>
      </Box>
    );
  };

  // Render single agent item
  const renderAgentItem = (agent: AgentConfig, index: number) => {
    const isCurrent = index === selectedIndex;
    const isChecked = isSelected(agent.id);

    const pointer = isCurrent ? figures.pointer : ' ';
    const checkbox = isChecked ? figures.checkboxOn : figures.checkboxOff;

    const detectedBadge = detectedAgents.some(d => d.agent.id === agent.id)
      ? chalk.green(' (auto-detected)')
      : '';

    return (
      <Box key={agent.id} flexDirection="column" marginBottom={1}>
        <Box>
          <Text color={isCurrent ? 'cyan' : 'white'}>
            {pointer} {checkbox} {agent.name}{detectedBadge}
          </Text>
        </Box>
        <Box marginLeft={4}>
          <Text color="gray" dimColor>
            {agent.description}
          </Text>
        </Box>
      </Box>
    );
  };

  return (
    <Box flexDirection="column">
      {/* Header */}
      <Box
        borderStyle="round"
        borderColor="cyan"
        paddingX={2}
        paddingY={1}
        marginBottom={1}
      >
        <Text bold color="cyan">
          fspec init - Select AI Coding Agents
        </Text>
      </Box>

      {/* Detection notice */}
      {renderDetectionNotice()}

      {/* Agent list */}
      <Box flexDirection="column" paddingX={2}>
        {agents.map((agent, index) => renderAgentItem(agent, index))}
      </Box>

      {/* Footer */}
      <Box
        borderStyle="single"
        borderColor="gray"
        paddingX={2}
        marginTop={1}
      >
        <Text color="gray">
          {selectedAgents.length} selected |
          ↑↓ Navigate | Space: Toggle | Enter: Confirm | ESC: Cancel | ?: Help
        </Text>
      </Box>
    </Box>
  );
};
```

---

## Step 6: Updated Init Command

**File**: `src/commands/init.ts`

```typescript
import { Command } from 'commander';
import { render } from 'ink';
import React from 'react';
import { AgentSelector } from '../components/AgentSelector';
import { getAvailableAgents } from '../utils/agentRegistry';
import { detectAgents } from '../utils/agentDetection';
import { installAgents } from '../utils/agentInstaller';
import chalk from 'chalk';

export const initCommand = new Command('init')
  .description('Initialize fspec in your project with interactive agent selection')
  .option('--agent <agent>', 'Specify agent(s) non-interactively (can be repeated)', [])
  .option('--non-interactive', 'Skip interactive selection (use with --agent)')
  .action(async (options) => {
    const cwd = process.cwd();

    // Non-interactive mode (backwards compatible)
    if (options.agent.length > 0 || options.nonInteractive) {
      const agentIds = Array.isArray(options.agent) ? options.agent : [options.agent];

      console.log(chalk.cyan('Installing fspec for agents:'), agentIds.join(', '));

      try {
        await installAgents(cwd, agentIds);
        console.log(chalk.green('✓ fspec initialized successfully'));
        process.exit(0);
      } catch (error: any) {
        console.error(chalk.red('✗ Initialization failed:'), error.message);
        process.exit(1);
      }
      return;
    }

    // Interactive mode
    const availableAgents = getAvailableAgents();
    const detectedAgents = detectAgents(cwd);

    let selectedAgentIds: string[] = [];

    // Render interactive selector
    const { waitUntilExit } = render(
      React.createElement(AgentSelector, {
        agents: availableAgents,
        detectedAgents,
        onConfirm: (agentIds: string[]) => {
          selectedAgentIds = agentIds;
        },
        onCancel: () => {
          console.log(chalk.yellow('Installation cancelled'));
          process.exit(0);
        },
      })
    );

    // Wait for user to make selection
    await waitUntilExit();

    // Install selected agents
    if (selectedAgentIds.length > 0) {
      console.log(chalk.cyan('\nInstalling fspec for:'), selectedAgentIds.join(', '));

      try {
        await installAgents(cwd, selectedAgentIds);
        console.log(chalk.green('✓ fspec initialized successfully'));

        // Show what was installed
        selectedAgentIds.forEach(id => {
          const agent = availableAgents.find(a => a.id === id);
          if (agent) {
            console.log(chalk.gray(`  • ${agent.name}: spec/${agent.docTemplate}, ${agent.slashCommandPath}`));
          }
        });

        process.exit(0);
      } catch (error: any) {
        console.error(chalk.red('✗ Initialization failed:'), error.message);
        process.exit(1);
      }
    } else {
      console.log(chalk.yellow('No agents selected. Installation cancelled.'));
      process.exit(0);
    }
  });
```

---

## Step 7: Agent Installation Logic

**File**: `src/utils/agentInstaller.ts`

```typescript
import { mkdir, copyFile, access } from 'fs/promises';
import { join, dirname } from 'path';
import { getAgentById } from './agentRegistry';
import { generateAgentDoc } from './templateGenerator';
import { generateSlashCommands } from './slashCommandGenerator';

export async function installAgents(
  cwd: string,
  agentIds: string[]
): Promise<void> {
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (!agent) {
      throw new Error(`Unknown agent: ${agentId}`);
    }

    // 1. Install agent documentation (spec/AGENT.md)
    await installAgentDoc(cwd, agent);

    // 2. Install slash commands (all fspec commands)
    await installSlashCommands(cwd, agent);
  }
}

async function installAgentDoc(cwd: string, agent: AgentConfig): Promise<void> {
  const specDir = join(cwd, 'spec');
  await mkdir(specDir, { recursive: true });

  const targetPath = join(specDir, agent.docTemplate);

  // Generate agent-specific documentation
  const content = await generateAgentDoc(agent);

  await writeFile(targetPath, content);
}

async function installSlashCommands(
  cwd: string,
  agent: AgentConfig
): Promise<void> {
  const commandsDir = join(cwd, agent.slashCommandPath);
  await mkdir(commandsDir, { recursive: true });

  // Generate slash commands for ALL fspec commands
  const commands = [
    'validate',
    'format',
    'list-features',
    'create-feature',
    'add-scenario',
    'create-work-unit',
    'board',
    'show-work-unit',
    // ... all other fspec commands
  ];

  for (const command of commands) {
    const commandPath = join(commandsDir, `fspec-${command}.md`);
    const content = await generateSlashCommand(agent, command);
    await writeFile(commandPath, content);
  }
}
```

---

## Step 8: Template Generation (Dynamic)

**File**: `src/utils/templateGenerator.ts`

```typescript
import { readFile } from 'fs/promises';
import { join } from 'path';
import type { AgentConfig } from './agentRegistry';

/**
 * Generate agent-specific documentation from base template
 * Applies transformations:
 * 1. Remove <system-reminder> tags if not supported
 * 2. Remove meta-cognitive prompts if not supported
 * 3. Replace agent-specific references
 */
export async function generateAgentDoc(agent: AgentConfig): Promise<string> {
  // Read base template (bundled in dist/spec/templates/base/AGENT.md)
  const templatePath = await resolveTemplatePath('base/AGENT.md');
  let content = await readFile(templatePath, 'utf-8');

  // 1. Strip system-reminder tags if not supported
  if (!agent.supportsSystemReminders) {
    content = stripSystemReminders(content);
  }

  // 2. Remove meta-cognitive prompts if not supported
  if (!agent.supportsMetaCognition) {
    content = removeMetaCognitivePrompts(content);
  }

  // 3. Replace placeholders
  content = content
    .replace(/\{\{AGENT_NAME\}\}/g, agent.name)
    .replace(/\{\{SLASH_COMMAND_PATH\}\}/g, agent.slashCommandPath)
    .replace(/\{\{AGENT_ID\}\}/g, agent.id);

  return content;
}

function stripSystemReminders(content: string): string {
  // Remove <system-reminder>...</system-reminder> tags but preserve content
  return content.replace(
    /<system-reminder>([\s\S]*?)<\/system-reminder>/g,
    (_, inner) => {
      // Convert to markdown comment or plain text
      return `<!-- ${inner.trim()} -->`;
    }
  );
}

function removeMetaCognitivePrompts(content: string): string {
  // Remove phrases like "ultrathink", "deeply consider", etc.
  const patterns = [
    /ultrathink your next steps/gi,
    /ultrathink on/gi,
    /deeply consider/gi,
    /take a moment to reflect/gi,
  ];

  let result = content;
  for (const pattern of patterns) {
    result = result.replace(pattern, '');
  }

  return result;
}

async function resolveTemplatePath(relativePath: string): Promise<string> {
  // Try production path first (dist/spec/templates/)
  const prodPath = join(__dirname, '..', '..', 'spec', 'templates', relativePath);
  try {
    await access(prodPath);
    return prodPath;
  } catch {
    // Fall back to dev path (spec/templates/)
    return join(process.cwd(), 'spec', 'templates', relativePath);
  }
}
```

---

## Step 9: Slash Command Generation (Dynamic)

**File**: `src/utils/slashCommandGenerator.ts`

```typescript
import type { AgentConfig } from './agentRegistry';

/**
 * Generate agent-specific slash command file
 * Uses TemplateManager pattern: shared body + agent-specific wrapper
 */
export async function generateSlashCommand(
  agent: AgentConfig,
  command: string
): Promise<string> {
  const sharedBody = getCommandBody(command);
  const wrapper = getAgentWrapper(agent, command);

  return wrapper.replace('{{COMMAND_BODY}}', sharedBody);
}

function getCommandBody(command: string): string {
  // Shared command logic (agent-agnostic)
  const bodies: Record<string, string> = {
    validate: `
Validate all Gherkin feature files in the project for syntax errors.

Run: fspec validate

This checks all .feature files in spec/features/ directory using the official Cucumber Gherkin parser.
`,
    format: `
Format all Gherkin feature files using AST-based formatter.

Run: fspec format

This ensures consistent indentation, spacing, and structure across all feature files.
`,
    // ... bodies for all other commands
  };

  return bodies[command] || `Run: fspec ${command}`;
}

function getAgentWrapper(agent: AgentConfig, command: string): string {
  if (agent.slashCommandFormat === 'markdown') {
    // Claude/Cursor/Cline style (Markdown with YAML frontmatter)
    return `---
name: fspec: ${command}
description: ${getCommandDescription(command)}
category: fspec
tags: [fspec, ${command}]
---

# fspec ${command}

{{COMMAND_BODY}}

Arguments: \${ARGUMENTS}
`;
  } else if (agent.slashCommandFormat === 'toml') {
    // Other agents might use TOML
    return `[command]
name = "fspec: ${command}"
description = "${getCommandDescription(command)}"

{{COMMAND_BODY}}

Arguments: {{args}}
`;
  }

  return '{{COMMAND_BODY}}';
}

function getCommandDescription(command: string): string {
  const descriptions: Record<string, string> = {
    validate: 'Validate Gherkin feature files',
    format: 'Format Gherkin feature files',
    'list-features': 'List all feature files',
    'create-feature': 'Create new feature file',
    // ... descriptions for all commands
  };

  return descriptions[command] || `Run fspec ${command}`;
}
```

---

## Step 10: Dependencies to Add

**File**: `package.json`

```json
{
  "dependencies": {
    "ink": "^4.4.1",
    "react": "^18.2.0",
    "figures": "^5.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/node": "^20.0.0"
  }
}
```

---

## Step 11: Vite Configuration Updates

**File**: `vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import { VitePluginNode } from 'vite-plugin-node';
import { viteStaticCopy } from 'vite-plugin-static-copy';

export default defineConfig({
  plugins: [
    VitePluginNode({
      // ...existing config
    }),
    viteStaticCopy({
      targets: [
        // Copy agent template base files
        {
          src: 'spec/templates/base/*.md',
          dest: 'spec/templates/base',
        },
      ],
    }),
  ],
});
```

---

## Usage Examples

### Example 1: Auto-Detection + Interactive

```bash
$ fspec init
┌─────────────────────────────────────────────┐
│ fspec init - Select AI Coding Agents       │
└─────────────────────────────────────────────┘

✓ Auto-detected: Claude Code (.claude/commands/ found)
Pre-selected below. Space to toggle, Enter to confirm.

  [✓] Claude Code (auto-detected)
      Anthropic CLI (nested slash commands, system-reminders)

  [ ] Cursor
      IDE with AI assistant (flat slash commands)

  [ ] Cline
      VS Code extension (no system-reminders)

  ▸ [ ] Aider
      CLI agent (no meta-cognitive prompts)

1 selected | ↑↓ Navigate | Space: Toggle | Enter: Confirm | ESC: Cancel | ?: Help
```

### Example 2: Multi-Select for Team

User navigates and selects multiple:

```bash
  [✓] Claude Code (auto-detected)
  [✓] Cursor          ← user added this
  [ ] Cline
  [✓] Aider           ← user added this

3 selected | Press Enter to confirm
```

After pressing Enter:

```bash
Installing fspec for: claude, cursor, aider
✓ fspec initialized successfully
  • Claude Code: spec/CLAUDE.md, .claude/commands/
  • Cursor: spec/CURSOR.md, .cursor/commands/
  • Aider: spec/AIDER.md, .aider/
```

### Example 3: Non-Interactive (Backwards Compatible)

```bash
$ fspec init --agent=cursor --agent=aider
Installing fspec for agents: cursor, aider
✓ fspec initialized successfully
  • Cursor: spec/CURSOR.md, .cursor/commands/
  • Aider: spec/AIDER.md, .aider/
```

---

## Implementation Phases

### Phase 1: Foundation (INIT-004-A)
- [ ] Create agent registry (`agentRegistry.ts`) with 18 agents
- [ ] Implement agent detection (`agentDetection.ts`)
- [ ] Add safe input hook (`useSafeInput.tsx`)
- [ ] Test agent detection logic

### Phase 2: Interactive UI (INIT-004-B)
- [ ] Create agent selection hook (`useAgentSelection.tsx`)
- [ ] Build AgentSelector component (`AgentSelector.tsx`)
- [ ] Add ink/react dependencies
- [ ] Test keyboard navigation

### Phase 3: Template System (INIT-004-C)
- [ ] Create base template (`spec/templates/base/AGENT.md`)
- [ ] Implement template generator (`templateGenerator.ts`)
- [ ] Implement slash command generator (`slashCommandGenerator.ts`)
- [ ] Add system-reminder stripping logic
- [ ] Add meta-cognitive prompt removal logic

### Phase 4: Installation Logic (INIT-004-D)
- [ ] Implement agent installer (`agentInstaller.ts`)
- [ ] Update init command to use interactive selector
- [ ] Update Vite config to bundle templates
- [ ] Test full installation flow

### Phase 5: Testing & Refinement (INIT-004-E)
- [ ] Write tests for agent detection
- [ ] Write tests for template generation
- [ ] Write tests for multi-agent installation
- [ ] Test with all 18 agents
- [ ] Validate backwards compatibility

---

## Key Patterns from Cage

### 1. Safe Input Handling
```typescript
useSafeInput((input, key) => {
  if (key.upArrow) moveUp();
  else if (key.downArrow) moveDown();
  else if (input === ' ') toggleSelection();
});
```

### 2. State Management
```typescript
const [state, setState] = useState({
  selectedIndex: 0,
  selectedAgents: new Set(),
});
```

### 3. Visual Indicators
```typescript
const pointer = isCurrent ? figures.pointer : ' ';
const checkbox = isSelected ? figures.checkboxOn : figures.checkboxOff;
```

### 4. Accessibility Announcements
```typescript
announce(`Selected ${agent.name}`);
```

---

## Benefits of This Approach

1. **User-Friendly**: No need to remember agent IDs or flags
2. **Auto-Detection**: Automatically detects and suggests installed agents
3. **Multi-Select**: Supports teams with multiple tools
4. **Backwards Compatible**: CLI flags still work for automation
5. **Scalable**: Easy to add new agents to registry
6. **Consistent**: Uses proven patterns from cage project
7. **Professional**: Clean, modern CLI UX

---

## Testing Strategy

```typescript
describe('Interactive Agent Selector', () => {
  it('should auto-detect Claude Code', () => {
    // Mock .claude/commands/ directory
    // Run detection
    // Verify Claude Code is pre-selected
  });

  it('should allow multi-select with space key', () => {
    // Render component
    // Simulate space key
    // Verify agent is toggled
  });

  it('should install to correct paths', async () => {
    // Select cursor + aider
    // Confirm selection
    // Verify spec/CURSOR.md created
    // Verify .cursor/commands/fspec-*.md created
  });
});
```

---

## Conclusion

This implementation transforms `fspec init` into a modern, interactive CLI experience while maintaining backwards compatibility. By leveraging patterns from the cage project, we get:

- Robust keyboard navigation
- Professional UI with ink/react
- Flexible multi-agent support
- Clean separation of concerns

The TemplateManager pattern avoids the 360-540 file explosion by generating content on-the-fly, while the agent registry provides a single source of truth for all agent configurations.

**Ready for Example Mapping session to finalize requirements and edge cases.**

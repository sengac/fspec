# Post-Implementation Issues - INIT-004

**Work Unit**: INIT-004
**Created**: 2025-10-21
**Status**: Implementation Complete, Issues Catalogued
**Purpose**: Document bugs, anti-patterns, and technical debt discovered during critical review

---

## Executive Summary

After completing INIT-004 (Support multiple AI agents beyond Claude), a critical review revealed **25 issues** ranging from critical bugs to code quality improvements. This document tracks all identified problems for future remediation.

**Severity Breakdown**:
- 🔴 **Critical (Must Fix Before Release)**: 3 issues
- ⚠️ **High Priority (Fix Soon)**: 4 issues
- 🟡 **Medium Priority (Fix During Refactoring)**: 6 issues
- 🟢 **Low Priority (Code Quality)**: 12 issues

**Most Dangerous Issues**:
1. Deletes user's custom files (violates non-destructive requirement)
2. Path traversal security vulnerability
3. Duplicate agent directory conflicts

---

## 🔴 Critical Issues (Must Fix Before Release)

### Issue #1: DESTRUCTIVE FILE DELETION IN "NON-DESTRUCTIVE" INSTALLATION

**Location**: `src/commands/init.ts:78-83` (removeOtherAgentFiles)

**Problem**:
```typescript
// Deletes ENTIRE directory, not just fspec files
await rm(slashCmdPath, { recursive: true, force: true });
```

**Violates**: Business Rule #21 ("fspec init is NON-DESTRUCTIVE: if .cursor/commands/ already exists with custom files, create fspec.md alongside them. Never delete or overwrite user's custom files.")

**Impact**:
- User loses ALL custom slash commands when switching agents
- `.cursor/commands/my-custom-command.md` is permanently deleted
- Data loss, angry users

**Example**:
```bash
# User has custom files
.cursor/commands/
  ├── my-custom-command.md
  └── my-other-command.md

# User switches agents
fspec init --agent=aider

# ALL CURSOR FILES DELETED!
.cursor/commands/  # <-- GONE
```

**Proposed Fix**:
```typescript
async function removeOtherAgentFiles(cwd: string, keepAgentIds: string[]) {
  for (const agent of AGENT_REGISTRY) {
    if (keepAgentIds.includes(agent.id)) continue;

    // Only delete specific fspec files, not entire directory
    const filename = agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
    const fspecFile = join(cwd, agent.slashCommandPath, filename);

    try {
      await rm(fspecFile, { force: true });
    } catch {
      // File doesn't exist, ignore
    }

    // DO NOT delete the directory itself
  }
}
```

**Test Case Needed**:
```typescript
it('should preserve user files when removing old agent', async () => {
  const customFile = join(testDir, '.cursor', 'commands', 'my-custom.md');
  await writeFile(customFile, 'My custom command');

  await installAgents(testDir, ['claude']); // Switch to Claude

  // User file should still exist
  expect(existsSync(customFile)).toBe(true);
});
```

**Priority**: 🔴 **CRITICAL** - Ship blocker

---

### Issue #2: PATH TRAVERSAL VULNERABILITY

**Location**: `src/commands/init.ts:109-114` (installSlashCommand)

**Problem**:
```typescript
// No validation of agent.slashCommandPath
const commandsDir = join(cwd, agent.slashCommandPath);
await mkdir(commandsDir, { recursive: true });
```

**Security Risk**: If `slashCommandPath: '../../../etc/'`, could write files outside project directory.

**Impact**:
- Arbitrary file write vulnerability
- Could overwrite system files
- Security audit failure

**Attack Vector**:
```typescript
// Malicious agent config
{
  id: 'evil',
  slashCommandPath: '../../../tmp/pwned/',
}

// Results in:
// /home/user/project/../../../tmp/pwned/fspec.md
// = /tmp/pwned/fspec.md (outside project!)
```

**Proposed Fix**:
```typescript
async function installSlashCommand(cwd: string, agent: AgentConfig): Promise<void> {
  const commandsDir = join(cwd, agent.slashCommandPath);

  // Validate path doesn't escape cwd
  const normalized = normalize(commandsDir);
  const cwdNormalized = normalize(cwd);

  if (!normalized.startsWith(cwdNormalized)) {
    throw new Error(
      `Invalid agent configuration: slashCommandPath "${agent.slashCommandPath}" ` +
      `escapes project directory`
    );
  }

  await mkdir(commandsDir, { recursive: true });
  // ... rest of function
}
```

**Test Case Needed**:
```typescript
it('should reject path traversal in slashCommandPath', async () => {
  const maliciousAgent: AgentConfig = {
    id: 'evil',
    slashCommandPath: '../../../tmp/',
    // ... other required fields
  };

  await expect(
    installAgentFiles(testDir, maliciousAgent)
  ).rejects.toThrow('escapes project directory');
});
```

**Priority**: 🔴 **CRITICAL** - Security vulnerability

---

### Issue #3: DUPLICATE AGENT DIRECTORY CONFLICTS

**Location**: `src/utils/agentRegistry.ts:240-257`

**Problem**:
```typescript
{
  id: 'codex',
  slashCommandPath: '.codex/commands/',
  // ...
},
{
  id: 'codex-cli',
  slashCommandPath: '.codex/commands/',  // SAME PATH!
  // ...
}
```

**Impact**:
- Both agents write to same directory
- Installing `codex-cli` after `codex` overwrites files
- Agent switching breaks installation

**Example**:
```bash
fspec init --agent=codex       # Creates .codex/commands/fspec.md
fspec init --agent=codex-cli   # Overwrites .codex/commands/fspec.md
# Now codex installation is corrupted
```

**Proposed Fix Option A** (Separate Paths):
```typescript
{
  id: 'codex',
  slashCommandPath: '.codex/commands/',
},
{
  id: 'codex-cli',
  slashCommandPath: '.codex-cli/commands/',  // Different path
}
```

**Proposed Fix Option B** (Merge Agents):
```typescript
{
  id: 'codex',
  name: 'Codex / Codex CLI',
  slashCommandPath: '.codex/commands/',
  // Single agent supporting both variants
}
// Remove 'codex-cli' entirely
```

**Test Case Needed**:
```typescript
it('should not allow agents with duplicate slashCommandPath', () => {
  const paths = AGENT_REGISTRY.map(a => a.slashCommandPath);
  const uniquePaths = new Set(paths);

  expect(paths.length).toBe(uniquePaths.size);
});
```

**Priority**: 🔴 **CRITICAL** - Data corruption risk

---

## ⚠️ High Priority Issues (Fix Soon)

### Issue #4: FRAGILE REGEX FOR SYSTEM-REMINDER TRANSFORMATION

**Location**: `src/utils/templateGenerator.ts:38-48`

**Problem**:
```typescript
/<system-reminder>([\s\S]*?)<\/system-reminder>/g
```

**Fails On**:
- Nested tags: `<system-reminder><system-reminder>foo</system-reminder></system-reminder>`
- Malformed XML: `<system-reminder>foo</SystemReminder>` (case mismatch)
- Content containing closing tag: `Agent name: </system-reminder> parser`

**Impact**: Template transformation silently breaks, generates corrupted documentation.

**Proposed Fix**:
Use proper XML parser or more robust regex:
```typescript
function stripSystemReminders(content: string, agent: AgentConfig): string {
  if (agent.supportsSystemReminders) {
    return content;
  }

  // More robust: match opening tag, capture content, match closing tag
  // Use reluctant quantifier and enforce tag matching
  const regex = /<system-reminder[^>]*>([\s\S]*?)<\/system-reminder>/gi;

  return content.replace(regex, (match, inner) => {
    return transformToVisibleInstruction(inner, agent);
  });
}
```

Or use proper XML parser:
```typescript
import { parseStringPromise } from 'xml2js';

// Extract system-reminders using proper parser
// Then transform them
```

**Priority**: ⚠️ **HIGH** - Silent failures are dangerous

---

### Issue #5: AGGRESSIVE META-COGNITIVE PROMPT REMOVAL

**Location**: `src/utils/templateGenerator.ts:78-91`

**Problem**:
```typescript
result.replace(/ultrathink/gi, '');
```

**No Word Boundaries**: Matches substrings in unrelated words.

**False Positives**:
- "multirathinker" → "multier" (corrupted)
- "ultrathinking-docs.com" → "ing-docs.com" (broken URL)
- "I ultra think this is great" → "I ultra  this is great" (awkward grammar)

**Proposed Fix**:
```typescript
function removeMetaCognitivePrompts(content: string, agent: AgentConfig): string {
  if (agent.supportsMetaCognition) {
    return content;
  }

  const patterns = [
    /\bultrathink\b/gi,              // Word boundaries
    /\bdeeply consider\b/gi,
    /\btake a moment to reflect\b/gi,
  ];

  let result = content;
  for (const pattern of patterns) {
    result = result.replace(pattern, '');
  }

  return result.replace(/\n{3,}/g, '\n\n');
}
```

**Test Case Needed**:
```typescript
it('should not corrupt unrelated words when removing meta-cognitive prompts', () => {
  const template = `
    Visit ultrathinking-docs.com for more info.
    The multirathinker pattern is useful.
  `;

  const result = removeMetaCognitivePrompts(template, aiderAgent);

  expect(result).toContain('ultrathinking-docs.com');
  expect(result).toContain('multirathinker');
});
```

**Priority**: ⚠️ **HIGH** - Data corruption

---

### Issue #6: YAML FRONTMATTER DUPLICATION

**Location**: `src/commands/init.ts:223-232`

**Problem**:
```typescript
if (!template.startsWith('---')) {
  template = `---\nname: fspec\n...\n---\n\n${template}`;
}
```

**Fails If**:
- Template has whitespace before frontmatter: `\n\n---\nname: foo\n---`
- Template has comments before frontmatter: `<!-- comment -->\n---`

**Results In**: Invalid Markdown with duplicate YAML blocks.

**Proposed Fix**:
```typescript
function ensureYamlFrontmatter(template: string): string {
  const trimmed = template.trim();

  // Check if frontmatter exists in first 10 lines
  const lines = trimmed.split('\n').slice(0, 10);
  const hasFrontmatter = lines.some(line => line.trim() === '---');

  if (hasFrontmatter) {
    return template; // Already has frontmatter
  }

  // Add frontmatter
  return `---
name: fspec - Load Project Context
description: Load fspec workflow and ACDD methodology
category: Project
tags: [fspec, acdd, workflow]
---

${trimmed}`;
}
```

**Priority**: ⚠️ **HIGH** - Broken slash commands

---

### Issue #7: NO VALID AGENT LIST IN ERROR MESSAGE

**Location**: `src/commands/init.ts:32-34`

**Problem**:
```typescript
throw new Error(`Unknown agent: ${agentId}. Run 'fspec init --help'...`);
```

**Violates**: Business Rule #24 ("Invalid agent ID: Show error with list of valid agents, exit code 1")

**Current UX**:
```bash
$ fspec init --agent=invalid
Error: Unknown agent: invalid. Run 'fspec init --help' to see valid agent IDs.

# User must run another command to see list
$ fspec init --help
...
```

**Proposed Fix**:
```typescript
const validAgents = AGENT_REGISTRY.map(a => a.id).join(', ');
throw new Error(
  `Unknown agent: ${agentId}.\n\n` +
  `Valid agents:\n  ${validAgents}\n\n` +
  `Run 'fspec init --help' for more information.`
);
```

**Better UX**:
```bash
$ fspec init --agent=invalid
Error: Unknown agent: invalid.

Valid agents:
  claude, cursor, cline, aider, windsurf, copilot, gemini, qwen, kilocode,
  roo, codebuddy, amazonq, auggie, opencode, codex, factory, crush, codex-cli

Run 'fspec init --help' for more information.
```

**Priority**: ⚠️ **HIGH** - Poor user experience

---

## 🟡 Medium Priority Issues (Fix During Refactoring)

### Issue #8: NO BACKUP BEFORE AGENT SWITCHING

**Location**: `src/commands/init.ts:52-85` (removeOtherAgentFiles)

**Problem**: When switching agents (Business Rule #17), old agent files are permanently deleted. No undo, no backup.

**Impact**: User loses custom modifications to agent-specific files.

**Example**:
```bash
# User customizes Claude documentation
vim spec/CLAUDE.md
# ... adds custom notes

# User switches to Cursor
fspec init --agent=cursor
# spec/CLAUDE.md is GONE, no way to recover custom notes
```

**Proposed Fix Option A** (Backup):
```typescript
async function backupAgentFiles(cwd: string, agentId: string): Promise<void> {
  const agent = getAgentById(agentId);
  if (!agent) return;

  const backupDir = join(cwd, '.fspec-backups', new Date().toISOString());
  await mkdir(backupDir, { recursive: true });

  // Backup files
  const files = [
    join(cwd, agent.rootStubFile),
    join(cwd, 'spec', agent.docTemplate),
  ];

  for (const file of files) {
    if (existsSync(file)) {
      await copyFile(file, join(backupDir, basename(file)));
    }
  }

  console.log(chalk.gray(`Backed up ${agentId} files to ${backupDir}`));
}
```

**Proposed Fix Option B** (Warning):
```typescript
async function warnBeforeAgentSwitch(cwd: string, oldAgents: string[]): Promise<void> {
  console.warn(chalk.yellow(
    `⚠️  Switching agents will remove these files:\n` +
    oldAgents.map(id => {
      const agent = getAgentById(id);
      return `  - ${agent?.rootStubFile}\n  - spec/${agent?.docTemplate}`;
    }).join('\n')
  ));

  // Prompt user for confirmation (if interactive)
}
```

**Priority**: 🟡 **MEDIUM** - User experience improvement

---

### Issue #9: SYNCHRONOUS I/O IN DETECTION

**Location**: `src/utils/agentDetection.ts:18-26`

**Problem**:
```typescript
if (existsSync(fullPath)) { ... }
```

**Impact**:
- Blocks event loop
- In loop with 18 agents × ~2 paths each = 36 synchronous filesystem calls
- Slow with network filesystems (NFS, SMB)

**Proposed Fix**:
```typescript
export async function detectAgents(cwd: string): Promise<DetectedAgent[]> {
  const detected: DetectedAgent[] = [];

  for (const agent of AGENT_REGISTRY) {
    for (const detectionPath of agent.detectionPaths) {
      const fullPath = join(cwd, detectionPath);

      try {
        await access(fullPath); // Async instead of sync
        detected.push({ agent, detectedPath: detectionPath });
        break;
      } catch {
        // Path doesn't exist, try next
      }
    }
  }

  return detected;
}
```

**Priority**: 🟡 **MEDIUM** - Performance optimization

---

### Issue #10: TEMPLATE PATH RESOLUTION FRAGILE

**Location**: `src/utils/templateGenerator.ts:123-137`

**Problem**:
```typescript
const prodPath = join(__dirname, '..', '..', 'spec', 'templates', relativePath);
```

**Issues**:
- `__dirname` behaves differently in ESM vs CommonJS
- Breaks with bundlers (esbuild, webpack)
- Breaks with `npm link` (symlinked installations)
- Breaks in Docker with different directory structures

**Proposed Fix**:
```typescript
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

async function resolveTemplatePath(relativePath: string): Promise<string> {
  // Try multiple strategies
  const strategies = [
    // Strategy 1: Relative to current module
    () => join(__dirname, '..', '..', 'spec', 'templates', relativePath),

    // Strategy 2: Relative to process.cwd()
    () => join(process.cwd(), 'spec', 'templates', relativePath),

    // Strategy 3: Bundled in dist/
    () => join(__dirname, 'spec', 'templates', relativePath),
  ];

  for (const strategy of strategies) {
    const path = strategy();
    try {
      await access(path);
      return path;
    } catch {
      continue;
    }
  }

  throw new Error(`Template not found: ${relativePath}`);
}
```

**Priority**: 🟡 **MEDIUM** - Environment compatibility

---

### Issue #11: NO VALIDATION OF GENERATED CONTENT

**Location**: `src/commands/init.ts:125-185` (generateSlashCommandContent)

**Problem**: Generates TOML/Markdown but never validates syntax before writing.

**Impact**: Could write malformed files that crash agents.

**Example**:
```toml
# Invalid TOML - missing quotes
name = fspec - Load Project Context
```

**Proposed Fix**:
```typescript
import TOML from '@iarna/toml';

async function generateSlashCommandContent(agent: AgentConfig): Promise<string> {
  if (agent.slashCommandFormat === 'toml') {
    const tomlContent = `...`;

    // Validate TOML syntax
    try {
      TOML.parse(tomlContent);
    } catch (error) {
      throw new Error(`Generated invalid TOML for ${agent.id}: ${error.message}`);
    }

    return tomlContent;
  }

  // ... Markdown generation
}
```

**Priority**: 🟡 **MEDIUM** - Quality assurance

---

### Issue #12: SILENT FAILURES WITH `force: true`

**Location**: `src/commands/init.ts:62-67, 70-75, 78-83`

**Problem**:
```typescript
try {
  await rm(rootStubPath, { force: true });
} catch {
  // File may not exist
}
```

**Impact**: `force: true` silently ignores ALL errors (locked files, permissions, disk full).

**Proposed Fix**:
```typescript
try {
  await rm(rootStubPath);
} catch (error: any) {
  // Only ignore ENOENT (file doesn't exist)
  if (error.code !== 'ENOENT') {
    console.warn(chalk.yellow(
      `Warning: Failed to remove ${rootStubPath}: ${error.message}`
    ));
  }
}
```

**Priority**: 🟡 **MEDIUM** - Error visibility

---

### Issue #13: RACE CONDITION IN CONCURRENT INIT

**Location**: Entire `src/commands/init.ts`

**Problem**: No locking mechanism. Two concurrent `fspec init` processes could corrupt installation.

**Race Condition**:
```bash
# Terminal 1
fspec init --agent=cursor &

# Terminal 2 (simultaneously)
fspec init --agent=aider &

# Both processes:
# 1. Read "no files exist"
# 2. Try to create directories
# 3. Try to write files
# 4. Corrupt each other's installation
```

**Proposed Fix**:
```typescript
import lockfile from 'proper-lockfile';

export async function installAgents(cwd: string, agentIds: string[]): Promise<void> {
  const lockPath = join(cwd, '.fspec-init.lock');

  // Acquire lock
  const release = await lockfile.lock(cwd, {
    lockfilePath: lockPath,
    retries: { retries: 5, minTimeout: 100 },
  });

  try {
    // ... installation logic
  } finally {
    // Always release lock
    await release();
  }
}
```

**Priority**: 🟡 **MEDIUM** - Edge case in CI environments

---

## 🟢 Low Priority / Code Quality Issues

### Issue #14: GOD FILE - init.ts TOO LARGE

**Location**: `src/commands/init.ts` (400+ lines)

**Problem**: 8+ functions mixing concerns:
- Installation logic
- Deletion logic
- Template generation
- File I/O
- Slash command generation

**Impact**: Hard to maintain, test, understand.

**Proposed Refactoring**:
```
src/
├── installers/
│   ├── agentInstaller.ts       # installAgents, installAgentFiles
│   ├── fileRemover.ts          # removeOtherAgentFiles
│   ├── slashCommandGenerator.ts # generateSlashCommandContent
│   └── stubGenerator.ts        # installRootStub
└── commands/
    └── init.ts                 # CLI command handler only
```

**Priority**: 🟢 **LOW** - Code quality

---

### Issue #15: MAGIC STRINGS EVERYWHERE

**Location**: Throughout codebase

**Problem**:
```typescript
if (agentId === 'cursor') { ... }
if (agent.category === 'cli') { ... }
```

**Impact**: Easy to typo, no IDE autocomplete, hard to refactor.

**Proposed Fix**:
```typescript
// src/utils/agentConstants.ts
export const AGENT_IDS = {
  CLAUDE: 'claude',
  CURSOR: 'cursor',
  CLINE: 'cline',
  // ... all 18 agents
} as const;

export const AGENT_CATEGORIES = {
  IDE: 'ide',
  CLI: 'cli',
  EXTENSION: 'extension',
} as const;

// Usage
if (agentId === AGENT_IDS.CURSOR) { ... }
if (agent.category === AGENT_CATEGORIES.CLI) { ... }
```

**Priority**: 🟢 **LOW** - Developer experience

---

### Issue #16: NO LOGGING

**Location**: Throughout codebase

**Problem**: Silent failures everywhere. No visibility into what's happening.

**Example**:
```typescript
try {
  await rm(docPath, { force: true });
} catch {
  // User has NO IDEA this failed
}
```

**Proposed Fix**:
```typescript
import debug from 'debug';

const log = debug('fspec:init');
const logWarn = debug('fspec:init:warn');

try {
  log('Removing old doc file: %s', docPath);
  await rm(docPath);
  log('Successfully removed: %s', docPath);
} catch (error: any) {
  if (error.code === 'ENOENT') {
    log('File does not exist (skip): %s', docPath);
  } else {
    logWarn('Failed to remove %s: %s', docPath, error.message);
  }
}
```

**Priority**: 🟢 **LOW** - Debugging experience

---

### Issue #17: UNNECESSARY DYNAMIC IMPORT

**Location**: `src/commands/init.ts:53`

**Problem**:
```typescript
async function removeOtherAgentFiles() {
  const { AGENT_REGISTRY } = await import('../utils/agentRegistry');
}
```

**Impact**: Already imported at top, dynamic import is redundant and slower.

**Fix**: Use top-level import (already exists at line 6).

**Priority**: 🟢 **LOW** - Performance

---

### Issue #18: NO JSON SCHEMA VALIDATION FOR AgentConfig

**Location**: `src/utils/agentRegistry.ts`

**Problem**: No runtime validation that agent configs have required fields.

**Impact**: Runtime errors if config is malformed (missing required field).

**Proposed Fix**:
```typescript
import Ajv from 'ajv';

const ajv = new Ajv();

const agentConfigSchema = {
  type: 'object',
  required: [
    'id', 'name', 'description', 'slashCommandPath',
    'slashCommandFormat', 'supportsSystemReminders',
    'supportsMetaCognition', 'docTemplate', 'rootStubFile',
    'detectionPaths', 'available', 'category'
  ],
  properties: {
    id: { type: 'string' },
    name: { type: 'string' },
    // ... all fields
  },
};

const validate = ajv.compile(agentConfigSchema);

// Validate at module load time
for (const agent of AGENT_REGISTRY) {
  if (!validate(agent)) {
    throw new Error(`Invalid agent config: ${JSON.stringify(validate.errors)}`);
  }
}
```

**Priority**: 🟢 **LOW** - Type safety

---

### Issue #19: NO CACHING OF TEMPLATE READS

**Location**: `src/utils/templateGenerator.ts:10-17`

**Problem**: Reads base template from disk for every agent during multi-agent install.

**Impact**: Slow for `fspec init --agent=cursor --agent=aider --agent=cline ...`.

**Proposed Fix**:
```typescript
let templateCache: string | null = null;

export async function generateAgentDoc(agent: AgentConfig): Promise<string> {
  // Cache template content
  if (!templateCache) {
    const templatePath = await resolveTemplatePath('base/AGENT.md');
    templateCache = await readFile(templatePath, 'utf-8');
  }

  let content = templateCache;

  // Apply transformations...
  return content;
}
```

**Priority**: 🟢 **LOW** - Performance optimization

---

### Issue #20: MISSING TEST COVERAGE FOR IMPLEMENTATION

**Location**: Coverage tracking

**Problem**: Only 3/17 scenarios have implementation coverage linked.

**Missing Coverage**:
- Install Cursor slash command
- Transform system-reminders for Cline
- Install comprehensive slash command documentation
- Bundle templates in distribution
- Remove meta-cognitive prompts for CLI agents
- Interactive mode with auto-detection
- Install multiple agents simultaneously
- Agent switching (idempotent behavior)
- Non-destructive installation
- Invalid agent ID error
- Transform system-reminders for CLI agents without emoji
- Generate TOML format for Gemini CLI
- Install root stub for auto-loading agents
- Template transformation with placeholders

**Proposed Action**: Link all remaining scenarios to implementation code.

**Priority**: 🟢 **LOW** - Documentation completeness

---

### Issue #21: NO TESTS FOR EDGE CASES

**Location**: Test suite

**Missing Tests**:
- Empty agent IDs array: `installAgents(cwd, [])`
- Duplicate agents: `installAgents(cwd, ['cursor', 'cursor'])`
- Filesystem errors (disk full, permissions denied)
- Concurrent execution (race conditions)
- Windows path handling (`C:\`, backslashes)
- Symlink handling
- Very long agent names (path length limits)
- Special characters in agent names

**Proposed Action**: Add edge case test suite.

**Priority**: 🟢 **LOW** - Test coverage

---

### Issue #22: PLACEHOLDER REPLACEMENT IS NAIVE

**Location**: `src/utils/templateGenerator.ts:101-105`

**Problem**:
```typescript
.replace(/\{\{AGENT_NAME\}\}/g, agent.name)
```

**Fails If**: `agent.name` contains `{{` or regex special chars like `$`.

**Example**:
```typescript
agent.name = "Foo {Bar} $1"
// Results in broken template
```

**Proposed Fix**:
```typescript
function replacePlaceholders(content: string, agent: AgentConfig): string {
  // Escape special characters in replacement values
  const escapedName = agent.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const escapedPath = agent.slashCommandPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const escapedId = agent.id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  return content
    .replace(/\{\{AGENT_NAME\}\}/g, escapedName)
    .replace(/\{\{SLASH_COMMAND_PATH\}\}/g, escapedPath)
    .replace(/\{\{AGENT_ID\}\}/g, escapedId);
}
```

**Priority**: 🟢 **LOW** - Edge case

---

### Issue #23: SYMLINK HANDLING UNDEFINED

**Location**: `src/utils/agentDetection.ts:21-23`

**Problem**: `existsSync()` follows symlinks, but what if `.cursor/` is a broken symlink?

**Impact**: Detection fails with ENOENT.

**Proposed Fix**:
```typescript
import { lstatSync } from 'fs';

for (const detectionPath of agent.detectionPaths) {
  const fullPath = join(cwd, detectionPath);

  try {
    const stats = lstatSync(fullPath); // Don't follow symlinks

    if (stats.isDirectory() || stats.isSymbolicLink()) {
      detected.push({ agent, detectedPath: detectionPath });
      break;
    }
  } catch {
    // Path doesn't exist, try next
  }
}
```

**Priority**: 🟢 **LOW** - Edge case

---

### Issue #24: CASE SENSITIVITY ISSUES

**Location**: `src/utils/agentDetection.ts`

**Problem**: On macOS/Windows, `.Cursor/` and `.cursor/` are same. On Linux, different.

**Impact**: Inconsistent behavior across platforms.

**Proposed Fix**:
```typescript
export function detectAgents(cwd: string): DetectedAgent[] {
  const detected: DetectedAgent[] = [];

  for (const agent of AGENT_REGISTRY) {
    for (const detectionPath of agent.detectionPaths) {
      // Normalize to lowercase for case-insensitive comparison
      const normalizedPath = detectionPath.toLowerCase();
      const fullPath = join(cwd, normalizedPath);

      if (existsSync(fullPath)) {
        detected.push({ agent, detectedPath: normalizedPath });
        break;
      }
    }
  }

  return detected;
}
```

**Priority**: 🟢 **LOW** - Cross-platform compatibility

---

### Issue #25: MISSING VITE CONFIGURATION

**Location**: `vite.config.ts`

**Problem**: Business Rule #9 says "All agent templates must be bundled in fspec distribution using Vite copy plugin" but **we didn't update vite.config.ts**.

**Impact**: Base template (`spec/templates/base/AGENT.md`) won't be bundled in production build. Template resolution will fail in production.

**Current vite.config.ts**:
```typescript
// No viteStaticCopy plugin configured
```

**Proposed Fix**:
```typescript
import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';

export default defineConfig({
  plugins: [
    viteStaticCopy({
      targets: [
        {
          src: 'spec/templates/base/*.md',
          dest: 'spec/templates/base',
        },
      ],
    }),
  ],
  // ... rest of config
});
```

**Test After Fix**:
```bash
npm run build
ls dist/spec/templates/base/AGENT.md  # Should exist
```

**Priority**: 🟢 **LOW** - But required for production deployment

---

## Remediation Plan

### Immediate (Before Release)

1. **Fix Issue #1** (Destructive file deletion) - 2 hours
2. **Fix Issue #2** (Path traversal) - 1 hour
3. **Fix Issue #3** (Duplicate agent paths) - 30 minutes
4. **Fix Issue #25** (Vite bundling) - 30 minutes

**Total**: ~4 hours

### Short Term (Next Sprint)

5. **Fix Issue #4** (Fragile regex) - 2 hours
6. **Fix Issue #5** (Aggressive prompt removal) - 1 hour
7. **Fix Issue #6** (YAML frontmatter) - 1 hour
8. **Fix Issue #7** (Error messages) - 30 minutes

**Total**: ~4.5 hours

### Medium Term (Refactoring Sprint)

9-13. Medium priority issues - 8 hours

### Long Term (Technical Debt)

14-24. Low priority issues - 12 hours

---

## Testing Strategy

For each issue fix, add corresponding test case:

```typescript
describe('Issue Regression Tests', () => {
  describe('Issue #1: Non-destructive installation', () => {
    it('should preserve user files when switching agents', async () => {
      // Test case from issue description
    });
  });

  describe('Issue #2: Path traversal protection', () => {
    it('should reject paths escaping cwd', async () => {
      // Test case from issue description
    });
  });

  // ... all 25 issues
});
```

---

## Conclusion

Total identified issues: **25**
Estimated remediation time: **28.5 hours**
Critical issues requiring immediate attention: **4**

**Recommendation**: Fix critical issues (#1, #2, #3, #25) before any production release. Schedule high-priority issues for next sprint. Address medium/low priority issues during regular refactoring cycles.

**Next Steps**:
1. Create work units for critical issues
2. Prioritize in backlog
3. Fix issues with TDD (write failing tests first)
4. Update this document as issues are resolved

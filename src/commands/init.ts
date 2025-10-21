import { mkdir, writeFile, access, readFile, copyFile, rm } from 'fs/promises';
import type { Command } from 'commander';
import { join, dirname, isAbsolute, normalize, relative } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';
import { getAgentById, type AgentConfig } from '../utils/agentRegistry';
import { generateAgentDoc } from '../utils/templateGenerator';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface InitOptions {
  cwd?: string;
  installType: 'claude-code' | 'custom';
  customPath?: string;
  confirmOverwrite?: boolean;
}

interface InitResult {
  message: string;
  filePath: string;
  exitCode: number;
}

/**
 * Install fspec for multiple agents
 */
export async function installAgents(
  cwd: string,
  agentIds: string[]
): Promise<void> {
  // Validate agent IDs
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (!agent) {
      const { AGENT_REGISTRY } = await import('../utils/agentRegistry');
      const validAgents = AGENT_REGISTRY.filter(a => a.available)
        .map(a => `  - ${a.id}: ${a.description}`)
        .join('\n');
      throw new Error(
        `Unknown agent: ${agentId}.\n\nValid agent IDs:\n${validAgents}`
      );
    }
  }

  // Remove old agent files if switching agents (idempotent behavior)
  await removeOtherAgentFiles(cwd, agentIds);

  // Install each agent
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (agent) {
      await installAgentFiles(cwd, agent);
    }
  }
}

/**
 * Remove files for agents NOT in the installation list
 */
async function removeOtherAgentFiles(
  cwd: string,
  keepAgentIds: string[]
): Promise<void> {
  const { AGENT_REGISTRY } = await import('../utils/agentRegistry');

  for (const agent of AGENT_REGISTRY) {
    // Skip agents we're installing
    if (keepAgentIds.includes(agent.id)) {
      continue;
    }

    // Remove root stub file
    const rootStubPath = join(cwd, agent.rootStubFile);
    try {
      await rm(rootStubPath, { force: true });
    } catch {
      // File may not exist
    }

    // Remove full doc file
    const docPath = join(cwd, 'spec', agent.docTemplate);
    try {
      await rm(docPath, { force: true });
    } catch {
      // File may not exist
    }

    // Remove slash command file (NOT the entire directory)
    const filename =
      agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
    const slashCmdFile = join(cwd, agent.slashCommandPath, filename);
    try {
      await rm(slashCmdFile, { force: true });
    } catch {
      // File may not exist
    }
  }
}

/**
 * Install files for a single agent
 */
export async function installAgentFiles(
  cwd: string,
  agent: AgentConfig
): Promise<void> {
  // 1. Install root stub file (e.g., CURSOR.md or AGENTS.md)
  await installRootStub(cwd, agent);

  // 2. Install full documentation (spec/AGENT.md)
  await installFullDoc(cwd, agent);

  // 3. Install slash command file
  await installSlashCommand(cwd, agent);
}

/**
 * Install root stub file for auto-loading
 */
async function installRootStub(cwd: string, agent: AgentConfig): Promise<void> {
  const stubPath = join(cwd, agent.rootStubFile);

  // Generate short stub content
  const stubContent = `# ${agent.name} - fspec Project

This project uses **fspec** for Acceptance Criteria Driven Development (ACDD).

**Quick Start**:
- Run the \`/fspec\` slash command to load full workflow
- Or read \`spec/${agent.docTemplate}\` for complete documentation

**Commands**:
- \`fspec board\` - View Kanban board
- \`fspec --help\` - See all commands
- \`fspec help specs\` - Gherkin management
- \`fspec help work\` - Kanban workflow

**Learn more**: https://github.com/sengac/fspec
`;

  await writeFile(stubPath, stubContent, 'utf-8');
}

/**
 * Install full documentation file
 */
async function installFullDoc(cwd: string, agent: AgentConfig): Promise<void> {
  const specDir = join(cwd, 'spec');
  await mkdir(specDir, { recursive: true });

  const docPath = join(specDir, agent.docTemplate);

  // Generate agent-specific documentation
  const content = await generateAgentDoc(agent);

  await writeFile(docPath, content, 'utf-8');
}

/**
 * Install slash command file
 */
async function installSlashCommand(
  cwd: string,
  agent: AgentConfig
): Promise<void> {
  const commandsDir = join(cwd, agent.slashCommandPath);
  await mkdir(commandsDir, { recursive: true });

  // Use correct file extension based on format
  const filename =
    agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
  const commandPath = join(commandsDir, filename);

  // Generate slash command content
  const content = await generateSlashCommandContent(agent);

  await writeFile(commandPath, content, 'utf-8');
}

/**
 * Generate slash command content
 */
async function generateSlashCommandContent(
  agent: AgentConfig
): Promise<string> {
  if (agent.slashCommandFormat === 'toml') {
    // TOML format for Gemini CLI, Qwen Code
    return `[command]
name = "fspec - Load Project Context"
description = "Load fspec workflow and ACDD methodology"

# fspec Command - Load Full Context

Run these commands to load fspec context:

1. fspec --help
2. fspec help specs
3. fspec help work
4. fspec help discovery

Then read the comprehensive guide at spec/${agent.docTemplate} for full ACDD workflow.
`;
  }

  // Markdown format with YAML frontmatter (most agents)
  // Load the existing fspec.md template
  const possiblePaths = [
    join(__dirname, '.claude', 'commands', 'fspec.md'),
    join(__dirname, '..', '..', '.claude', 'commands', 'fspec.md'),
  ];

  let template: string | null = null;
  for (const path of possiblePaths) {
    try {
      template = await readFile(path, 'utf-8');
      break;
    } catch {
      continue;
    }
  }

  if (!template) {
    // Fallback to basic template
    return `---
name: fspec - Load Project Context
description: Load fspec workflow and ACDD methodology
category: Project
tags: [fspec, acdd, workflow]
---

# fspec Command - Load Full Context

Run these commands to load fspec context:

1. \`fspec --help\`
2. \`fspec help specs\`
3. \`fspec help work\`
4. \`fspec help discovery\`

Then read the comprehensive guide at \`spec/${agent.docTemplate}\` for full ACDD workflow.
`;
  }

  // Prepend YAML frontmatter if template doesn't have it
  if (!template.startsWith('---')) {
    template = `---
name: fspec - Load Project Context
description: Load fspec workflow and ACDD methodology
category: Project
tags: [fspec, acdd, workflow]
---

${template}`;
  }

  // Return the full template (which should be 1000+ lines)
  return template;
}

/**
 * Initialize fspec slash command in a project
 *
 * Creates the /fspec slash command file for Claude Code or custom location.
 * Includes interactive prompts for installation type and path selection.
 */
export async function init(options: InitOptions): Promise<InitResult> {
  const cwd = options.cwd || process.cwd();

  // Determine target path
  let targetPath: string;
  if (options.installType === 'claude-code') {
    targetPath = join(cwd, '.claude', 'commands', 'fspec.md');
  } else if (options.installType === 'custom') {
    if (!options.customPath) {
      throw new Error('Custom path is required when installType is "custom"');
    }
    // Validate custom path
    validatePath(options.customPath);
    targetPath = join(cwd, options.customPath);
  } else {
    throw new Error('Invalid installType. Must be "claude-code" or "custom"');
  }

  // Check if file already exists
  const fileExists = await checkFileExists(targetPath);
  if (fileExists) {
    if (!options.confirmOverwrite) {
      // User declined overwrite
      return {
        message: 'Installation cancelled',
        filePath: targetPath,
        exitCode: 0,
      };
    }
    // User confirmed overwrite, proceed
  }

  // Create parent directories if they don't exist
  const parentDir = dirname(targetPath);
  await mkdir(parentDir, { recursive: true });

  // Generate template content
  const templateContent = await generateTemplate();

  // Write file
  await writeFile(targetPath, templateContent, 'utf-8');

  // Copy CLAUDE.md template to spec/ directory
  await copyClaudeTemplate(cwd);

  // Calculate relative path for display
  const relativePath = relative(cwd, targetPath);

  // Return success message
  return {
    message: `✓ Installed /fspec command to ${relativePath}\n\nNext steps:\nRun /fspec in Claude Code to activate`,
    filePath: targetPath,
    exitCode: 0,
  };
}

/**
 * Validate that path is relative to current directory
 * Rejects parent paths (../) and absolute paths (/...)
 */
function validatePath(path: string): void {
  // Reject absolute paths
  if (isAbsolute(path)) {
    throw new Error(
      'Path must be relative to current directory (cannot use absolute paths like /... or ~/...)'
    );
  }

  // Reject home directory paths
  if (path.startsWith('~')) {
    throw new Error(
      'Path must be relative to current directory (cannot use absolute paths like /... or ~/...)'
    );
  }

  // Normalize path to resolve any ./ or ../
  const normalizedPath = normalize(path);

  // Check if normalized path tries to escape current directory
  if (normalizedPath.startsWith('..')) {
    throw new Error(
      'Path must be relative to current directory (cannot escape current directory with ../)'
    );
  }
}

/**
 * Check if file exists
 */
async function checkFileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Generate generic template from fspec.md
 *
 * Reads the current fspec.md template and replaces fspec-specific
 * examples with generic placeholders (example-project style).
 */
async function generateTemplate(): Promise<string> {
  // Read the fspec.md template from the installed package location
  // Try multiple paths to support different execution contexts:
  // 1. .claude/commands/fspec.md (production - within dist/)
  // 2. ../.claude/commands/fspec.md (development from src/commands/)
  const possiblePaths = [
    join(__dirname, '.claude', 'commands', 'fspec.md'), // From dist/ (production)
    join(__dirname, '..', '..', '.claude', 'commands', 'fspec.md'), // From src/commands/ (dev)
  ];

  let template: string | null = null;
  for (const path of possiblePaths) {
    try {
      template = await readFile(path, 'utf-8');
      break;
    } catch {
      // Try next path
      continue;
    }
  }

  if (!template) {
    throw new Error(
      'Could not find fspec.md template. Tried paths: ' +
        possiblePaths.join(', ')
    );
  }

  return template;
}

/**
 * Copy CLAUDE.md template to spec/ directory
 *
 * Copies the bundled CLAUDE.md template from templates/ to spec/CLAUDE.md
 * in the target project. Creates spec/ directory if it doesn't exist.
 * Always overwrites existing CLAUDE.md without prompting.
 */
async function copyClaudeTemplate(cwd: string): Promise<void> {
  // Resolve CLAUDE.md path from package installation
  // Try multiple paths to support different execution contexts:
  // 1. spec/CLAUDE.md (production - within dist/)
  // 2. ../../spec/CLAUDE.md (development from src/commands/)
  const possiblePaths = [
    join(__dirname, 'spec', 'CLAUDE.md'), // From dist/ (production)
    join(__dirname, '..', '..', 'spec', 'CLAUDE.md'), // From src/commands/ (dev)
  ];

  let sourcePath: string | null = null;
  for (const path of possiblePaths) {
    try {
      await access(path);
      sourcePath = path;
      break;
    } catch {
      // Try next path
      continue;
    }
  }

  if (!sourcePath) {
    throw new Error(
      'Could not find spec/CLAUDE.md. Tried paths: ' + possiblePaths.join(', ')
    );
  }

  // Target path in project
  const specDir = join(cwd, 'spec');
  const targetPath = join(specDir, 'CLAUDE.md');

  // Create spec/ directory if it doesn't exist
  await mkdir(specDir, { recursive: true });

  // Copy CLAUDE.md (overwrites if exists)
  await copyFile(sourcePath, targetPath);
}

export function registerInitCommand(program: Command): void {
  program
    .command('init')
    .description('Initialize /fspec slash command for Claude Code')
    .option('--type <type>', 'Installation type: claude-code or custom')
    .option(
      '--path <path>',
      'Custom installation path (relative to current directory)'
    )
    .option('--yes', 'Skip confirmation prompts (auto-confirm overwrite)')
    .action(
      async (options: { type?: string; path?: string; yes?: boolean }) => {
        try {
          let installType: 'claude-code' | 'custom';
          let customPath: string | undefined;
          // Determine install type
          if (options.type) {
            if (options.type !== 'claude-code' && options.type !== 'custom') {
              console.error(
                chalk.red('✗ Invalid type. Must be "claude-code" or "custom"')
              );
              process.exit(1);
            }
            installType = options.type as 'claude-code' | 'custom';
            if (installType === 'custom' && !options.path) {
              console.error(
                chalk.red('✗ --path is required when --type=custom')
              );
              process.exit(1);
            }
            customPath = options.path;
          } else {
            // Interactive mode (default to claude-code for now)
            installType = 'claude-code';
          }
          const result = await init({
            installType,
            customPath,
            confirmOverwrite: options.yes !== false,
          });
          console.log(chalk.green(result.message));
          process.exit(result.exitCode);
        } catch (error: any) {
          console.error(chalk.red('✗ Init failed:'), error.message);
          process.exit(1);
        }
      }
    );
}

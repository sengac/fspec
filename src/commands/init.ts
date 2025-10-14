import { mkdir, writeFile, access, readFile, copyFile } from 'fs/promises';
import type { Command } from 'commander';
import { join, dirname, isAbsolute, normalize, relative } from 'path';
import { fileURLToPath } from 'url';

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

  // Copy rspec.md template to .claude/commands/ directory
  await copyRspecTemplate(cwd);

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
  // 1. dist/.claude/commands/fspec.md (production)
  // 2. .claude/commands/fspec.md (development from project root)
  const possiblePaths = [
    join(__dirname, '..', '.claude', 'commands', 'fspec.md'), // From dist/
    join(__dirname, '..', '..', '.claude', 'commands', 'fspec.md'), // From src/commands/
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

  // Replace fspec-specific work unit IDs with generic examples
  template = template.replace(/CLI-\d+/g, 'EXAMPLE-001');
  template = template.replace(/TEST-\d+/g, 'EXAMPLE-002');
  template = template.replace(/AUTH-\d+/g, 'EXAMPLE-003');
  template = template.replace(/DASH-\d+/g, 'EXAMPLE-004');
  template = template.replace(/API-\d+/g, 'EXAMPLE-005');
  template = template.replace(/UI-\d+/g, 'EXAMPLE-006');
  template = template.replace(/SEC-\d+/g, 'EXAMPLE-007');
  template = template.replace(/HOOK-\d+/g, 'EXAMPLE-008');
  template = template.replace(/BACK-\d+/g, 'EXAMPLE-009');

  // Replace fspec-specific feature names with generic examples
  template = template.replace(
    /user-authentication\.feature/g,
    'example-feature.feature'
  );
  template = template.replace(
    /gherkin-validation\.feature/g,
    'example-validation.feature'
  );
  template = template.replace(
    /feature-file-validation\.feature/g,
    'example-feature.feature'
  );
  template = template.replace(/login\.feature/g, 'example-login.feature');
  template = template.replace(
    /shopping-cart\.feature/g,
    'example-cart.feature'
  );

  // Replace fspec-specific command examples with generic project examples
  template = template.replace(/fspec-test-/g, 'example-project-test-');

  // Ensure "example-project" placeholder is used
  if (!template.includes('example-project')) {
    template = template.replace(/fspec validate/g, 'example-project validate');
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
  // 1. dist/spec/CLAUDE.md (production - bundled from spec/)
  // 2. spec/CLAUDE.md (development/test from project root)
  const possiblePaths = [
    join(__dirname, '..', 'spec', 'CLAUDE.md'), // From dist/
    join(__dirname, '..', '..', 'spec', 'CLAUDE.md'), // From src/commands/
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

/**
 * Copy rspec.md template to .claude/commands/ directory
 *
 * Copies the bundled rspec.md template to .claude/commands/rspec.md
 * in the target project. Creates .claude/commands/ directory if it doesn't exist.
 * Always overwrites existing rspec.md without prompting.
 */
async function copyRspecTemplate(cwd: string): Promise<void> {
  // Resolve rspec.md path from package installation
  // Try multiple paths to support different execution contexts:
  // 1. dist/.claude/commands/rspec.md (production - bundled from .claude/)
  // 2. .claude/commands/rspec.md (development/test from project root)
  const possiblePaths = [
    join(__dirname, '..', '.claude', 'commands', 'rspec.md'), // From dist/
    join(__dirname, '..', '..', '.claude', 'commands', 'rspec.md'), // From src/commands/
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
      'Could not find .claude/commands/rspec.md. Tried paths: ' +
        possiblePaths.join(', ')
    );
  }

  // Target path in project
  const commandsDir = join(cwd, '.claude', 'commands');
  const targetPath = join(commandsDir, 'rspec.md');

  // Create .claude/commands/ directory if it doesn't exist
  await mkdir(commandsDir, { recursive: true });

  // Copy rspec.md (overwrites if exists)
  await copyFile(sourcePath, targetPath);
}

export function registerInitCommand(program: Command): void {
  program
    .command('init')
    .description('Initialize /fspec and /rspec slash commands for Claude Code')
    .option('--type <type>', 'Installation type: claude-code or custom')
    .option('--path <path>', 'Custom installation path (relative to current directory)')
    .option('--yes', 'Skip confirmation prompts (auto-confirm overwrite)')
    .action(async (options: { type?: string; path?: string; yes?: boolean }) => {
      try {
        let installType: 'claude-code' | 'custom';
        let customPath: string | undefined;
        // Determine install type
        if (options.type) {
          if (options.type !== 'claude-code' && options.type !== 'custom') {
            console.error(
              chalk.red(
                '✗ Invalid type. Must be "claude-code" or "custom"'
              )
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
    });
}

import { mkdir, writeFile, access, readFile } from 'fs/promises';
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
  // When built, __dirname will be dist/, so we need to go up and find .claude/commands/fspec.md
  const templatePath = join(
    __dirname,
    '..',
    '.claude',
    'commands',
    'fspec.md'
  );
  let template = await readFile(templatePath, 'utf-8');

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

import { exec } from 'child_process';
import { promisify } from 'util';
import { access } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';
import { glob } from 'tinyglobby';

const execAsync = promisify(exec);

// Get the project root (where package.json is located)
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..', '..');

export interface FormatOptions {
  cwd?: string;
  file?: string;
}

export interface FormatResult {
  formattedCount: number;
}

export async function formatFeatures(options: FormatOptions = {}): Promise<FormatResult> {
  const cwd = options.cwd || process.cwd();

  let files: string[];

  if (options.file) {
    // Format specific file
    const filePath = join(cwd, options.file);
    try {
      await access(filePath);
      files = [options.file];
    } catch (error) {
      throw new Error(`File not found: ${options.file}`);
    }
  } else {
    // Format all feature files
    files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    if (files.length === 0) {
      return { formattedCount: 0 };
    }
  }

  try {
    // Format each file individually to ensure consistent counting
    let formattedCount = 0;

    for (const file of files) {
      // Use prettier from project's node_modules directly
      const prettierBin = join(projectRoot, 'node_modules', '.bin', 'prettier');
      const absoluteFilePath = join(cwd, file);
      await execAsync(
        `"${prettierBin}" --write --parser gherkin "${absoluteFilePath}"`,
        { cwd: projectRoot } // Run from project root so prettier can find its config and plugins
      );
      formattedCount++;
    }

    return { formattedCount };
  } catch (error: any) {
    throw new Error(`Failed to format files: ${error.message}`);
  }
}

export async function formatCommand(file?: string): Promise<void> {
  try {
    const result = await formatFeatures({ file });

    if (result.formattedCount === 0) {
      console.log(chalk.yellow('No feature files found to format'));
      process.exit(0);
    }

    if (file) {
      console.log(chalk.green(`✓ Formatted ${file}`));
    } else {
      console.log(chalk.green(`✓ Formatted ${result.formattedCount} feature files`));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

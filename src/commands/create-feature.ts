import { mkdir, writeFile, access } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import { toKebabCase } from '../utils/file-helpers';
import { generateFeatureTemplate } from '../utils/templates';
import { getFileNamingReminder } from '../utils/system-reminder';
import { createCoverageFile } from '../utils/coverage-file';

export async function createFeature(
  name: string,
  cwd: string = process.cwd()
): Promise<string> {
  const featuresDir = join(cwd, 'spec', 'features');
  const fileName = `${toKebabCase(name)}.feature`;
  const filePath = join(featuresDir, fileName);

  // Check if file already exists
  try {
    await access(filePath);
    throw new Error(
      `File already exists: spec/features/${fileName}\nSuggestion: Use a different name or delete the existing file`
    );
  } catch (error: any) {
    if (error.code === 'EACCES') {
      throw new Error(
        `Permission denied: Cannot access spec/features/${fileName}\nSuggestion: Check file permissions for the spec/features directory`
      );
    }
    if (error.code !== 'ENOENT') {
      throw new Error(
        `Failed to check if file exists: ${error.message}\nSuggestion: Verify you have access to the spec/features directory`
      );
    }
    // File doesn't exist, proceed
  }

  // Create spec/features/ directory if it doesn't exist
  try {
    await mkdir(featuresDir, { recursive: true });
  } catch (error: any) {
    if (error.code === 'EACCES') {
      throw new Error(
        `Permission denied: Cannot create directory spec/features/\nSuggestion: Check file permissions`
      );
    }
    throw new Error(`Failed to create directory: ${error.message}`);
  }

  // Generate template content
  const content = generateFeatureTemplate(name);

  // Write file
  try {
    await writeFile(filePath, content, 'utf-8');
  } catch (error: any) {
    if (error.code === 'EACCES') {
      throw new Error(
        `Permission denied: Cannot write to ${fileName}\nSuggestion: Check file permissions`
      );
    }
    if (error.code === 'ENOSPC') {
      throw new Error(
        `No space left on device: Cannot write ${fileName}\nSuggestion: Free up disk space`
      );
    }
    throw new Error(`Failed to write file: ${error.message}`);
  }

  // Create coverage file (graceful degradation - don't fail feature creation)
  try {
    const coverageResult = await createCoverageFile(filePath);
    // Store result for display in command function
    (createFeature as any).lastCoverageResult = coverageResult;
  } catch (error: any) {
    // Log warning but don't fail feature creation
    (createFeature as any).lastCoverageResult = {
      status: 'error',
      message: `Warning: Failed to create coverage file: ${error.message}`,
    };
  }

  return filePath;
}

export async function createFeatureCommand(name: string): Promise<void> {
  try {
    // Check for file naming anti-patterns BEFORE creating the file
    const kebabName = toKebabCase(name);
    const fileNamingReminder = getFileNamingReminder(kebabName);

    const filePath = await createFeature(name);
    const fileName = filePath.split('/').slice(-2).join('/'); // spec/features/file.feature

    console.log(chalk.green(`✓ Created ${fileName}`));
    console.log(chalk.gray('  Edit the file to add your scenarios'));

    // Display coverage file creation result
    const coverageResult = (createFeature as any).lastCoverageResult;
    if (coverageResult) {
      if (coverageResult.status === 'created') {
        console.log(chalk.green(coverageResult.message));
      } else if (coverageResult.status === 'skipped') {
        console.log(chalk.yellow(coverageResult.message));
      } else if (coverageResult.status === 'recreated') {
        console.log(chalk.yellow(coverageResult.message));
      } else if (coverageResult.status === 'error') {
        console.log(chalk.red(coverageResult.message));
      }
    }

    // Display file naming reminder if anti-pattern detected
    if (fileNamingReminder) {
      console.log('\n' + fileNamingReminder);
    }

    // Check for prefill in generated file
    const { readFile } = await import('fs/promises');
    const { detectPrefill } = await import('../utils/prefill-detection');
    const content = await readFile(filePath, 'utf-8');
    const prefillResult = detectPrefill(content);

    if (prefillResult.hasPrefill && prefillResult.systemReminder) {
      console.log('\n' + prefillResult.systemReminder);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerCreateFeatureCommand(program: Command): void {
  program
    .command('create-feature')
    .description('Create a new feature file with template')
    .argument('<name>', 'Feature name (e.g., "User Authentication")')
    .action(createFeatureCommand);
}

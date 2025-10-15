import { mkdir, writeFile, access, readFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import { toKebabCase } from '../utils/file-helpers';
import { generateFeatureTemplate } from '../utils/templates';
import { getFileNamingReminder } from '../utils/system-reminder';
import { createCoverageFile } from '../utils/coverage-file';
import { detectPrefill } from '../utils/prefill-detection';

export interface CreateFeatureResult {
  filePath: string;
  prefillDetection: {
    hasPrefill: boolean;
    matches: Array<{
      pattern: string;
      line?: number;
      context?: string;
      suggestion: string;
    }>;
    systemReminder?: string;
  };
  coverageFile: {
    created: boolean;
    path?: string;
    status: 'created' | 'skipped' | 'recreated' | 'error';
    message: string;
  };
  fileNamingReminder?: string;
}

export async function createFeature(
  name: string,
  cwd: string = process.cwd()
): Promise<CreateFeatureResult> {
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
  let coverageFileResult: CreateFeatureResult['coverageFile'];
  try {
    const coverageResult = await createCoverageFile(filePath);
    coverageFileResult = {
      created: coverageResult.status === 'created',
      path: coverageResult.path,
      status: coverageResult.status,
      message: coverageResult.message,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    coverageFileResult = {
      created: false,
      status: 'error',
      message: `Warning: Failed to create coverage file: ${errorMessage}`,
    };
  }

  // Detect prefill in generated content
  const generatedContent = await readFile(filePath, 'utf-8');
  const prefillResult = detectPrefill(generatedContent);

  // Get file naming reminder
  const kebabName = toKebabCase(name);
  const fileNamingReminder = getFileNamingReminder(kebabName);

  return {
    filePath,
    prefillDetection: {
      hasPrefill: prefillResult.hasPrefill,
      matches: prefillResult.matches,
      systemReminder: prefillResult.systemReminder,
    },
    coverageFile: coverageFileResult,
    fileNamingReminder: fileNamingReminder || undefined,
  };
}

export async function createFeatureCommand(name: string): Promise<void> {
  try {
    const result = await createFeature(name);
    const fileName = result.filePath.split('/').slice(-2).join('/'); // spec/features/file.feature

    console.log(chalk.green(`✓ Created ${fileName}`));
    console.log(chalk.gray('  Edit the file to add your scenarios'));

    // Display coverage file creation result
    if (result.coverageFile.status === 'created') {
      console.log(chalk.green(result.coverageFile.message));
    } else if (result.coverageFile.status === 'skipped') {
      console.log(chalk.yellow(result.coverageFile.message));
    } else if (result.coverageFile.status === 'recreated') {
      console.log(chalk.yellow(result.coverageFile.message));
    } else if (result.coverageFile.status === 'error') {
      console.log(chalk.red(result.coverageFile.message));
    }

    // Display file naming reminder if anti-pattern detected
    if (result.fileNamingReminder) {
      console.log('\n' + result.fileNamingReminder);
    }

    // Display prefill detection system-reminder
    if (result.prefillDetection.hasPrefill && result.prefillDetection.systemReminder) {
      console.log('\n' + result.prefillDetection.systemReminder);
    }

    process.exit(0);
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(chalk.red('Error:'), errorMessage);
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

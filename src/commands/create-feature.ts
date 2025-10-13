import { mkdir, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { toKebabCase } from '../utils/file-helpers';
import { generateFeatureTemplate } from '../utils/templates';
import { getFileNamingReminder } from '../utils/system-reminder';

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

    // Display file naming reminder if anti-pattern detected
    if (fileNamingReminder) {
      console.log('\n' + fileNamingReminder);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

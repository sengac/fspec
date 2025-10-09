import { mkdir, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { toKebabCase } from '../utils/file-helpers';
import { generateFeatureTemplate } from '../utils/templates';

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
    if (error.code !== 'ENOENT') {
      throw error; // File exists or other error
    }
    // File doesn't exist, proceed
  }

  // Create spec/features/ directory if it doesn't exist
  await mkdir(featuresDir, { recursive: true });

  // Generate template content
  const content = generateFeatureTemplate(name);

  // Write file
  await writeFile(filePath, content, 'utf-8');

  return filePath;
}

export async function createFeatureCommand(name: string): Promise<void> {
  try {
    const filePath = await createFeature(name);
    const fileName = filePath.split('/').slice(-2).join('/'); // spec/features/file.feature

    console.log(chalk.green(`✓ Created ${fileName}`));
    console.log(chalk.gray('  Edit the file to add your scenarios'));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

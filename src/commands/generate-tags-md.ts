import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Tags } from '../types/tags';
import { validateTagsJson } from '../validators/json-schema';
import { generateTagsMd } from '../generators/tags-md';

interface GenerateTagsMdOptions {
  cwd?: string;
}

interface GenerateTagsMdResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function generateTagsMdCommand(
  options: GenerateTagsMdOptions = {}
): Promise<GenerateTagsMdResult> {
  const { cwd = process.cwd() } = options;

  try {
    const tagsJsonPath = join(cwd, 'spec/tags.json');
    const tagsMdPath = join(cwd, 'spec/TAGS.md');

    // Check if tags.json exists
    if (!existsSync(tagsJsonPath)) {
      return {
        success: false,
        error: 'tags.json not found: spec/tags.json',
      };
    }

    // Validate tags.json against schema
    const validation = await validateTagsJson(tagsJsonPath);
    if (!validation.valid) {
      return {
        success: false,
        error: `tags.json has validation errors: ${validation.errors?.join(', ')}`,
      };
    }

    // Read tags.json
    const content = await readFile(tagsJsonPath, 'utf-8');
    const tagsData: Tags = JSON.parse(content);

    // Generate TAGS.md
    const markdown = await generateTagsMd(tagsData);

    // Write TAGS.md
    await writeFile(tagsMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: 'Generated spec/TAGS.md from spec/tags.json',
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function generateTagsMdCommandCLI(): Promise<void> {
  try {
    const result = await generateTagsMdCommand();

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

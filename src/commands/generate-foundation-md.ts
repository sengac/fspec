import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Foundation } from '../types/foundation';
import { validateFoundationJson } from '../validators/json-schema';
import { generateFoundationMd } from '../generators/foundation-md';

interface GenerateFoundationMdOptions {
  cwd?: string;
}

interface GenerateFoundationMdResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function generateFoundationMdCommand(
  options: GenerateFoundationMdOptions = {}
): Promise<GenerateFoundationMdResult> {
  const { cwd = process.cwd() } = options;

  try {
    const foundationJsonPath = join(cwd, 'spec/foundation.json');
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');

    // Check if foundation.json exists
    if (!existsSync(foundationJsonPath)) {
      return {
        success: false,
        error: 'foundation.json not found: spec/foundation.json',
      };
    }

    // Validate foundation.json against schema
    const validation = await validateFoundationJson(foundationJsonPath);
    if (!validation.valid) {
      const errorMessages =
        validation.errors
          ?.map(err => `${err.instancePath}: ${err.message}`)
          .join('; ') || 'Unknown errors';
      return {
        success: false,
        error: `foundation.json has validation errors: ${errorMessages}`,
      };
    }

    // Read foundation.json
    const content = await readFile(foundationJsonPath, 'utf-8');
    const foundationData: Foundation = JSON.parse(content);

    // Generate FOUNDATION.md
    const markdown = await generateFoundationMd(foundationData);

    // Write FOUNDATION.md
    await writeFile(foundationMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: 'Generated spec/FOUNDATION.md from spec/foundation.json',
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function generateFoundationMdCommandCLI(): Promise<void> {
  try {
    const result = await generateFoundationMdCommand();

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

export function registerGenerateFoundationMdCommand(program: Command): void {
  program
    .command('generate-foundation-md')
    .description('Generate FOUNDATION.md from foundation.json')
    .action(generateFoundationMdCommandCLI);
}

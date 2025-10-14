import { readFile, access } from 'fs/promises';
import type { Command } from 'commander';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface ShowFoundationSchemaOptions {
  cwd?: string;
}

interface ShowFoundationSchemaResult {
  success: boolean;
  output?: string;
  error?: string;
}

export async function showFoundationSchema(
  options: ShowFoundationSchemaOptions = {}
): Promise<ShowFoundationSchemaResult> {
  const cwd = options.cwd || process.cwd();

  try {
    // Read the schema file from bundled location
    // Try multiple paths to support different execution contexts:
    // 1. dist/schemas/ (production, when running from dist/index.js)
    // 2. src/schemas/ (tests, when running from src/commands/*.ts)
    const possiblePaths = [
      join(__dirname, 'schemas', 'foundation.schema.json'), // From dist/
      join(__dirname, '..', 'schemas', 'foundation.schema.json'), // From src/commands/
    ];

    let schemaContent: string | null = null;
    for (const path of possiblePaths) {
      try {
        schemaContent = await readFile(path, 'utf-8');
        break;
      } catch {
        // Try next path
        continue;
      }
    }

    if (!schemaContent) {
      throw new Error(
        'Could not find foundation.schema.json. Tried paths: ' +
          possiblePaths.join(', ')
      );
    }

    return {
      success: true,
      output: schemaContent,
    };
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : 'Unknown error';
    return {
      success: false,
      error: `Failed to read foundation schema: ${errorMessage}`,
    };
  }
}

export async function showFoundationSchemaCommand(): Promise<void> {
  try {
    const result = await showFoundationSchema();

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(result.output);
    process.exit(0);
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : 'Unknown error';
    console.error(chalk.red('Error:'), errorMessage);
    process.exit(1);
  }
}

export function registerShowFoundationSchemaCommand(program: Command): void {
  program
    .command('show-foundation-schema')
    .description('Display foundation.json JSON Schema with guidance for AI agents')
    .action(showFoundationSchemaCommand);
}

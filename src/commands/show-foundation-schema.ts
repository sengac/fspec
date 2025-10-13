import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

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
    // Read the schema file from src/schemas/foundation.schema.json
    const schemaPath = join(__dirname, '../schemas/foundation.schema.json');
    const schemaContent = await readFile(schemaPath, 'utf-8');

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

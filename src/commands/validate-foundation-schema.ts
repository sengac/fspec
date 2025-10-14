import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';
import Ajv from 'ajv';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface ValidateFoundationSchemaOptions {
  cwd?: string;
}

interface ValidateFoundationSchemaResult {
  success: boolean;
  output?: string;
  error?: string;
}

export async function validateFoundationSchema(
  options: ValidateFoundationSchemaOptions = {}
): Promise<ValidateFoundationSchemaResult> {
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

    const schema = JSON.parse(schemaContent);

    // Read the foundation.json file
    const foundationPath = join(cwd, 'spec/foundation.json');
    const foundationContent = await readFile(foundationPath, 'utf-8');
    const foundation = JSON.parse(foundationContent);

    // Validate using Ajv
    // Use strictSchema: false and logger: false to suppress warnings about unknown formats like "uri"
    const ajv = new Ajv({
      allErrors: true,
      verbose: true,
      strictSchema: false,
      logger: false,
    });
    const validate = ajv.compile(schema);
    const valid = validate(foundation);

    if (!valid) {
      // Format errors in a user-friendly way
      const errors = validate.errors || [];
      const errorMessages = errors.map((err) => {
        const path = err.instancePath || err.schemaPath;

        // Special handling for minimum array length
        if (err.keyword === 'minItems') {
          const arrayPath = path.replace(/^\//, '').replace(/\//g, '.');
          return `Field ${arrayPath} must have at least ${err.params.limit} items (found ${err.data?.length || 0})`;
        }

        return `${path}: ${err.message}`;
      });

      return {
        success: false,
        error: errorMessages.join('\n'),
      };
    }

    return {
      success: true,
      output: '✓ foundation.json is valid according to the schema',
    };
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : 'Unknown error';

    if (errorMessage.includes('ENOENT')) {
      return {
        success: false,
        error: 'foundation.json not found in spec/ directory',
      };
    }

    return {
      success: false,
      error: `Failed to validate foundation schema: ${errorMessage}`,
    };
  }
}

export async function validateFoundationSchemaCommand(): Promise<void> {
  try {
    const result = await validateFoundationSchema();

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green(result.output));
    process.exit(0);
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : 'Unknown error';
    console.error(chalk.red('Error:'), errorMessage);
    process.exit(1);
  }
}

export function registerValidateFoundationSchemaCommand(program: Command): void {
  program
    .command('validate-foundation-schema')
    .description('Validate foundation.json against JSON Schema')
    .action(validateFoundationSchemaCommand);
}

import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import Ajv from 'ajv';

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
    // Read the schema file
    const schemaPath = join(__dirname, '../schemas/foundation.schema.json');
    const schemaContent = await readFile(schemaPath, 'utf-8');
    const schema = JSON.parse(schemaContent);

    // Read the foundation.json file
    const foundationPath = join(cwd, 'spec/foundation.json');
    const foundationContent = await readFile(foundationPath, 'utf-8');
    const foundation = JSON.parse(foundationContent);

    // Validate using Ajv
    const ajv = new Ajv({ allErrors: true, verbose: true });
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

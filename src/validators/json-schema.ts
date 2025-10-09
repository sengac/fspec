import Ajv, { type ErrorObject } from 'ajv';
import addFormats from 'ajv-formats';
import { readFile } from 'fs/promises';
import { join } from 'path';

export interface ValidationResult {
  valid: boolean;
  errors: ErrorObject[];
}

export interface ValidationResults {
  filePath: string;
  valid: boolean;
  errors: ErrorObject[];
}

/**
 * Validate a JSON file against a JSON Schema
 */
async function validateJsonFile(
  jsonPath: string,
  schemaPath: string
): Promise<ValidationResult> {
  const ajv = new Ajv({ allErrors: true, verbose: true });
  addFormats(ajv); // Add format validation (uri, date-time, etc.)

  // Read JSON file
  const jsonContent = await readFile(jsonPath, 'utf-8');
  const jsonData = JSON.parse(jsonContent);

  // Read schema file
  const schemaContent = await readFile(schemaPath, 'utf-8');
  const schema = JSON.parse(schemaContent);

  // Validate
  const validate = ajv.compile(schema);
  const valid = validate(jsonData);

  return {
    valid: Boolean(valid),
    errors: validate.errors || [],
  };
}

/**
 * Validate foundation.json against foundation.schema.json
 * Schema is bundled in src/schemas/
 */
export async function validateFoundationJson(
  jsonPath?: string,
  schemaPath?: string
): Promise<ValidationResult> {
  const foundationJsonPath = jsonPath || 'spec/foundation.json';
  // Schema is bundled with the application in src/schemas/
  const foundationSchemaPath = schemaPath || join(import.meta.dirname, '../schemas/foundation.schema.json');

  return validateJsonFile(foundationJsonPath, foundationSchemaPath);
}

/**
 * Validate tags.json against tags.schema.json
 * Schema is bundled in src/schemas/
 */
export async function validateTagsJson(
  jsonPath?: string,
  schemaPath?: string
): Promise<ValidationResult> {
  const tagsJsonPath = jsonPath || 'spec/tags.json';
  // Schema is bundled with the application in src/schemas/
  const tagsSchemaPath = schemaPath || join(import.meta.dirname, '../schemas/tags.schema.json');

  return validateJsonFile(tagsJsonPath, tagsSchemaPath);
}

/**
 * Validate all JSON files (foundation.json and tags.json)
 * Uses bundled schemas from src/schemas/
 */
export async function validateJson(cwd?: string): Promise<ValidationResults[]> {
  const baseDir = cwd || process.cwd();
  const results: ValidationResults[] = [];

  // Validate foundation.json (schema path is handled by validateFoundationJson)
  try {
    const foundationJsonPath = join(baseDir, 'spec', 'foundation.json');
    const foundationResult = await validateFoundationJson(foundationJsonPath);

    results.push({
      filePath: foundationJsonPath,
      ...foundationResult,
    });
  } catch (error: any) {
    // If file doesn't exist, skip
    if (error.code !== 'ENOENT') {
      throw error;
    }
  }

  // Validate tags.json (schema path is handled by validateTagsJson)
  try {
    const tagsJsonPath = join(baseDir, 'spec', 'tags.json');
    const tagsResult = await validateTagsJson(tagsJsonPath);

    results.push({
      filePath: tagsJsonPath,
      ...tagsResult,
    });
  } catch (error: any) {
    // If file doesn't exist, skip
    if (error.code !== 'ENOENT') {
      throw error;
    }
  }

  return results;
}

/**
 * Format validation errors for display
 */
export function formatValidationErrors(errors: ErrorObject[]): string[] {
  return errors.map((error) => {
    const path = error.instancePath || '/';
    const message = error.message || 'Unknown error';
    const params = error.params ? ` (${JSON.stringify(error.params)})` : '';

    return `Validation error at ${path}: ${message}${params}`;
  });
}

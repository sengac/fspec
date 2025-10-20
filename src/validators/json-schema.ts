import Ajv, { type ErrorObject } from 'ajv';
import addFormats from 'ajv-formats';
import { readFile } from 'fs/promises';
import foundationSchema from '../schemas/generic-foundation.schema.json';
import tagsSchema from '../schemas/tags.schema.json';

export interface ValidationResult {
  valid: boolean;
  errors: ErrorObject[];
}

/**
 * Validate a JSON object against a JSON Schema
 */
function validateJsonObject(
  data: unknown,
  schema: object
): ValidationResult {
  const ajv = new Ajv({ allErrors: true, verbose: true });
  addFormats(ajv); // Add format validation (uri, date-time, etc.)

  // Validate
  const validate = ajv.compile(schema);
  const valid = validate(data);

  return {
    valid: Boolean(valid),
    errors: validate.errors || [],
  };
}

/**
 * Validate a JSON file against a JSON Schema
 */
async function validateJsonFile(
  jsonPath: string,
  schema: object
): Promise<ValidationResult> {
  // Read JSON file
  const jsonContent = await readFile(jsonPath, 'utf-8');
  const jsonData = JSON.parse(jsonContent);

  // Validate using object validator
  return validateJsonObject(jsonData, schema);
}

/**
 * Validate a foundation.json object against generic-foundation.schema.json
 * Schema is bundled into the application at build time
 */
export function validateFoundationObject(data: unknown): ValidationResult {
  return validateJsonObject(data, foundationSchema);
}

/**
 * Validate a tags.json object against tags.schema.json
 * Schema is bundled into the application at build time
 */
export function validateTagsObject(data: unknown): ValidationResult {
  return validateJsonObject(data, tagsSchema);
}

/**
 * Validate foundation.json file against generic-foundation.schema.json
 * Schema is bundled into the application at build time
 */
export async function validateFoundationJson(
  jsonPath?: string
): Promise<ValidationResult> {
  const foundationJsonPath = jsonPath || 'spec/foundation.json';
  return validateJsonFile(foundationJsonPath, foundationSchema);
}

/**
 * Validate tags.json file against tags.schema.json
 * Schema is bundled into the application at build time
 */
export async function validateTagsJson(
  jsonPath?: string
): Promise<ValidationResult> {
  const tagsJsonPath = jsonPath || 'spec/tags.json';
  return validateJsonFile(tagsJsonPath, tagsSchema);
}

/**
 * Format validation errors for display
 */
export function formatValidationErrors(errors: ErrorObject[]): string[] {
  return errors.map(error => {
    const path = error.instancePath || '/';
    const message = error.message || 'Unknown error';
    const params = error.params ? ` (${JSON.stringify(error.params)})` : '';

    return `Validation error at ${path}: ${message}${params}`;
  });
}

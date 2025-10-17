import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import { readFile } from 'fs/promises';
import type { Foundation } from '../types/foundation';
import type { Tags } from '../types/tags';
import foundationSchema from '../schemas/generic-foundation.schema.json';
import tagsSchema from '../schemas/tags.schema.json';

interface ValidationError {
  path: string;
  message: string;
}

interface ValidationResult {
  valid: boolean;
  errors?: ValidationError[];
  data?: unknown;
}

// Initialize Ajv with formats support
const ajv = new Ajv({ allErrors: true });
addFormats(ajv);

// Compile schemas
const validateFoundation = ajv.compile(foundationSchema);
const validateTags = ajv.compile(tagsSchema);

export function validateFoundationJson(data: Foundation): ValidationResult {
  const valid = validateFoundation(data);

  if (!valid && validateFoundation.errors) {
    return {
      valid: false,
      errors: formatValidationErrors(validateFoundation.errors),
    };
  }

  return { valid: true };
}

export function validateTagsJson(data: Tags): ValidationResult {
  const valid = validateTags(data);

  if (!valid && validateTags.errors) {
    return {
      valid: false,
      errors: formatValidationErrors(validateTags.errors),
    };
  }

  return { valid: true };
}

export async function validateFromFile(
  filePath: string,
  schemaType: 'foundation' | 'tags'
): Promise<ValidationResult> {
  try {
    const content = await readFile(filePath, 'utf-8');
    const data = JSON.parse(content);

    const result =
      schemaType === 'foundation'
        ? validateFoundationJson(data)
        : validateTagsJson(data);

    if (result.valid) {
      return { ...result, data };
    }

    return result;
  } catch (error: unknown) {
    if (error instanceof SyntaxError) {
      throw new Error(`JSON parsing failed: ${error.message}`);
    }
    throw error;
  }
}

export function formatValidationErrors(
  ajvErrors: Array<{
    instancePath: string;
    schemaPath: string;
    keyword: string;
    params: Record<string, unknown>;
    message?: string;
  }>
): ValidationError[] {
  return ajvErrors.map(error => ({
    path: error.instancePath || '/',
    message: error.message || 'Validation error',
  }));
}

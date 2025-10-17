/**
 * Generic Foundation Schema Validator
 *
 * Feature: spec/features/design-generic-foundation-schema.feature
 *
 * Validates generic foundation documents using Ajv with ajv-formats.
 * Supports ANY project type (web apps, CLI tools, libraries, services, mobile apps).
 * Focuses ONLY on WHY (problem) and WHAT (solution), never HOW (implementation).
 */

import Ajv, { type ErrorObject } from 'ajv';
import addFormats from 'ajv-formats';
import { readFile } from 'fs/promises';
import genericFoundationSchema from '../schemas/generic-foundation.schema.json';
import type { GenericFoundation } from '../types/generic-foundation';

export interface GenericFoundationValidationResult {
  valid: boolean;
  errors: ErrorObject[];
  data?: GenericFoundation;
}

/**
 * Validate a generic foundation document against the schema
 * Uses Ajv with ajv-formats for uri, email, date-time validation
 */
export async function validateGenericFoundation(
  jsonPath: string
): Promise<GenericFoundationValidationResult> {
  // Create Ajv instance with all errors and verbose mode
  const ajv = new Ajv({ allErrors: true, verbose: true });
  addFormats(ajv); // Add format validation (uri, email, date-time, etc.)

  // Compile schema
  const validate = ajv.compile(genericFoundationSchema);

  // Read and parse JSON file
  const jsonContent = await readFile(jsonPath, 'utf-8');
  const jsonData = JSON.parse(jsonContent);

  // Validate
  const valid = validate(jsonData);

  return {
    valid: Boolean(valid),
    errors: validate.errors || [],
    data: valid ? (jsonData as GenericFoundation) : undefined,
  };
}

/**
 * Validate a generic foundation object (in-memory)
 * Useful for testing and runtime validation
 */
export function validateGenericFoundationObject(
  data: unknown
): GenericFoundationValidationResult {
  const ajv = new Ajv({ allErrors: true, verbose: true });
  addFormats(ajv);

  const validate = ajv.compile(genericFoundationSchema);
  const valid = validate(data);

  return {
    valid: Boolean(valid),
    errors: validate.errors || [],
    data: valid ? (data as GenericFoundation) : undefined,
  };
}

/**
 * Format validation errors for display
 * Provides clear, actionable error messages with field path and reason
 */
export function formatGenericFoundationErrors(errors: ErrorObject[]): string[] {
  return errors.map(error => {
    const path = error.instancePath || '/';
    const message = error.message || 'Unknown error';
    const params = error.params ? ` (${JSON.stringify(error.params)})` : '';

    return `Validation error at ${path}: ${message}${params}`;
  });
}

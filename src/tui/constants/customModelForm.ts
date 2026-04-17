/**
 * Constants for custom model form in the Model Selector
 *
 * MODEL-004: Defines form field definitions for adding/editing custom models.
 */

import type { CustomModelDefinition } from '../../utils/provider-config';

/**
 * Field type for custom model form (text, number, select, boolean)
 */
export type CustomModelFieldType = 'text' | 'number' | 'select' | 'boolean';

/**
 * Custom model form field definition
 */
export interface CustomModelFormField {
  /** Key in CustomModelDefinition */
  key: keyof CustomModelDefinition;
  /** Human-readable label */
  label: string;
  /** Field input type */
  fieldType: CustomModelFieldType;
  /** Whether the field is required */
  required: boolean;
  /** Placeholder text */
  placeholder?: string;
  /** Allowed values for select fields */
  options?: string[];
}

/**
 * Custom model form fields in display order.
 *
 * These fields match the CustomModelDefinition interface:
 * id, displayName, facade, contextWindow, maxOutputTokens, reasoning, hasVision
 */
export const CUSTOM_MODEL_FORM_FIELDS: CustomModelFormField[] = [
  {
    key: 'id',
    label: 'Model ID',
    fieldType: 'text',
    required: true,
    placeholder: 'e.g., meta-llama/Meta-Llama-3.1-405B',
  },
  {
    key: 'displayName',
    label: 'Display Name',
    fieldType: 'text',
    required: false,
    placeholder: 'e.g., Llama 3.1 405B',
  },
  {
    key: 'facade',
    label: 'Facade',
    fieldType: 'select',
    required: false,
    placeholder: '(default: openai)',
    options: ['openai', 'codex', 'claude', 'gemini', 'zai'],
  },
  {
    key: 'contextWindow',
    label: 'Context Window',
    fieldType: 'number',
    required: false,
    placeholder: '128000',
  },
  {
    key: 'maxOutputTokens',
    label: 'Max Output Tokens',
    fieldType: 'number',
    required: false,
    placeholder: '16384',
  },
  {
    key: 'compactionThreshold',
    label: 'Compaction Trigger',
    fieldType: 'text',
    required: false,
    placeholder: '80% or 200000',
  },
  {
    key: 'reasoning',
    label: 'Reasoning',
    fieldType: 'boolean',
    required: false,
    placeholder: 'false',
  },
  {
    key: 'hasVision',
    label: 'Vision',
    fieldType: 'boolean',
    required: false,
    placeholder: 'false',
  },
];

/**
 * Create empty custom model form values
 */
export function createEmptyCustomModelValues(): Partial<CustomModelDefinition> {
  return {};
}

/**
 * Create pre-filled custom model form values from an existing definition
 */
export function prefillCustomModelValues(
  definition: CustomModelDefinition
): Partial<CustomModelDefinition> {
  return { ...definition };
}

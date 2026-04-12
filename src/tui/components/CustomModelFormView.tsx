/**
 * CustomModelFormView - Form for adding/editing custom models
 *
 * MODEL-004: Renders the custom model form overlay within the Model Selector.
 * This is a purely presentational component — all state comes from props.
 *
 * Follows the same pattern as the profile form in ProviderSettingsPanel.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { CustomModelDefinition } from '../../utils/provider-config';
import {
  CUSTOM_MODEL_FORM_FIELDS,
  type CustomModelFormField,
} from '../constants/customModelForm';

export interface CustomModelFormViewProps {
  /** Form title ('Add Custom Model' or 'Edit Custom Model') */
  title: string;
  /** Profile name this model belongs to */
  profileName: string;
  /** Current form field values */
  values: Partial<CustomModelDefinition>;
  /** Currently focused field index */
  fieldIndex: number;
  /** Terminal width */
  width: number;
}

/**
 * Get display value for a form field
 */
function getFieldDisplayValue(
  field: CustomModelFormField,
  values: Partial<CustomModelDefinition>
): string {
  const raw = values[field.key];

  if (raw === undefined || raw === null) {
    return '';
  }

  if (field.fieldType === 'boolean') {
    return raw ? 'true' : 'false';
  }

  return String(raw);
}

/**
 * CustomModelFormView Component
 */
export function CustomModelFormView({
  title,
  profileName,
  values,
  fieldIndex,
  width,
}: CustomModelFormViewProps): React.ReactElement {
  const contentWidth = width - 8;

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      {/* Form header */}
      <Box marginBottom={1}>
        <Text bold color="cyan">
          {title}
        </Text>
        <Text dimColor> — profile: {profileName}</Text>
      </Box>

      {/* Form fields */}
      {CUSTOM_MODEL_FORM_FIELDS.map((field, idx) => {
        const isActive = idx === fieldIndex;
        const displayValue = getFieldDisplayValue(field, values);

        return (
          <Box key={field.key} width={contentWidth}>
            <Text
              backgroundColor={isActive ? 'cyan' : undefined}
              color={isActive ? 'black' : 'white'}
            >
              {isActive ? '> ' : '  '}
              <Text bold={isActive}>{field.label}</Text>
              {field.required && (
                <Text color={isActive ? 'black' : 'red'}>*</Text>
              )}
              {': '}
              {displayValue ? (
                <Text>{displayValue}</Text>
              ) : (
                <Text dimColor={!isActive}>
                  {field.placeholder || ''}
                </Text>
              )}
              {isActive && <Text inverse> </Text>}
              {field.fieldType === 'select' && isActive && (
                <Text dimColor>
                  {' '}
                  (←/→ to cycle: {field.options?.join(', ')})
                </Text>
              )}
              {field.fieldType === 'boolean' && isActive && (
                <Text dimColor> (←/→ to toggle)</Text>
              )}
            </Text>
          </Box>
        );
      })}

      {/* Footer */}
      <Box marginTop={1}>
        <Text dimColor>
          ↑↓: navigate fields | ←→: cycle options | Enter: save | Esc: cancel
        </Text>
      </Box>
    </Box>
  );
}

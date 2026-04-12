/**
 * Custom model form input handler for Model Selector
 *
 * MODEL-004: Extracts custom model form keyboard handling from ModelSelectorScreen.
 * Handles input in add-custom-model, edit-custom-model, and delete-custom-model-confirm modes.
 *
 * Follows the same handler pattern as profileFormModeHandler.ts.
 */

import type { Key } from 'ink';
import type { UseModelSelectorStateReturn } from '../hooks/useModelSelectorState';
import { CUSTOM_MODEL_FORM_FIELDS } from '../constants/customModelForm';
import { filterPrintableChars } from '../utils/providerSettingsHelpers';

/**
 * Handle keyboard input in delete-custom-model-confirm mode.
 *
 * @returns true if input was handled (mode is active)
 */
export function handleDeleteConfirmInput(
  input: string,
  key: Key,
  state: UseModelSelectorStateReturn
): boolean {
  if (state.customModelMode.type !== 'delete-custom-model-confirm') {
    return false;
  }

  if (key.escape || input === 'n' || input === 'N') {
    state.setCustomModelMode({ type: 'browse' });
    return true;
  }

  if (key.return || input === 'y' || input === 'Y') {
    void state.deleteCustomModelConfirmed();
    return true;
  }

  return true;
}

/**
 * Handle keyboard input in add/edit custom model form mode.
 *
 * @returns true if input was handled (mode is active)
 */
export function handleCustomModelFormInput(
  input: string,
  key: Key,
  state: UseModelSelectorStateReturn
): boolean {
  const { customModelMode, customModelForm } = state;

  if (
    customModelMode.type !== 'add-custom-model' &&
    customModelMode.type !== 'edit-custom-model'
  ) {
    return false;
  }

  // Escape: cancel form
  if (key.escape) {
    state.setCustomModelMode({ type: 'browse' });
    return true;
  }

  // Enter: save form
  if (key.return) {
    void state.saveCustomModelForm();
    return true;
  }

  // Arrow down: next field
  if (key.downArrow) {
    state.setCustomModelFormFieldIndex(prev =>
      Math.min(prev + 1, CUSTOM_MODEL_FORM_FIELDS.length - 1)
    );
    return true;
  }

  // Arrow up: previous field
  if (key.upArrow) {
    state.setCustomModelFormFieldIndex(prev => Math.max(prev - 1, 0));
    return true;
  }

  const currentField = CUSTOM_MODEL_FORM_FIELDS[customModelForm.fieldIndex];

  // Left/Right arrows for select fields
  if (
    currentField.fieldType === 'select' &&
    (key.leftArrow || key.rightArrow)
  ) {
    const options = currentField.options || [];
    const currentVal = customModelForm.values[currentField.key] as
      | string
      | undefined;
    const currentIdx = currentVal ? options.indexOf(currentVal) : -1;
    let newIdx: number;
    if (key.rightArrow) {
      newIdx = currentIdx < options.length - 1 ? currentIdx + 1 : 0;
    } else {
      newIdx = currentIdx > 0 ? currentIdx - 1 : options.length - 1;
    }
    state.setCustomModelFormValues(prev => ({
      ...prev,
      [currentField.key]: options[newIdx],
    }));
    return true;
  }

  // Left/Right arrows for boolean fields
  if (
    currentField.fieldType === 'boolean' &&
    (key.leftArrow || key.rightArrow)
  ) {
    const currentVal = customModelForm.values[currentField.key] as
      | boolean
      | undefined;
    state.setCustomModelFormValues(prev => ({
      ...prev,
      [currentField.key]: !currentVal,
    }));
    return true;
  }

  // Backspace: remove last character from text/number fields
  if (key.backspace || key.delete) {
    if (
      currentField.fieldType === 'text' ||
      currentField.fieldType === 'number'
    ) {
      state.setCustomModelFormValues(prev => {
        const current = String(prev[currentField.key] || '');
        const newVal = current.slice(0, -1);
        if (currentField.fieldType === 'number') {
          const num = parseInt(newVal, 10);
          return { ...prev, [currentField.key]: isNaN(num) ? undefined : num };
        }
        return { ...prev, [currentField.key]: newVal || undefined };
      });
    }
    return true;
  }

  // Text input for text/number fields
  if (
    currentField.fieldType === 'text' ||
    currentField.fieldType === 'number'
  ) {
    const cleanInput = filterPrintableChars(input);
    if (cleanInput) {
      state.setCustomModelFormValues(prev => {
        const current = String(prev[currentField.key] || '');
        const newVal = current + cleanInput;
        if (currentField.fieldType === 'number') {
          const num = parseInt(newVal, 10);
          return { ...prev, [currentField.key]: isNaN(num) ? undefined : num };
        }
        return { ...prev, [currentField.key]: newVal };
      });
    }
  }

  return true;
}

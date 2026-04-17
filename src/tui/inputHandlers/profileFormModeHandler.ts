/**
 * Profile form mode input handler
 *
 * TUI-074: Handles keyboard input in profile create/edit mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type { HookMode } from '../types/settingsMode';
import { PROFILE_FORM_FIELDS } from '../constants/providerSettings';
import { filterPrintableChars } from '../utils/providerSettingsHelpers';
import {
  parseCompactionThreshold,
  formatCompactionThreshold,
} from '../utils/compactionThresholdParser';

/**
 * Handles input in profile form mode (create/edit)
 * @returns true if input was handled (mode is active)
 */
export function handleProfileFormMode(
  mode: HookMode,
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  if (mode.type !== 'create-profile' && mode.type !== 'edit-profile') {
    return false;
  }

  const isNewProfile = mode.type === 'create-profile';

  if (key.escape) {
    providerSettings.setMode({ type: 'list' });
    return true;
  }

  // TUI-084: Use arrow keys for field navigation, NOT Tab
  if (key.downArrow) {
    handleArrowDown(providerSettings, isNewProfile);
    return true;
  }

  if (key.upArrow) {
    handleArrowUp(providerSettings, isNewProfile);
    return true;
  }

  // Tab key is intentionally ignored in profile form mode (TUI-084)
  if (key.tab) {
    return true;
  }

  if (key.return && !key.shift) {
    handleSave(mode, providerSettings);
    return true;
  }

  if (key.backspace || key.delete) {
    handleBackspace(providerSettings);
    return true;
  }

  const cleanInput = filterPrintableChars(input);
  if (cleanInput) {
    handleCharInput(cleanInput, providerSettings);
  }
  return true;
}

/**
 * TUI-084: Handle Down arrow key to move to next field
 */
function handleArrowDown(
  providerSettings: UseProviderSettingsStateReturn,
  isNewProfile: boolean
): void {
  if (providerSettings.isEditingName) {
    // Exit name editing and go to first field
    providerSettings.setIsEditingName(false);
    providerSettings.setFormFieldIndex(0);
  } else if (providerSettings.formFieldIndex < PROFILE_FORM_FIELDS.length - 1) {
    providerSettings.setFormFieldIndex(prev => prev + 1);
  }
}

/**
 * TUI-084: Handle Up arrow key to move to previous field
 */
function handleArrowUp(
  providerSettings: UseProviderSettingsStateReturn,
  isNewProfile: boolean
): void {
  if (providerSettings.isEditingName) {
    // Already at the top, do nothing
    return;
  } else if (providerSettings.formFieldIndex > 0) {
    providerSettings.setFormFieldIndex(prev => prev - 1);
  } else if (isNewProfile) {
    // At first field, go back to name editing
    providerSettings.setIsEditingName(true);
  }
}

function handleSave(
  mode: HookMode & { type: 'create-profile' | 'edit-profile' },
  providerSettings: UseProviderSettingsStateReturn
): void {
  const values = providerSettings.formValues;
  const name = providerSettings.profileName.trim();
  if (values.baseUrl && values.apiKey && name) {
    const config = {
      baseUrl: values.baseUrl,
      apiKey: values.apiKey,
      ...(values.contextWindow && { contextWindow: values.contextWindow }),
      ...(values.maxOutputTokens && {
        maxOutputTokens: values.maxOutputTokens,
      }),
      ...(values.compactionThreshold && {
        compactionThreshold: values.compactionThreshold,
      }),
    };
    void providerSettings
      .saveProfileConfig(mode.providerId, name, config)
      .then(() => {
        providerSettings.setMode({ type: 'list' });
      });
  }
}

function handleBackspace(
  providerSettings: UseProviderSettingsStateReturn
): void {
  if (providerSettings.isEditingName) {
    providerSettings.setProfileName(prev => prev.slice(0, -1));
  } else {
    const field = PROFILE_FORM_FIELDS[providerSettings.formFieldIndex];
    providerSettings.setFormValues(prev => {
      const current = String(prev[field] || '');
      return { ...prev, [field]: current.slice(0, -1) };
    });
  }
}

function handleCharInput(
  cleanInput: string,
  providerSettings: UseProviderSettingsStateReturn
): void {
  if (providerSettings.isEditingName) {
    providerSettings.setProfileName(prev => prev + cleanInput);
  } else {
    const field = PROFILE_FORM_FIELDS[providerSettings.formFieldIndex];
    providerSettings.setFormValues(prev => {
      const current = String(prev[field] || '');
      const newValue = current + cleanInput;
      if (field === 'contextWindow' || field === 'maxOutputTokens') {
        const num = parseInt(newValue, 10);
        return { ...prev, [field]: isNaN(num) ? undefined : num };
      }
      if (field === 'compactionThreshold') {
        // CTX-008: Store raw string during editing, parse on save
        // parseCompactionThreshold handles "80%", "200000", etc.
        const parsed = parseCompactionThreshold(newValue);
        return { ...prev, [field]: parsed };
      }
      return { ...prev, [field]: newValue };
    });
  }
}

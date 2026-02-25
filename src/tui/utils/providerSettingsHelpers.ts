/**
 * Helper functions for provider settings
 *
 * TUI-074: Utility functions for provider settings screen
 */

import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type { ProfileConfig } from '../../utils/provider-config';
import { DEFAULT_PROFILE_BASE_URL } from '../constants/providerSettings';

/**
 * Filters input to only printable ASCII characters (32-126).
 * Used for text input fields to prevent control characters.
 */
export function filterPrintableChars(input: string): string {
  return input
    .split('')
    .filter(ch => {
      const code = ch.charCodeAt(0);
      return code >= 32 && code <= 126;
    })
    .join('');
}

/**
 * Initialize state for creating a new profile
 */
export function initializeNewProfile(
  providerSettings: UseProviderSettingsStateReturn,
  providerId: string
): void {
  providerSettings.setFormValues({
    baseUrl: DEFAULT_PROFILE_BASE_URL,
    apiKey: '',
  });
  providerSettings.setProfileName('');
  providerSettings.setFormFieldIndex(0);
  providerSettings.setIsEditingName(true);
  providerSettings.setMode({
    type: 'create-profile',
    providerId,
  });
}

/**
 * Initialize state for editing an existing profile
 */
export function initializeEditProfile(
  providerSettings: UseProviderSettingsStateReturn,
  providerId: string,
  profileName: string,
  config: ProfileConfig
): void {
  providerSettings.setFormValues({ ...config });
  providerSettings.setProfileName(profileName);
  providerSettings.setFormFieldIndex(0);
  providerSettings.setIsEditingName(false);
  providerSettings.setMode({
    type: 'edit-profile',
    providerId,
    profileName,
  });
}

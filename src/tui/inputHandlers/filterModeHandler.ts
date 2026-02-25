/**
 * Filter mode input handler
 *
 * TUI-074: Handles keyboard input in filter mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import { filterPrintableChars } from '../utils/providerSettingsHelpers';

/**
 * Handles input in filter mode
 * @returns true if input was handled (mode is active)
 */
export function handleFilterMode(
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  if (!providerSettings.isFilterMode) {
    return false;
  }

  if (key.escape) {
    providerSettings.setIsFilterMode(false);
    providerSettings.setFilter('');
    return true;
  }

  if (key.return) {
    providerSettings.setIsFilterMode(false);
    return true;
  }

  if (key.backspace || key.delete) {
    providerSettings.setFilter(providerSettings.filter.slice(0, -1));
    return true;
  }

  const cleanFilter = filterPrintableChars(input);
  if (cleanFilter) {
    providerSettings.setFilter(providerSettings.filter + cleanFilter);
  }
  return true;
}

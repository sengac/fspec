/**
 * Constants for provider settings components
 *
 * TUI-074: Shared constants for provider settings screen
 */

import type { ProfileConfig } from '../../utils/provider-config';

/**
 * Profile form field keys in display order
 */
export const PROFILE_FORM_FIELDS: Array<keyof ProfileConfig> = [
  'baseUrl',
  'apiKey',
  'contextWindow',
  'maxOutputTokens',
];

/**
 * Default base URL for new profiles
 */
export const DEFAULT_PROFILE_BASE_URL = 'http://localhost:8888';

/**
 * Header/footer height offset for visible area calculation
 */
export const SETTINGS_PANEL_CHROME_HEIGHT = 6;

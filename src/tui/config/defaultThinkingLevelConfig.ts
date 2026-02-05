/**
 * TUI-058: Default Thinking Level Configuration
 *
 * Manages persistence of the default thinking level for new sessions.
 * Follows the same pattern as TUI-035 (lastUsedModel persistence).
 *
 * Config path: tui.defaultThinkingLevel
 */

import { loadConfig, writeConfig } from '../../utils/config';
import type { JsThinkingLevel } from '../../utils/thinkingLevel';

/**
 * Load the default thinking level from user config.
 *
 * @returns The persisted default level, or null if not configured
 */
export async function loadDefaultThinkingLevel(): Promise<JsThinkingLevel | null> {
  try {
    const config = await loadConfig();
    const level = config?.tui?.defaultThinkingLevel;

    // Validate that it's a valid JsThinkingLevel (0-3)
    if (typeof level === 'number' && level >= 0 && level <= 3) {
      return level as JsThinkingLevel;
    }

    return null;
  } catch {
    // Config load failed - return null (will use Off)
    return null;
  }
}

/**
 * Save the default thinking level to user config.
 *
 * @param level - The thinking level to set as default
 */
export async function saveDefaultThinkingLevel(
  level: JsThinkingLevel
): Promise<void> {
  try {
    const existingConfig = await loadConfig();
    const updatedConfig = {
      ...existingConfig,
      tui: {
        ...existingConfig?.tui,
        defaultThinkingLevel: level,
      },
    };
    await writeConfig('user', updatedConfig);
  } catch {
    // Silently fail - user can try again
    // This matches the pattern used in lastUsedModel persistence
  }
}

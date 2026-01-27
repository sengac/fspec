/**
 * Tool Integration Utilities
 *
 * Wrapper functions for calling Rust tools from TUI components.
 */

import { globSearch } from '@sengac/codelet-napi';

export interface GlobResult {
  success: boolean;
  data?: string;
  error?: string;
}

/**
 * Call the Glob tool to search for files matching a pattern
 */
export async function callGlobTool(
  pattern: string,
  path?: string,
  caseInsensitive?: boolean
): Promise<GlobResult> {
  try {
    return await globSearch(pattern, path, caseInsensitive);
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

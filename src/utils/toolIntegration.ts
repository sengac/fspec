/**
 * Tool Integration Utilities
 *
 * Wrapper functions for calling tools from TUI components.
 * This provides a clean interface for file search and other tool operations.
 *
 * Work Unit: TUI-055
 */

import { globSearch } from '@sengac/codelet-napi';

export interface GlobResult {
  success: boolean;
  data?: string;
  error?: string;
}

/**
 * Call the Glob tool to search for files matching a pattern
 * @param pattern Glob pattern to search for (e.g., "**\/src*")
 * @returns Promise with file results
 */
export async function callGlobTool(pattern: string): Promise<GlobResult> {
  try {
    return await globSearch(pattern);
  } catch (error) {
    console.error('callGlobTool error:', error);
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

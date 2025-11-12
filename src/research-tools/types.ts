/**
 * Research Tool Plugin System Types
 *
 * Standard interface for research tools (both bundled and custom).
 * Tools implement this interface to integrate with fspec research command.
 */

import type { ResearchToolHelpConfig } from '../utils/help-formatter';

/**
 * Research tool interface
 * All tools (bundled and custom) must implement this interface
 */
export interface ResearchTool {
  /** Tool name (e.g., 'ast', 'perplexity', 'custom') */
  name: string;

  /** Brief description of what the tool does */
  description: string;

  /**
   * Execute the research tool with provided arguments
   * @param args - Command-line arguments passed to the tool
   * @returns Promise resolving to tool output (typically JSON or text)
   */
  execute(args: string[]): Promise<string>;

  /**
   * Get help configuration for this tool
   * Returns structured configuration for standardized help display
   * @returns ResearchToolHelpConfig object
   */
  getHelpConfig(): ResearchToolHelpConfig;
}

/**
 * Tool module export format
 * Custom tools must export their tool as default or named 'tool'
 */
export interface ResearchToolModule {
  default?: ResearchTool;
  tool?: ResearchTool;
}

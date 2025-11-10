/**
 * Research Tool Registry
 *
 * Manages discovery and loading of research tools.
 * Supports both bundled tools (src/research-tools/) and custom tools (spec/research-tools/).
 */

import { join } from 'path';
import { pathToFileURL } from 'url';
import type { ResearchTool, ResearchToolModule } from './types';

// Import bundled tools
import astTool from './ast';

/**
 * Bundled tools registry
 * Core tools that ship with fspec
 */
const BUNDLED_TOOLS: Map<string, ResearchTool> = new Map([
  ['ast', astTool],
  // TODO: Add other bundled tools (perplexity, jira, confluence, stakeholder)
]);

/**
 * Get a research tool by name
 * Checks bundled tools first, then attempts dynamic loading from spec/research-tools/
 *
 * @param toolName - Name of the tool to load
 * @param cwd - Current working directory (for custom tools)
 * @returns Promise resolving to ResearchTool instance
 * @throws Error if tool not found or fails to load
 */
export async function getResearchTool(
  toolName: string,
  cwd: string = process.cwd()
): Promise<ResearchTool> {
  // Check bundled tools first
  const bundledTool = BUNDLED_TOOLS.get(toolName);
  if (bundledTool) {
    return bundledTool;
  }

  // Try loading custom tool from spec/research-tools/<toolName>.js
  try {
    const customToolPath = join(
      cwd,
      'spec',
      'research-tools',
      `${toolName}.js`
    );
    const toolUrl = pathToFileURL(customToolPath).href;

    // Dynamic import with cache busting
    const module = (await import(
      `${toolUrl}?t=${Date.now()}`
    )) as ResearchToolModule;

    // Extract tool from module (support both default export and named 'tool' export)
    const tool = module.default || module.tool;

    if (!tool) {
      throw new Error(
        `Custom tool "${toolName}" must export either default or named 'tool' with ResearchTool interface`
      );
    }

    // Validate tool implements interface
    if (!tool.name || !tool.description || !tool.execute || !tool.help) {
      throw new Error(
        `Custom tool "${toolName}" does not implement ResearchTool interface correctly`
      );
    }

    return tool;
  } catch (error: any) {
    if (error.code === 'ERR_MODULE_NOT_FOUND' || error.code === 'ENOENT') {
      throw new Error(
        `Research tool not found: ${toolName}\n\n` +
          `Available bundled tools: ${Array.from(BUNDLED_TOOLS.keys()).join(', ')}\n` +
          `To use custom tools: create spec/research-tools/${toolName}.ts and run 'fspec build-tool ${toolName}'`
      );
    }
    throw error;
  }
}

/**
 * List all available research tools
 * Returns both bundled and discovered custom tools
 *
 * @param cwd - Current working directory
 * @returns Array of tool names
 */
export function listAvailableTools(cwd: string = process.cwd()): string[] {
  // For now, just return bundled tools
  // TODO: Scan spec/research-tools/ for custom .js files
  return Array.from(BUNDLED_TOOLS.keys());
}

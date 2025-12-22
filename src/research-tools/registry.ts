/**
 * Research Tool Registry
 *
 * Manages discovery and loading of research tools.
 * Supports both bundled tools (src/research-tools/) and custom tools (spec/research-tools/).
 */

import { join } from 'path';
import { pathToFileURL } from 'url';
import type { ResearchTool, ResearchToolModule } from './types';
import { getFspecUserDir } from '../utils/config';

// Import bundled tools
import astTool from './ast';
import perplexityTool from './perplexity';
import jiraTool from './jira';
import confluenceTool from './confluence';
import stakeholderTool from './stakeholder';

/**
 * Bundled tools registry
 * Core tools that ship with fspec
 */
const BUNDLED_TOOLS: Map<string, ResearchTool> = new Map([
  ['ast', astTool],
  ['perplexity', perplexityTool],
  ['jira', jiraTool],
  ['confluence', confluenceTool],
  ['stakeholder', stakeholderTool],
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
    if (
      !tool.name ||
      !tool.description ||
      !tool.execute ||
      !tool.getHelpConfig
    ) {
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
 * Configuration status for a research tool
 */
export interface ToolConfigStatus {
  configured: boolean;
  reason: string;
  requiredFields: string[];
  configExample?: string;
}

/**
 * Get configuration status for all bundled research tools
 * Uses full validation to accurately determine if tools are configured
 *
 * @param cwd - Current working directory
 * @returns Promise resolving to Map of tool name to configuration status
 */
export async function getToolConfigurationStatus(
  cwd: string = process.cwd()
): Promise<Map<string, ToolConfigStatus>> {
  const statusMap = new Map<string, ToolConfigStatus>();

  // AST tool requires no configuration
  statusMap.set('ast', {
    configured: true,
    reason: 'No configuration required',
    requiredFields: [],
  });

  // Try to load configuration
  const config = await loadConfigIfExists(cwd);

  // Check each tool's configuration
  const otherTools = ['perplexity', 'jira', 'confluence', 'stakeholder'];
  for (const toolName of otherTools) {
    const requiredFields = getRequiredFields(toolName);
    const isConfigured = checkToolConfiguration(
      config,
      toolName,
      requiredFields
    );

    statusMap.set(toolName, {
      configured: isConfigured,
      reason: isConfigured ? 'Configured' : 'Configuration incomplete',
      requiredFields,
      configExample: isConfigured ? undefined : getConfigExample(toolName),
    });
  }

  return statusMap;
}

/**
 * Load configuration if it exists
 * @param cwd - Current working directory
 * @returns Configuration object or empty object
 */
async function loadConfigIfExists(cwd: string): Promise<any> {
  const fs = await import('fs/promises');
  const configPaths = [
    join(cwd, 'spec', 'fspec-config.json'),
    join(getFspecUserDir(), 'fspec-config.json'),
  ];

  for (const configPath of configPaths) {
    try {
      const content = await fs.readFile(configPath, 'utf-8');
      return JSON.parse(content);
    } catch {
      // File doesn't exist or can't be read, try next
      continue;
    }
  }

  return {};
}

/**
 * Check if a tool is properly configured
 * @param config - Configuration object
 * @param toolName - Name of the tool
 * @param requiredFields - Array of required field names
 * @returns true if all required fields exist with non-empty values
 */
function checkToolConfiguration(
  config: any,
  toolName: string,
  requiredFields: string[]
): boolean {
  if (!config.research || !config.research[toolName]) {
    return false;
  }

  const toolConfig = config.research[toolName];

  // Special case for stakeholder: needs at least one platform configured
  if (toolName === 'stakeholder') {
    const platforms = ['teams', 'slack', 'discord'];
    return platforms.some(platform => {
      const platformConfig = toolConfig[platform];
      if (!platformConfig) return false;

      // Check if platform has webhookUrl or token
      const hasWebhook =
        platformConfig.webhookUrl && platformConfig.webhookUrl.trim() !== '';
      const hasToken =
        platformConfig.token && platformConfig.token.trim() !== '';
      return hasWebhook || hasToken;
    });
  }

  // Check if all required fields exist and have non-empty values
  for (const field of requiredFields) {
    if (!toolConfig[field] || toolConfig[field].trim() === '') {
      return false;
    }
  }

  return true;
}

/**
 * Get required configuration fields for a tool
 *
 * @param toolName - Name of the tool
 * @returns Array of required field names
 */
function getRequiredFields(toolName: string): string[] {
  const requirements: Record<string, string[]> = {
    perplexity: ['apiKey'],
    jira: ['jiraUrl', 'username', 'apiToken'],
    confluence: ['confluenceUrl', 'username', 'apiToken'],
    stakeholder: [],
  };

  return requirements[toolName] || [];
}

/**
 * Get JSON configuration example for a tool
 *
 * @param toolName - Name of the tool
 * @returns JSON string with configuration example
 */
export function getConfigExample(toolName: string): string {
  const examples: Record<string, any> = {
    perplexity: {
      research: {
        perplexity: {
          apiKey: 'pplx-your-api-key-here',
        },
      },
    },
    jira: {
      research: {
        jira: {
          jiraUrl: 'https://example.atlassian.net',
          username: 'your-email@example.com',
          apiToken: 'your-api-token',
        },
      },
    },
    confluence: {
      research: {
        confluence: {
          confluenceUrl: 'https://example.atlassian.net/wiki',
          username: 'your-email',
          apiToken: 'your-token',
        },
      },
    },
    stakeholder: {
      research: {
        stakeholder: {
          teams: {
            webhookUrl: 'https://...',
          },
        },
      },
    },
  };

  return JSON.stringify(examples[toolName] || {}, null, 2);
}

/**
 * List all available research tools
 * Returns both bundled and discovered custom tools
 *
 * @param _cwd - Current working directory
 * @returns Array of tool names
 */
export function listAvailableTools(_cwd: string = process.cwd()): string[] {
  // For now, just return bundled tools
  // TODO: Scan spec/research-tools/ for custom .js files
  return Array.from(BUNDLED_TOOLS.keys());
}

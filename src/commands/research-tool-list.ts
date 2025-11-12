/**
 * Research tool discovery and status display (RES-010)
 *
 * Provides tool listing with configuration status, descriptions, and usage guidance.
 * Integrates with config resolution system from RES-012.
 */

import { resolveConfig } from '../utils/config-resolution';

export interface ToolInfo {
  name: string;
  description: string;
  status: 'CONFIGURED' | 'NOT CONFIGURED' | 'PARTIALLY CONFIGURED';
  statusIndicator: string;
  configured: boolean;
  configSource?: 'ENV' | 'USER' | 'PROJECT' | 'DEFAULT';
  configGuidance?: string;
}

export interface ToolHelp {
  description: string;
  configRequirements: string;
  usageExamples: string[];
}

/**
 * Tool registry with descriptions and required config fields
 */
export const TOOL_REGISTRY: Record<
  string,
  { description: string; required: string[] }
> = {
  perplexity: {
    description:
      'Perplexity AI research tool for web search and AI-powered answers',
    required: ['apiKey'],
  },
  jira: {
    description: 'Jira integration for querying issues and project data',
    required: ['url', 'token'],
  },
  confluence: {
    description:
      'Confluence integration for searching documentation and wiki pages',
    required: ['url', 'token'],
  },
  stakeholder: {
    description: 'Stakeholder communication tool for Teams/Slack/Discord',
    required: ['teamsWebhook'],
  },
  ast: {
    description:
      'AST code analysis tool for pattern detection and deep code analysis',
    required: [],
  },
};

/**
 * Environment variable mappings for configuration guidance
 */
const ENV_VAR_MAPPINGS: Record<string, Record<string, string>> = {
  perplexity: {
    apiKey: 'PERPLEXITY_API_KEY',
    model: 'PERPLEXITY_MODEL',
  },
  jira: {
    url: 'JIRA_URL',
    token: 'JIRA_TOKEN',
  },
  confluence: {
    url: 'CONFLUENCE_URL',
    token: 'CONFLUENCE_TOKEN',
  },
  stakeholder: {
    teamsWebhook: 'TEAMS_WEBHOOK_URL',
    slackWebhook: 'SLACK_WEBHOOK_URL',
  },
};

/**
 * List all research tools with configuration status
 * @param cwd - Current working directory for resolving project config
 * @param showAll - If true, show all tools; if false, show only configured tools
 * @param userConfigPath - Optional path to user config (for testing)
 */
export function listResearchTools(
  cwd?: string,
  showAll: boolean = false,
  userConfigPath?: string
): ToolInfo[] {
  const allTools: ToolInfo[] = [];

  // Prepare config options with project-specific path if cwd provided
  const configOptions: any = cwd
    ? { projectConfigPath: `${cwd}/spec/fspec-config.json` }
    : {};

  // Add user config path if provided (mainly for testing to avoid real user config)
  if (userConfigPath) {
    configOptions.userConfigPath = userConfigPath;
  }

  for (const [toolName, toolMeta] of Object.entries(TOOL_REGISTRY)) {
    const config = resolveConfig(toolName, configOptions);
    const toolInfo: ToolInfo = {
      name: toolName,
      description: toolMeta.description,
      status: 'NOT CONFIGURED',
      statusIndicator: '✗',
      configured: false,
    };

    // Check if tool is configured
    const hasRequiredFields = toolMeta.required.every(field => {
      return (
        config[field] !== undefined &&
        config[field] !== null &&
        config[field] !== ''
      );
    });

    if (hasRequiredFields) {
      toolInfo.status = 'CONFIGURED';
      toolInfo.statusIndicator = '✓';
      toolInfo.configured = true;
      toolInfo.configSource = config.source;
    } else {
      // Generate configuration guidance for unconfigured tools
      const envVars = ENV_VAR_MAPPINGS[toolName] || {};
      const envVarExamples: string[] = [];

      for (const [configKey, envVarName] of Object.entries(envVars)) {
        if (toolMeta.required.includes(configKey)) {
          envVarExamples.push(`export ${envVarName}="..."`);
        }
      }

      if (envVarExamples.length > 0) {
        toolInfo.configGuidance = envVarExamples.join('\n');
      }
    }

    allTools.push(toolInfo);
  }

  // Filter to only configured tools if showAll is false
  if (!showAll) {
    return allTools.filter(tool => tool.configured);
  }

  return allTools;
}

/**
 * Get tool-specific help with configuration requirements
 */
export function getToolHelp(toolName: string): ToolHelp {
  const toolMeta = TOOL_REGISTRY[toolName];

  if (!toolMeta) {
    throw new Error(`Unknown tool: ${toolName}`);
  }

  const envVars = ENV_VAR_MAPPINGS[toolName] || {};
  const requiredEnvVars = toolMeta.required
    .map(field => {
      const envVarName = Object.entries(envVars).find(
        ([key]) => key === field
      )?.[1];
      return envVarName || field;
    })
    .filter(Boolean);

  const configRequirements =
    requiredEnvVars.length > 0
      ? requiredEnvVars.join(', ')
      : 'No configuration required';

  const usageExamples = [
    `fspec research --tool=${toolName} --query="your question"`,
    `export ${requiredEnvVars[0] || 'CONFIG_VAR'}="your-value"`,
  ];

  return {
    description: `${toolName.charAt(0).toUpperCase() + toolName.slice(1)}: ${toolMeta.description}`,
    configRequirements,
    usageExamples,
  };
}

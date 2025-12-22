/**
 * Multi-layer configuration resolution system for research tools
 * RES-012 Phase 1: Config Resolution + Validation
 *
 * Priority order: ENV vars → User config → Project config → Defaults
 */

import fs from 'fs';
import path from 'path';
import { config as loadDotenv } from 'dotenv';
import { getFspecUserDir } from './config';

export type ConfigSource = 'ENV' | 'USER' | 'PROJECT' | 'DEFAULT';

export interface ResolvedConfig {
  [key: string]: any;
  source: ConfigSource;
}

export interface ConfigOptions {
  userConfigPath?: string;
  projectConfigPath?: string;
  envPath?: string;
}

/**
 * Default configuration values for each research tool
 */
const DEFAULTS: Record<string, any> = {
  perplexity: {
    model: 'sonar',
  },
  jira: {},
  confluence: {},
  stakeholder: {},
};

/**
 * Required fields for each research tool
 */
const REQUIRED_FIELDS: Record<string, string[]> = {
  perplexity: ['apiKey'],
  jira: ['url', 'token'],
  confluence: ['url', 'token'],
  stakeholder: [],
};

/**
 * Environment variable mappings for each tool
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
 * Load and merge configuration from multiple sources
 * Priority: ENV vars → User config → Project config → Defaults
 */
export function resolveConfig(
  toolName: string,
  options: ConfigOptions = {}
): ResolvedConfig {
  // Load .env file if specified
  if (options.envPath && fs.existsSync(options.envPath)) {
    loadDotenv({ path: options.envPath });
  }

  // Set default paths if not provided
  const userConfigPath =
    options.userConfigPath || path.join(getFspecUserDir(), 'fspec-config.json');
  const projectConfigPath =
    options.projectConfigPath ||
    path.join(process.cwd(), 'spec', 'fspec-config.json');

  const config: ResolvedConfig = {
    source: 'DEFAULT',
  };

  // Layer 4: Defaults (lowest priority)
  if (DEFAULTS[toolName]) {
    Object.assign(config, DEFAULTS[toolName]);
  }

  // Layer 3: Project config
  if (fs.existsSync(projectConfigPath)) {
    try {
      const projectConfig = JSON.parse(
        fs.readFileSync(projectConfigPath, 'utf-8')
      );
      if (projectConfig.research?.[toolName]) {
        Object.assign(config, projectConfig.research[toolName]);
        config.source = 'PROJECT';
      }
    } catch (error) {
      // Ignore invalid JSON
    }
  }

  // Layer 2: User config (higher priority)
  if (fs.existsSync(userConfigPath)) {
    try {
      const userConfig = JSON.parse(fs.readFileSync(userConfigPath, 'utf-8'));
      if (userConfig.research?.[toolName]) {
        Object.assign(config, userConfig.research[toolName]);
        config.source = 'USER';
      }
    } catch (error) {
      // Ignore invalid JSON
    }
  }

  // Layer 1: Environment variables (highest priority)
  const envMappings = ENV_VAR_MAPPINGS[toolName] || {};
  let hasEnvVar = false;

  for (const [configKey, envVarName] of Object.entries(envMappings)) {
    const envValue = process.env[envVarName];
    if (envValue !== undefined) {
      config[configKey] = envValue;
      hasEnvVar = true;
    }
  }

  if (hasEnvVar) {
    config.source = 'ENV';
  }

  return config;
}

/**
 * Validate configuration and throw error if required fields are missing
 */
export function validateConfig(
  toolName: string,
  options: ConfigOptions = {}
): void {
  const config = resolveConfig(toolName, options);
  const requiredFields = REQUIRED_FIELDS[toolName] || [];

  const missingFields: string[] = [];

  for (const field of requiredFields) {
    if (!config[field]) {
      // Check if there's an env var for this field
      const envMappings = ENV_VAR_MAPPINGS[toolName] || {};
      const envVarName = Object.entries(envMappings).find(
        ([key]) => key === field
      )?.[1];

      if (envVarName) {
        missingFields.push(envVarName);
      } else {
        missingFields.push(field);
      }
    }
  }

  if (missingFields.length > 0) {
    const errorMessage = `Missing required configuration: ${missingFields.join(', ')}

To configure ${toolName}, you can:
  1. Set environment variable(s): ${missingFields.map(f => `export ${f}="..."`).join(', ')}
  2. Add to ~/.fspec/fspec-config.json (user config)
  3. Add to spec/fspec-config.json (project config)

For more information, run: fspec research --tool=${toolName} --help`;

    throw new Error(errorMessage);
  }
}

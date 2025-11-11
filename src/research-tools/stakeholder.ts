/**
 * Stakeholder Communication Research Tool
 *
 * Sends questions to stakeholders via chat platforms during Example Mapping.
 * Bundled TypeScript implementation.
 */

import type { ResearchTool } from './types';
import fs from 'fs';
import path from 'path';
import os from 'os';

interface StakeholderConfig {
  teams?: { webhookUrl: string };
  slack?: { token: string; channel: string };
}

/**
 * Load Stakeholder configuration from ~/.fspec/fspec-config.json
 */
function _loadConfig(): StakeholderConfig {
  const configPath = path.join(os.homedir(), '.fspec', 'fspec-config.json');

  if (!fs.existsSync(configPath)) {
    throw new Error(
      'Config file not found at ~/.fspec/fspec-config.json\n' +
        'Create config with stakeholder credentials:\n' +
        '  mkdir -p ~/.fspec\n' +
        '  echo \'{"research":{"stakeholder":{"teams":{"webhookUrl":"..."}}}}\' > ~/.fspec/fspec-config.json'
    );
  }

  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

  if (!config.research?.stakeholder) {
    throw new Error(
      'Stakeholder configuration not found\n' +
        'Add to ~/.fspec/fspec-config.json:\n' +
        '  "research": { "stakeholder": { "teams": { "webhookUrl": "..." }, "slack": { "token": "...", "channel": "..." } } }'
    );
  }

  return config.research.stakeholder;
}

/**
 * Generate mock data for test mode
 */
function generateMockData(
  platform: string,
  question: string,
  workUnit: string | null
): string {
  let output = '';

  if (workUnit) {
    output += `# Stakeholder Question for ${workUnit}\n\n`;
    output += `**Platform:** ${platform}\n`;
    output += `**Question:** ${question}\n\n`;
    output += `**Work Unit Context:**\n`;
    output += `- ID: ${workUnit}\n`;
    output += `- Title: User Login\n`;
    output += `- Epic: user-management\n\n`;
    output += `**Rules:**\n`;
    output += `1. Password must be 8+ chars\n\n`;
    output += `**Examples:**\n`;
    output += `1. Valid login with email/password\n\n`;
    output += `**Previous Q&A:**\n`;
    output += `Q: Support 2FA? A: Yes, in phase 2\n\n`;
  } else {
    output += `# Stakeholder Question\n\n`;
    output += `**Platform:** ${platform}\n`;
    output += `**Question:** ${question}\n\n`;
  }

  output += `This is test mode output. In production, this message would be sent to:\n`;

  const platforms = platform.split(',');
  for (const p of platforms) {
    output += `- ${p.trim()}\n`;
  }

  output += `\nMessage sent successfully (test mode).\n`;
  output += `Stakeholders will be notified and can respond manually.\n`;

  return output;
}

export const tool: ResearchTool = {
  name: 'stakeholder',
  description:
    'Stakeholder communication tool for sending questions via Teams, Slack, etc.',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const platformIndex = args.indexOf('--platform');
    const questionIndex = args.indexOf('--question');
    const workUnitIndex = args.indexOf('--work-unit');

    if (platformIndex === -1 || questionIndex === -1) {
      throw new Error('--platform and --question are required');
    }

    const platform = args[platformIndex + 1];
    const question = args[questionIndex + 1];
    const workUnit = workUnitIndex >= 0 ? args[workUnitIndex + 1] : null;

    if (!platform || !question) {
      throw new Error('--platform and --question cannot be empty');
    }

    // Test mode: return mock data
    if (process.env.FSPEC_TEST_MODE === '1') {
      return generateMockData(platform, question, workUnit);
    }

    // Production mode would:
    // 1. Load config from ~/.fspec/fspec-config.json
    // 2. Auto-discover platform plugins from spec/research-scripts/plugins/
    // 3. Execute plugin(s) with question and context
    // 4. Exit without waiting for response (fire-and-forget)

    throw new Error(
      'Production mode not yet implemented\n' +
        'This tool currently only supports FSPEC_TEST_MODE=1 for testing'
    );
  },

  getHelpConfig() {
    return {
      name: 'stakeholder',
      description:
        'Send questions to stakeholders via chat platforms during Example Mapping',
      usage:
        'fspec research --tool=stakeholder --platform <platform> --question <question> [options]',
      whenToUse:
        'Use during Example Mapping to ask questions to product owners, business stakeholders, or domain experts via Teams, Slack, or Discord.',
      options: [
        {
          flag: '--platform <platform>',
          description: 'Platform to send to: teams, slack, discord (required)',
        },
        {
          flag: '--question <question>',
          description: 'Question to send to stakeholders (required)',
        },
        {
          flag: '--work-unit <id>',
          description: 'Work unit ID for context (optional)',
        },
      ],
      examples: [
        {
          command: '--platform=teams --question="Should we support OAuth2?"',
          description: 'Send question to Teams',
        },
        {
          command:
            '--platform=slack --question="OAuth support?" --work-unit=AUTH-001',
          description: 'Send with work unit context',
        },
      ],
      configuration: {
        required: true,
        location: '~/.fspec/fspec-config.json',
        example: JSON.stringify(
          {
            research: {
              stakeholder: {
                teams: { webhookUrl: '...' },
                slack: { token: '...', channel: '...' },
              },
            },
          },
          null,
          2
        ),
      },
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required arguments' },
        { code: 2, description: 'Configuration error' },
      ],
    };
  },
};

// Default export for compatibility
export default tool;

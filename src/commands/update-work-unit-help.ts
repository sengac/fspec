import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-work-unit',
  description: 'Update work unit metadata (title, description, epic, etc.)',
  usage: 'fspec update-work-unit <id> [options]',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      flag: '--title <title>',
      description: 'New title',
    },
    {
      flag: '--description <description>',
      description: 'New description',
    },
    {
      flag: '--epic <epic>',
      description: 'Associate with epic',
    },
  ],
  examples: [
    {
      command: 'fspec update-work-unit AUTH-001 --title "OAuth 2.0 integration"',
      description: 'Update title',
      output: '✓ Updated work unit AUTH-001',
    },
  ],
  relatedCommands: ['show-work-unit', 'update-work-unit-status'],
};

export default config;

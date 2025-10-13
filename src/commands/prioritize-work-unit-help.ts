import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'prioritize-work-unit',
  description: 'Set priority order for a work unit within its status column',
  usage: 'fspec prioritize-work-unit <id> <priority>',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'priority',
      description: 'Priority number (1=highest)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec prioritize-work-unit AUTH-001 1',
      description: 'Set as highest priority',
      output: '✓ Set AUTH-001 priority to 1',
    },
  ],
  relatedCommands: ['list-work-units', 'board'],
  notes: [
    'Lower numbers = higher priority',
    'Affects display order in board and list commands',
  ],
};

export default config;

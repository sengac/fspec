import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-work-units',
  description: 'Validate work unit data integrity and relationships',
  usage: 'fspec validate-work-units [options]',
  whenToUse:
    'Use to check for data integrity issues like missing dependencies, invalid status transitions, or orphaned work units.',
  options: [
    {
      flag: '--fix',
      description: 'Automatically fix issues where possible',
    },
  ],
  examples: [
    {
      command: 'fspec validate-work-units',
      description: 'Validate all work units',
      output: '✓ All work units valid\n  Checked 42 work units',
    },
  ],
  relatedCommands: ['repair-work-units', 'check'],
};

export default config;

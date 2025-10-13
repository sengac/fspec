import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'clear-dependencies',
  description: 'Remove all dependencies from a work unit',
  usage: 'fspec clear-dependencies <workUnitId> [options]',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation',
    },
  ],
  examples: [
    {
      command: 'fspec clear-dependencies AUTH-001',
      description: 'Clear all dependencies',
      output: '✓ Cleared all dependencies from AUTH-001',
    },
  ],
  relatedCommands: ['add-dependency', 'remove-dependency'],
  notes: [
    'Removes all relationship types',
    'Requires confirmation unless --force is used',
  ],
};

export default config;

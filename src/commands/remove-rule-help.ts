import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-rule',
  description: 'Remove a business rule from Example Mapping by index',
  usage: 'fspec remove-rule <workUnitId> <index>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description: 'Rule index (from show-work-unit)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec remove-rule AUTH-001 1',
      description: 'Remove rule at index 1',
      output: '✓ Removed rule from AUTH-001',
    },
  ],
  relatedCommands: ['add-rule', 'show-work-unit'],
};

export default config;

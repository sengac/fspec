import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-example',
  description: 'Remove an example from Example Mapping by index',
  usage: 'fspec remove-example <workUnitId> <index>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description: 'Example index (from show-work-unit)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec remove-example AUTH-001 2',
      description: 'Remove example at index 2',
      output: '✓ Removed example from AUTH-001',
    },
  ],
  relatedCommands: ['add-example', 'show-work-unit'],
};

export default config;

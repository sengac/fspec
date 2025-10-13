import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-dependency',
  description: 'Remove a dependency relationship between work units',
  usage: 'fspec remove-dependency <workUnitId> [dependsOnId] [options]',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'dependsOnId',
      description: 'Dependency to remove (optional if using flags)',
      required: false,
    },
  ],
  options: [
    {
      flag: '--blocks <targetId>',
      description: 'Remove blocks relationship',
    },
    {
      flag: '--blocked-by <targetId>',
      description: 'Remove blocked-by relationship',
    },
    {
      flag: '--depends-on <targetId>',
      description: 'Remove depends-on relationship',
    },
    {
      flag: '--relates-to <targetId>',
      description: 'Remove relates-to relationship',
    },
  ],
  examples: [
    {
      command: 'fspec remove-dependency AUTH-001 AUTH-002',
      description: 'Remove depends-on',
      output: '✓ Removed dependency',
    },
  ],
  relatedCommands: ['add-dependency', 'clear-dependencies'],
};

export default config;

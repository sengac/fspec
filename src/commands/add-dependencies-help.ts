import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-dependencies',
  description: 'Add multiple dependency relationships to a work unit at once',
  usage: 'fspec add-dependencies <workUnitId> [options]',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      flag: '--blocks <ids>',
      description: 'Comma-separated list of work units this blocks',
    },
    {
      flag: '--blocked-by <ids>',
      description: 'Comma-separated list of work units blocking this',
    },
    {
      flag: '--depends-on <ids>',
      description: 'Comma-separated list of dependencies',
    },
    {
      flag: '--relates-to <ids>',
      description: 'Comma-separated list of related work units',
    },
  ],
  examples: [
    {
      command: 'fspec add-dependencies AUTH-001 --depends-on AUTH-002,AUTH-003 --relates-to UI-001',
      description: 'Add multiple dependencies',
      output: '✓ Added 3 dependencies to AUTH-001',
    },
  ],
  relatedCommands: ['add-dependency', 'remove-dependency'],
};

export default config;

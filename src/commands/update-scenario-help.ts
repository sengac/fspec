import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-scenario',
  description: 'Update a scenario name in a feature file',
  usage: 'fspec update-scenario <file> <old-name> <new-name>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'old-name',
      description: 'Current scenario name',
      required: true,
    },
    {
      name: 'new-name',
      description: 'New scenario name',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec update-scenario spec/features/login.feature "Old name" "New name"',
      description: 'Rename scenario',
      output: '✓ Updated scenario name',
    },
  ],
  relatedCommands: ['add-scenario', 'delete-scenario'],
};

export default config;

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
      command:
        'fspec update-scenario spec/features/login.feature "Old name" "New name"',
      description: 'Rename scenario and preserve coverage mappings',
      output: '✓ Updated scenario name',
    },
  ],
  relatedCommands: ['add-scenario', 'delete-scenario'],
  notes: [
    'Automatically renames scenario in .feature.coverage file if it exists',
    'All test mappings and implementation mappings are preserved during rename',
    'Use this command instead of manual delete + create to preserve coverage links',
  ],
};

export default config;

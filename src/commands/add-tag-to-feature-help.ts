import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-tag-to-feature',
  description: 'Add one or more tags to a feature file (feature-level tags)',
  usage: 'fspec add-tag-to-feature <file> <tags...>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'tags...',
      description: 'One or more tags to add (e.g., @phase1 @critical)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-tag-to-feature spec/features/login.feature @phase1',
      description: 'Add single tag',
      output: '✓ Added tag @phase1 to spec/features/login.feature',
    },
    {
      command: 'fspec add-tag-to-feature spec/features/login.feature @phase1 @critical',
      description: 'Add multiple tags',
      output: '✓ Added tags @phase1, @critical to spec/features/login.feature',
    },
  ],
  relatedCommands: ['remove-tag-from-feature', 'list-feature-tags', 'add-tag-to-scenario'],
  notes: [
    'Tags must be registered first',
    'Duplicates are automatically ignored',
  ],
};

export default config;

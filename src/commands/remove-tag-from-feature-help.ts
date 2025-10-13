import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-tag-from-feature',
  description: 'Remove one or more tags from a feature file',
  usage: 'fspec remove-tag-from-feature <file> <tags...>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'tags...',
      description: 'One or more tags to remove',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec remove-tag-from-feature spec/features/login.feature @wip',
      description: 'Remove tag',
      output: '✓ Removed tag @wip from spec/features/login.feature',
    },
  ],
  relatedCommands: ['add-tag-to-feature', 'list-feature-tags'],
};

export default config;

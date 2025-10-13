import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-tag',
  description: 'Update a registered tag description or category',
  usage: 'fspec update-tag <tag> [options]',
  arguments: [
    {
      name: 'tag',
      description: 'Tag to update',
      required: true,
    },
  ],
  options: [
    {
      flag: '--description <description>',
      description: 'New description',
    },
    {
      flag: '--category <category>',
      description: 'New category',
    },
  ],
  examples: [
    {
      command: 'fspec update-tag @performance --description "High-performance features"',
      description: 'Update tag description',
      output: '✓ Updated tag @performance',
    },
  ],
  relatedCommands: ['register-tag', 'delete-tag', 'list-tags'],
};

export default config;

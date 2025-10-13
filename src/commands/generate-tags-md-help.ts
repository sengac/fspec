import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-tags-md',
  description: 'Generate spec/TAGS.md from tags.json',
  usage: 'fspec generate-tags-md',
  examples: [
    {
      command: 'fspec generate-tags-md',
      description: 'Generate human-readable tag documentation',
      output:
        '✓ Generated spec/TAGS.md from tags.json\n✓ Tags grouped by category',
    },
  ],
  relatedCommands: ['list-tags', 'register-tag', 'validate-tags'],
  notes: [
    'TAGS.md is human-readable documentation',
    'tags.json is the machine-readable source of truth',
    'Generated file includes all registered tags grouped by category',
  ],
};

export default config;

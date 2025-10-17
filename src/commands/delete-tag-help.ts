import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-tag',
  description: 'Delete a tag from the registry',
  usage: 'fspec delete-tag <tag> [options]',
  arguments: [
    {
      name: 'tag',
      description: 'Tag to delete',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation',
    },
    {
      flag: '--dry-run',
      description: 'Show what would be deleted without making changes',
    },
  ],
  examples: [
    {
      command: 'fspec delete-tag @deprecated',
      description: 'Delete tag',
      output: '✓ Deleted tag @deprecated',
    },
  ],
  relatedCommands: ['register-tag', 'list-tags'],
  notes: [
    'Tag must not be in use in any feature files',
    'Run validate-tags after deletion to verify',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-features',
  description: 'Delete all feature files that have a specific tag',
  usage: 'fspec delete-features <tag> [options]',
  arguments: [
    {
      name: 'tag',
      description: 'Tag to filter by (e.g., @deprecated)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation prompt',
    },
  ],
  examples: [
    {
      command: 'fspec delete-features @deprecated',
      description: 'Delete all features tagged with @deprecated',
      output:
        'Found 3 features with @deprecated:\n  - login.feature\n  - signup.feature\n  - profile.feature\nConfirm deletion? (y/n): y\n✓ Deleted 3 feature files',
    },
    {
      command: 'fspec delete-features @wip --force',
      description: 'Delete without confirmation',
      output: '✓ Deleted 5 feature files with @wip',
    },
  ],
  relatedCommands: ['delete-scenarios', 'list-features', 'validate-tags'],
  notes: [
    'DESTRUCTIVE operation - deletes entire feature files',
    'Use --force to skip confirmation (dangerous)',
    'Consider using retag instead if renaming tags',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-scenarios',
  description: 'Delete all scenarios that have a specific tag',
  usage: 'fspec delete-scenarios <tag> [options]',
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
      command: 'fspec delete-scenarios @deprecated',
      description: 'Delete all scenarios tagged with @deprecated',
      output:
        'Found 8 scenarios with @deprecated across 3 features:\n  - login.feature: 2 scenarios\n  - signup.feature: 3 scenarios\n  - profile.feature: 3 scenarios\nConfirm deletion? (y/n): y\n✓ Deleted 8 scenarios',
    },
    {
      command: 'fspec delete-scenarios @wip --force',
      description: 'Delete without confirmation',
      output: '✓ Deleted 12 scenarios with @wip',
    },
  ],
  relatedCommands: ['delete-features', 'delete-scenario', 'list-scenario-tags'],
  notes: [
    'DESTRUCTIVE operation - deletes scenarios from feature files',
    'Does not delete entire feature files, only matching scenarios',
    'Use --force to skip confirmation (dangerous)',
    'Consider using remove-tag-from-scenario instead if just removing tags',
  ],
};

export default config;

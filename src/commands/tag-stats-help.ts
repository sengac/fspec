import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'tag-stats',
  description: 'Display statistics about tag usage across feature files',
  usage: 'fspec tag-stats',
  examples: [
    {
      command: 'fspec tag-stats',
      description: 'Show tag usage statistics',
      output:
        'Tag Usage Statistics:\n\n@phase1: 45 features, 123 scenarios\n@phase2: 12 features, 34 scenarios\n@cli: 30 features\n@parser: 15 features',
    },
  ],
  relatedCommands: ['list-tags', 'validate-tags', 'register-tag'],
  notes: [
    'Shows how many features and scenarios use each tag',
    'Helps identify unused or underused tags',
    'Only counts tags that are actually used in .feature files',
  ],
};

export default config;

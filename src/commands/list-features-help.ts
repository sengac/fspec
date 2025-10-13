import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-features',
  description: 'List all feature files with optional filtering by tags',
  usage: 'fspec list-features [options]',
  whenToUse:
    'Use to see all features in your project, or filter by specific tags to find features in a particular phase, component, or priority level.',
  options: [
    {
      flag: '--tag <tag>',
      description: 'Filter by tag (can be used multiple times, e.g., --tag=@phase1 --tag=@critical)',
    },
    {
      flag: '--format <format>',
      description: 'Output format: table (default), json, or simple',
    },
  ],
  examples: [
    {
      command: 'fspec list-features',
      description: 'List all features',
      output: 'spec/features/login.feature\nspec/features/signup.feature\n\nFound 2 features',
    },
    {
      command: 'fspec list-features --tag=@phase1',
      description: 'List phase 1 features',
      output: 'spec/features/login.feature [@phase1 @authentication]\n\nFound 1 feature',
    },
    {
      command: 'fspec list-features --tag=@phase1 --tag=@critical',
      description: 'List features with multiple tags',
      output: 'spec/features/login.feature [@phase1 @critical @authentication]\n\nFound 1 feature',
    },
  ],
  relatedCommands: ['show-feature', 'create-feature', 'list-tags', 'validate'],
  notes: [
    'Searches spec/features/ directory',
    'Multiple --tag flags filter with AND logic (feature must have all tags)',
    'JSON format useful for tooling integration',
  ],
};

export default config;

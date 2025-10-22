import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-feature-tags',
  description: 'List all tags on a specific feature file',
  usage: 'fspec list-feature-tags <file> [options]',
  whenToUse:
    'Use this command when you need to see all tags applied to a feature file, either just the tag names or with their category information from the tag registry.',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path (e.g., spec/features/login.feature)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--show-categories',
      description: 'Show tag categories from registry (spec/tags.json)',
    },
  ],
  examples: [
    {
      command: 'fspec list-feature-tags spec/features/login.feature',
      description: 'List all tags on a feature file',
      output: '@critical\n@authentication\n@cli\n@critical',
    },
    {
      command:
        'fspec list-feature-tags spec/features/login.feature --show-categories',
      description: 'List tags with their categories',
      output:
        '@critical (Phase Tags)\n@authentication (Feature Group Tags)\n@cli (Component Tags)\n@critical (Priority Tags)',
    },
  ],
  relatedCommands: [
    'add-tag-to-feature',
    'remove-tag-from-feature',
    'list-scenario-tags',
    'list-tags',
    'validate-tags',
  ],
  notes: [
    'Shows only tags at the feature level (not scenario-level tags)',
    'Use --show-categories to see which category each tag belongs to',
    'Tags are displayed in the order they appear in the feature file',
  ],
};

export default config;

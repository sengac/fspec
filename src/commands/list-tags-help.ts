import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-tags',
  description: 'List all registered tags from spec/tags.json',
  usage: 'fspec list-tags [options]',
  options: [
    {
      flag: '--category <category>',
      description:
        'Filter tags by category (e.g., "Phase Tags", "Component Tags")',
    },
  ],
  examples: [
    {
      command: 'fspec list-tags',
      description: 'List all tags',
      output:
        'Phase Tags:\n  @phase1 - Phase 1 features\n  @phase2 - Phase 2 features\n\nComponent Tags:\n  @cli - CLI commands\n  @parser - Parser features',
    },
    {
      command: 'fspec list-tags --category="Phase Tags"',
      description: 'List tags in specific category',
      output: '@phase1 - Phase 1 features\n@phase2 - Phase 2 features',
    },
  ],
  relatedCommands: ['register-tag', 'validate-tags', 'tag-stats'],
};

export default config;

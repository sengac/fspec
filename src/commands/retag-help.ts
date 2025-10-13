import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'retag',
  description: 'Replace one tag with another across all feature files',
  usage: 'fspec retag <old-tag> <new-tag>',
  arguments: [
    {
      name: 'old-tag',
      description: 'Tag to replace',
      required: true,
    },
    {
      name: 'new-tag',
      description: 'New tag name',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec retag @wip @in-progress',
      description: 'Replace @wip with @in-progress in all features',
      output:
        '✓ Replaced @wip with @in-progress in 5 feature files\n✓ Updated 12 scenarios',
    },
  ],
  relatedCommands: ['register-tag', 'update-tag', 'validate-tags'],
  notes: [
    'Both tags must be registered in spec/tags.json',
    'Updates both feature-level and scenario-level tags',
    'Run validate-tags after retagging to verify changes',
  ],
};

export default config;

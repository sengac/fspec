import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'register-tag',
  description: 'Register a new tag in the tag registry (spec/tags.json)',
  usage: 'fspec register-tag <tag> <category> <description>',
  whenToUse:
    'Use when creating a new custom tag that will be used in feature files. All tags must be registered before use to maintain tag compliance.',
  arguments: [
    {
      name: 'tag',
      description: 'Tag name including @ symbol (e.g., @my-tag)',
      required: true,
    },
    {
      name: 'category',
      description: 'Tag category (e.g., "Phase Tags", "Component Tags", "Custom Tags")',
      required: true,
    },
    {
      name: 'description',
      description: 'Description of what this tag represents',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec register-tag @performance "Technical Tags" "Performance-critical features"',
      description: 'Register a custom tag',
      output: '✓ Tag @performance registered\n  Category: Technical Tags',
    },
  ],
  prerequisites: [
    'spec/tags.json must exist (created by init)',
  ],
  relatedCommands: ['list-tags', 'update-tag', 'delete-tag', 'validate-tags'],
  notes: [
    'Tags must start with @ symbol',
    'Tags are stored in spec/tags.json',
    'Use lowercase-with-hyphens for tag names',
    'Register tags before using them in feature files',
  ],
};

export default config;

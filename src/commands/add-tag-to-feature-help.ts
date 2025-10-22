import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-tag-to-feature',
  description: 'Add one or more tags to a feature file (feature-level tags)',
  usage: 'fspec add-tag-to-feature <file> <tags...> [options]',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'tags...',
      description: 'One or more tags to add (e.g., @critical @critical)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--validate-registry',
      description: 'Validate tags against spec/tags.json before adding',
    },
  ],
  examples: [
    {
      command: 'fspec add-tag-to-feature spec/features/login.feature @critical',
      description: 'Add single tag',
      output: '✓ Added tag @critical to spec/features/login.feature',
    },
    {
      command:
        'fspec add-tag-to-feature spec/features/login.feature @critical @critical',
      description: 'Add multiple tags',
      output:
        '✓ Added tags @critical, @critical to spec/features/login.feature',
    },
    {
      command:
        'fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry',
      description: 'Add tag with registry validation',
      output:
        'Error: Tag @custom-tag is not registered in spec/tags.json\nRegister it first with: fspec register-tag @custom-tag "Category" "Description"',
    },
  ],
  relatedCommands: [
    'remove-tag-from-feature',
    'list-feature-tags',
    'add-tag-to-scenario',
    'register-tag',
  ],
  notes: [
    'Tags must be registered first (unless --validate-registry is skipped)',
    'Duplicates are automatically ignored',
    'Use --validate-registry to enforce tag registry compliance',
  ],
};

export default config;

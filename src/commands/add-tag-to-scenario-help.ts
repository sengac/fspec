import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-tag-to-scenario',
  description: 'Add one or more tags to a specific scenario',
  usage: 'fspec add-tag-to-scenario <file> <scenario> <tags...> [options]',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'scenario',
      description: 'Scenario name (must match exactly)',
      required: true,
    },
    {
      name: 'tags...',
      description: 'One or more tags to add',
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
      command:
        'fspec add-tag-to-scenario spec/features/login.feature "Login with invalid password" @edge-case',
      description: 'Add tag to scenario',
      output: '✓ Added tag @edge-case to scenario',
    },
    {
      command:
        'fspec add-tag-to-scenario spec/features/login.feature "Login scenario" @WORK-UNIT-001 --validate-registry',
      description: 'Add tag with registry validation',
      output: '✓ Added tag @WORK-UNIT-001 to scenario "Login scenario"',
    },
  ],
  relatedCommands: [
    'remove-tag-from-scenario',
    'list-scenario-tags',
    'register-tag',
  ],
  notes: [
    'Scenario name is case-sensitive',
    'Use --validate-registry to enforce tag registry compliance',
    'Duplicates are automatically ignored',
  ],
};

export default config;

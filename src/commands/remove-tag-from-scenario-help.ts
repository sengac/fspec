import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-tag-from-scenario',
  description: 'Remove one or more tags from a specific scenario in a feature file',
  usage: 'fspec remove-tag-from-scenario <file> <scenario> <tags...>',
  whenToUse:
    'Use this command when you need to remove tags from a specific scenario, such as removing @wip after completing work, removing @deprecated before deletion, or cleaning up incorrect tags.',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path (e.g., spec/features/login.feature)',
      required: true,
    },
    {
      name: 'scenario',
      description:
        'Scenario name exactly as it appears in the feature file (e.g., "Login with valid credentials")',
      required: true,
    },
    {
      name: 'tags...',
      description:
        'One or more tags to remove (e.g., @wip @deprecated) - must include @ symbol',
      required: true,
    },
  ],
  examples: [
    {
      command:
        'fspec remove-tag-from-scenario spec/features/login.feature "Login with valid credentials" @wip',
      description: 'Remove single tag from scenario',
      output: '✓ Removed tag @wip from scenario "Login with valid credentials"',
    },
    {
      command:
        'fspec remove-tag-from-scenario spec/features/login.feature "Login with valid credentials" @wip @manual',
      description: 'Remove multiple tags from scenario',
      output:
        '✓ Removed tags @wip, @manual from scenario "Login with valid credentials"',
    },
  ],
  commonErrors: [
    {
      error: 'Scenario not found',
      fix: 'Ensure scenario name matches exactly (case-sensitive)',
    },
    {
      error: 'Tag not found on scenario',
      fix: 'Use list-scenario-tags to see current tags on the scenario',
    },
  ],
  relatedCommands: [
    'add-tag-to-scenario',
    'list-scenario-tags',
    'remove-tag-from-feature',
    'retag',
  ],
  notes: [
    'Scenario names are case-sensitive and must match exactly',
    'Tags must include the @ symbol',
    'Removing a tag that does not exist on the scenario will show an error',
    'Feature file is automatically reformatted after tag removal',
  ],
};

export default config;

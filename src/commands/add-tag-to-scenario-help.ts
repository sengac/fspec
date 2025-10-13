import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-tag-to-scenario',
  description: 'Add one or more tags to a specific scenario',
  usage: 'fspec add-tag-to-scenario <file> <scenario> <tags...>',
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
  examples: [
    {
      command: 'fspec add-tag-to-scenario spec/features/login.feature "Login with invalid password" @edge-case',
      description: 'Add tag to scenario',
      output: '✓ Added tag @edge-case to scenario',
    },
  ],
  relatedCommands: ['remove-tag-from-scenario', 'list-scenario-tags'],
  notes: [
    'Scenario name is case-sensitive',
  ],
};

export default config;

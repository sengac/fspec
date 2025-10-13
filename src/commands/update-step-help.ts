import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-step',
  description: 'Update a step in a scenario',
  usage: 'fspec update-step <file> <scenario> <index> <text>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'scenario',
      description: 'Scenario name',
      required: true,
    },
    {
      name: 'index',
      description: 'Step index (0-based)',
      required: true,
    },
    {
      name: 'text',
      description: 'New step text',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec update-step spec/features/login.feature "Login scenario" 0 "Given I am logged out"',
      description: 'Update step text',
      output: '✓ Updated step in scenario',
    },
  ],
  relatedCommands: ['add-step', 'delete-step'],
};

export default config;

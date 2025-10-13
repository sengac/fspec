import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-step',
  description: 'Add a step to a scenario in a feature file',
  usage: 'fspec add-step <file> <scenario> <type> <text>',
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
      name: 'type',
      description: 'Step type: given, when, then, and, or but',
      required: true,
    },
    {
      name: 'text',
      description: 'Step text (e.g., "I am logged in")',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-step spec/features/login.feature "Login with valid credentials" given "I am on the login page"',
      description: 'Add given step',
      output: '✓ Added step to scenario',
    },
  ],
  relatedCommands: ['add-scenario', 'update-step', 'delete-step'],
};

export default config;

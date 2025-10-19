import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-step',
  description: 'Delete a step from a scenario',
  usage: 'fspec delete-step <file> <scenario> <index> [options]',
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
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation',
    },
  ],
  examples: [
    {
      command:
        'fspec delete-step spec/features/login.feature "Login with valid credentials" 2',
      description: 'Delete step',
      output: '✓ Deleted step from scenario',
    },
  ],
  relatedCommands: ['add-step', 'update-step'],
};

export default config;

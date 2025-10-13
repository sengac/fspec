import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-background',
  description: 'Add Background section with user story to a feature file',
  usage: 'fspec add-background <file> <role> <action> <benefit>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'role',
      description: 'User role (e.g., "developer", "user")',
      required: true,
    },
    {
      name: 'action',
      description: 'What they want to do',
      required: true,
    },
    {
      name: 'benefit',
      description: 'Why they want it',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-background spec/features/login.feature "user" "login securely" "access my account"',
      description: 'Add user story',
      output: '✓ Added Background section\n  As a user\n  I want to login securely\n  So that I can access my account',
    },
  ],
  relatedCommands: ['create-feature', 'show-feature'],
};

export default config;

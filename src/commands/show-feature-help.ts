import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-feature',
  description:
    'Display the contents of a feature file with syntax highlighting',
  usage: 'fspec show-feature <file>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-feature spec/features/login.feature',
      description: 'Display feature file',
      output:
        'Feature: User Login\n  Scenario: Login with valid credentials\n    Given I am on the login page...',
    },
  ],
  relatedCommands: ['list-features', 'validate'],
};

export default config;

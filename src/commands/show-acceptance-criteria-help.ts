import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-acceptance-criteria',
  description: 'Display acceptance criteria (scenarios) for a feature file',
  usage: 'fspec show-acceptance-criteria <file>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-acceptance-criteria spec/features/login.feature',
      description: 'Show acceptance criteria',
      output: 'Feature: User Login\n\nScenario: Login with valid credentials\n  Given I am on the login page\n  When I enter valid credentials\n  Then I should be logged in',
    },
  ],
  relatedCommands: ['get-scenarios', 'show-feature'],
};

export default config;

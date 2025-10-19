import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-scenario',
  description: 'Add a new scenario to an existing feature file',
  usage: 'fspec add-scenario <file> <scenario-name>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path (e.g., spec/features/login.feature)',
      required: true,
    },
    {
      name: 'scenario-name',
      description: 'Name of the scenario (e.g., "Login with invalid password")',
      required: true,
    },
  ],
  examples: [
    {
      command:
        'fspec add-scenario spec/features/login.feature "Login with invalid password"',
      description: 'Add new scenario',
      output:
        '✓ Added scenario "Login with invalid password" to spec/features/login.feature',
    },
  ],
  relatedCommands: ['add-step', 'create-feature', 'delete-scenario'],
  notes: [
    'Creates scenario with placeholder Given/When/Then steps',
    'Scenario is added at the end of the feature file',
  ],
};

export default config;

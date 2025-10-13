import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-scenario-tags',
  description: 'List all tags on a specific scenario',
  usage: 'fspec list-scenario-tags <file> <scenario>',
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
  ],
  examples: [
    {
      command: 'fspec list-scenario-tags spec/features/login.feature "Login with invalid password"',
      description: 'List scenario tags',
      output: '@edge-case\n@smoke',
    },
  ],
  relatedCommands: ['add-tag-to-scenario', 'list-feature-tags'],
};

export default config;

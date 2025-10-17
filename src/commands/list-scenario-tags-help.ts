import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-scenario-tags',
  description: 'List all tags on a specific scenario',
  usage: 'fspec list-scenario-tags <file> <scenario> [options]',
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
  options: [
    {
      flag: '--show-categories',
      description: 'Show tag categories from registry (spec/tags.json)',
    },
  ],
  examples: [
    {
      command: 'fspec list-scenario-tags spec/features/login.feature "Login with invalid password"',
      description: 'List scenario tags',
      output: '@edge-case\n@smoke',
    },
    {
      command: 'fspec list-scenario-tags spec/features/login.feature "Valid user login" --show-categories',
      description: 'List tags with categories',
      output: '@WORK-UNIT-001 (Work Unit Tags)\n@smoke (Test Type Tags)',
    },
  ],
  relatedCommands: ['add-tag-to-scenario', 'list-feature-tags'],
};

export default config;

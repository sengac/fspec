import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'get-scenarios',
  description: 'Extract scenarios from feature files with optional filtering',
  usage: 'fspec get-scenarios [options]',
  options: [
    {
      flag: '--file <file>',
      description: 'Specific feature file',
    },
    {
      flag: '--tag <tag>',
      description: 'Filter by tag (can use multiple)',
    },
    {
      flag: '--format <format>',
      description: 'Output format: text, json, or list',
    },
  ],
  examples: [
    {
      command: 'fspec get-scenarios --file spec/features/login.feature',
      description: 'Get scenarios from file',
      output: 'Login with valid credentials\nLogin with invalid password\nLogin with locked account',
    },
  ],
  relatedCommands: ['show-acceptance-criteria', 'list-features'],
};

export default config;

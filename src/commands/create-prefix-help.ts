import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-prefix',
  description:
    'Register a new work unit prefix for organizing work by component or area',
  usage: 'fspec create-prefix <prefix> <description>',
  whenToUse:
    'Use when starting work on a new component or area that needs its own work unit namespace.',
  arguments: [
    {
      name: 'prefix',
      description: 'Prefix code (e.g., AUTH, UI, API) - uppercase, short',
      required: true,
    },
    {
      name: 'description',
      description: 'Description of what this prefix represents',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec create-prefix AUTH "Authentication features"',
      description: 'Register new prefix',
      output: '✓ Created prefix AUTH\n  Description: Authentication features',
    },
  ],
  relatedCommands: [
    'list-prefixes',
    'update-prefix',
    'create-story',
    'create-bug',
    'create-task',
  ],
  notes: [
    'Prefix must be uppercase',
    'Required before creating work units with that prefix',
  ],
};

export default config;

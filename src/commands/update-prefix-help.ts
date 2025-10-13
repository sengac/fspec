import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-prefix',
  description: 'Update the description of a work unit prefix',
  usage: 'fspec update-prefix <prefix> [options]',
  arguments: [
    {
      name: 'prefix',
      description: 'Prefix to update',
      required: true,
    },
  ],
  options: [
    {
      flag: '--description <description>',
      description: 'New description',
    },
  ],
  examples: [
    {
      command: 'fspec update-prefix AUTH --description "Authentication and authorization"',
      description: 'Update prefix description',
      output: '✓ Updated prefix AUTH',
    },
  ],
  relatedCommands: ['create-prefix', 'list-prefixes'],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-prefixes',
  description: 'List all registered work unit prefixes',
  usage: 'fspec list-prefixes',
  examples: [
    {
      command: 'fspec list-prefixes',
      description: 'List all prefixes',
      output:
        'AUTH - Authentication features\nUI - User interface components\nAPI - API endpoints\n\nFound 3 prefixes',
    },
  ],
  relatedCommands: [
    'create-prefix',
    'create-story',
    'create-bug',
    'create-task',
  ],
};

export default config;

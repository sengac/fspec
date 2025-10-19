import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-epics',
  description: 'List all epics in the project',
  usage: 'fspec list-epics',
  examples: [
    {
      command: 'fspec list-epics',
      description: 'List all epics',
      output:
        'user-management - User Management Features\nauth - Authentication System\n\nFound 2 epics',
    },
  ],
  relatedCommands: ['create-epic', 'show-epic', 'delete-epic'],
};

export default config;

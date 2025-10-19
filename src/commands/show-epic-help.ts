import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-epic',
  description: 'Display details of an epic including associated work units',
  usage: 'fspec show-epic <epicId>',
  arguments: [
    {
      name: 'epicId',
      description: 'Epic ID (e.g., user-management)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-epic user-management',
      description: 'Show epic details',
      output:
        'user-management\nTitle: User Management Features\nDescription: Handles user accounts\n\nWork Units:\n  AUTH-001 - User login feature\n  AUTH-002 - Password reset',
    },
  ],
  relatedCommands: ['list-epics', 'create-epic', 'list-work-units'],
};

export default config;

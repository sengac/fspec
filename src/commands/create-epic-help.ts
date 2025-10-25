import type { CommandHelpConfig } from '../utils/help-formatter';

const createEpicHelp: CommandHelpConfig = {
  name: 'create-epic',
  description:
    'Create a new epic for organizing work units into high-level initiatives',
  usage: 'fspec create-epic <epicId> <title>',
  arguments: [
    {
      name: 'epicId',
      description:
        'Epic ID in lowercase-with-hyphens format (e.g., user-management)',
      required: true,
    },
    {
      name: 'title',
      description: 'Human-readable title for the epic',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <description>',
      description: 'Optional description providing more context about the epic',
    },
  ],
  examples: [
    {
      command: 'fspec create-epic user-management "User Management Features"',
      description: 'Create epic with ID and title',
      output:
        '✓ Created epic user-management\n  Title: User Management Features',
    },
    {
      command:
        'fspec create-epic auth "Authentication System" -d "Handles user login and sessions"',
      description: 'Create epic with description',
      output:
        '✓ Created epic auth\n  Title: Authentication System\n  Description: Handles user login and sessions',
    },
  ],
  relatedCommands: ['list-epics', 'show-epic', 'create-story', 'create-bug', 'create-task'],
  notes: [
    'Epic IDs must be lowercase with hyphens (no spaces or special characters)',
    'Epics help organize related work units into business initiatives',
  ],
};

export default createEpicHelp;

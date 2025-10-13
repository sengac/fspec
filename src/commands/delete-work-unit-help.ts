import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-work-unit',
  description: 'Delete a work unit and all its data',
  usage: 'fspec delete-work-unit <id> [options]',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID to delete',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation prompt',
    },
  ],
  examples: [
    {
      command: 'fspec delete-work-unit AUTH-999',
      description: 'Delete work unit',
      output: '✓ Deleted work unit AUTH-999',
    },
  ],
  relatedCommands: ['list-work-units', 'create-work-unit'],
  notes: [
    'Deletes all Example Mapping data',
    'Removes dependency relationships',
    'Cannot be undone',
  ],
};

export default config;

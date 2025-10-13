import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-epic',
  description: 'Delete an epic (does not delete associated work units)',
  usage: 'fspec delete-epic <epicId> [options]',
  arguments: [
    {
      name: 'epicId',
      description: 'Epic ID to delete',
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
      command: 'fspec delete-epic old-feature',
      description: 'Delete epic with confirmation',
      output: '✓ Deleted epic old-feature',
    },
  ],
  relatedCommands: ['create-epic', 'list-epics'],
  notes: [
    'Work units associated with the epic are NOT deleted',
    'Epic associations are removed from work units',
  ],
};

export default config;

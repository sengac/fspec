import type { CommandHelpConfig } from '../utils/help-formatter';

const listWorkUnitsHelp: CommandHelpConfig = {
  name: 'list-work-units',
  description:
    'List work units with optional filtering by status, prefix, or epic',
  usage: 'fspec list-work-units [options]',
  whenToUse:
    "Use this command when you need to view work units in the backlog, see what's in progress, or filter by specific criteria like status, prefix, or epic.",
  options: [
    {
      flag: '-s, --status <status>',
      description:
        'Filter by workflow status: backlog, specifying, testing, implementing, validating, done, blocked',
    },
    {
      flag: '--prefix <prefix>',
      description: 'Filter by work unit prefix (e.g., AUTH, UI, API)',
    },
    {
      flag: '--epic <epic>',
      description: 'Filter by epic name',
    },
  ],
  examples: [
    {
      command: 'fspec list-work-units',
      description: 'List all work units',
      output: 'AUTH-001 - User login feature\nUI-002 - Dashboard layout',
    },
    {
      command: 'fspec list-work-units --status=backlog',
      description: 'List only backlog items',
      output: 'AUTH-003 - Password reset\nAPI-004 - User endpoints',
    },
    {
      command: 'fspec list-work-units --prefix=AUTH',
      description: 'List all AUTH-prefixed work units',
      output: 'AUTH-001 - User login feature\nAUTH-003 - Password reset',
    },
  ],
  typicalWorkflow:
    'backlog → specifying → testing → implementing → validating → done. Use --status flag to see work units at each stage.',
  relatedCommands: [
    'show-work-unit',
    'create-work-unit',
    'update-work-unit-status',
    'board',
  ],
};

export default listWorkUnitsHelp;

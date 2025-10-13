import type { CommandHelpConfig} from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-work-units',
  description: 'Query work units with advanced filters and output formats',
  usage: 'fspec query-work-units [options]',
  options: [
    {
      flag: '--status <status>',
      description: 'Filter by status',
    },
    {
      flag: '--epic <epic>',
      description: 'Filter by epic',
    },
    {
      flag: '--prefix <prefix>',
      description: 'Filter by prefix',
    },
    {
      flag: '--format <format>',
      description: 'Output format: table, json, csv',
    },
  ],
  examples: [
    {
      command: 'fspec query-work-units --status=implementing --format=json',
      description: 'Query with filters',
      output: '[{"id":"AUTH-001","status":"implementing","title":"Login feature"}]',
    },
  ],
  relatedCommands: ['list-work-units', 'export-work-units'],
};

export default config;

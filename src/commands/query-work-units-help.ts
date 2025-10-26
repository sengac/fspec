import type { CommandHelpConfig } from '../utils/help-formatter';

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
      flag: '--type <type>',
      description: 'Filter by work unit type: story, task, or bug',
    },
    {
      flag: '--tag <tag>',
      description: 'Filter by tag (e.g., @cli, @high)',
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
      output:
        '[{"id":"AUTH-001","status":"implementing","title":"Login feature"}]',
    },
    {
      command: 'fspec query-work-units --type=story --status=done --tag=@cli',
      description: 'Query completed stories tagged with @cli',
      output: '[{"id":"CLI-001","status":"done","title":"CLI commands"}]',
    },
  ],
  relatedCommands: [
    'list-work-units',
    'export-work-units',
    'search-scenarios',
    'compare-implementations',
  ],
};

export default config;

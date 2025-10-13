import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'export-work-units',
  description: 'Export work units to JSON or CSV format',
  usage: 'fspec export-work-units <format> <output> [options]',
  arguments: [
    {
      name: 'format',
      description: 'Output format: json or csv',
      required: true,
    },
    {
      name: 'output',
      description: 'Output file path',
      required: true,
    },
  ],
  options: [
    {
      flag: '--status <status>',
      description: 'Filter by status',
    },
    {
      flag: '--epic <epic>',
      description: 'Filter by epic',
    },
  ],
  examples: [
    {
      command: 'fspec export-work-units json work-units.json',
      description: 'Export to JSON',
      output: '✓ Exported 42 work units to work-units.json',
    },
  ],
  relatedCommands: ['list-work-units', 'query-work-units'],
};

export default config;

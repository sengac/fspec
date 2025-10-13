import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'export-example-map',
  description: 'Export Example Mapping data from work unit to JSON file',
  usage: 'fspec export-example-map <workUnitId> <file>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'file',
      description: 'Output JSON file path',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec export-example-map AUTH-001 example-map.json',
      description: 'Export Example Mapping',
      output: '✓ Exported Example Mapping to example-map.json',
    },
  ],
  relatedCommands: ['import-example-map', 'show-work-unit'],
};

export default config;

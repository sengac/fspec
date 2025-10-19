import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'import-example-map',
  description: 'Import Example Mapping data from JSON file to work unit',
  usage: 'fspec import-example-map <workUnitId> <file>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'file',
      description: 'JSON file path containing Example Mapping data',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec import-example-map AUTH-001 example-map.json',
      description: 'Import Example Mapping',
      output:
        '✓ Imported Example Mapping data to AUTH-001\n  3 rules, 5 examples, 2 questions',
    },
  ],
  relatedCommands: ['export-example-map', 'show-work-unit'],
};

export default config;

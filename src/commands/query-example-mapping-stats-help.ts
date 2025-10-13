import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-example-mapping-stats',
  description: 'Show Example Mapping coverage statistics across work units',
  usage: 'fspec query-example-mapping-stats [options]',
  whenToUse:
    'Use to assess specification quality and identify work units that need more Example Mapping.',
  options: [
    {
      flag: '--status <status>',
      description: 'Filter by status',
    },
  ],
  examples: [
    {
      command: 'fspec query-example-mapping-stats',
      description: 'Show EM stats',
      output: 'Work units with Example Mapping: 25/42 (59%)\nTotal rules: 87\nTotal examples: 134\nUnanswered questions: 12',
    },
  ],
  relatedCommands: ['show-work-unit', 'add-rule', 'add-example'],
};

export default config;

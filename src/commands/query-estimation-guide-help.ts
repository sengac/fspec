import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-estimation-guide',
  description: 'Get estimation guidance based on historical data for a work unit',
  usage: 'fspec query-estimation-guide <workUnitId> [options]',
  whenToUse:
    'Use after Example Mapping to get data-driven estimation guidance based on similar historical work.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      flag: '--similar <count>',
      description: 'Number of similar work units to analyze',
    },
  ],
  examples: [
    {
      command: 'fspec query-estimation-guide AUTH-001',
      description: 'Get estimation guidance',
      output: 'Based on 5 similar work units:\n  Suggested estimate: 5 points\n  Range: 3-8 points\n  Average actual: 5.2 points',
    },
  ],
  relatedCommands: ['update-work-unit-estimate', 'query-estimate-accuracy'],
};

export default config;

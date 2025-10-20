import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-estimate-accuracy',
  description:
    'Show estimation accuracy metrics comparing estimates to actuals',
  usage: 'fspec query-estimate-accuracy [options]',
  whenToUse: 'Use to assess estimation accuracy and improve future estimates.',
  options: [
    {
      flag: '--prefix <prefix>',
      description: 'Filter by prefix',
    },
  ],
  examples: [
    {
      command: 'fspec query-estimate-accuracy',
      description: 'Show accuracy metrics',
      output:
        'Average estimate accuracy: 87%\nUnderestimated: 5 work units\nOverestimated: 3 work units',
    },
  ],
  relatedCommands: ['update-work-unit-estimate', 'query-metrics'],
};

export default config;

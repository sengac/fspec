import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-metrics',
  description: 'Query project metrics and statistics',
  usage: 'fspec query-metrics [options]',
  options: [
    {
      flag: '--metric <metric>',
      description: 'Specific metric to query',
    },
    {
      flag: '--format <format>',
      description: 'Output format: table or json',
    },
    {
      flag: '--work-unit-id <id>',
      description: 'Specific work unit to query metrics for',
    },
  ],
  examples: [
    {
      command: 'fspec query-metrics',
      description: 'Show all metrics',
      output: 'Velocity: 23 points/week\nCycle time: 3.5 days avg\nThroughput: 5 work units/week',
    },
  ],
  relatedCommands: ['record-metric', 'generate-summary-report'],
};

export default config;

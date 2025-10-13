import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'record-metric',
  description: 'Record a project metric value',
  usage: 'fspec record-metric <metric> <value> [options]',
  arguments: [
    {
      name: 'metric',
      description: 'Metric name (e.g., velocity, cycle-time)',
      required: true,
    },
    {
      name: 'value',
      description: 'Metric value',
      required: true,
    },
  ],
  options: [
    {
      flag: '--date <date>',
      description: 'Date for metric (ISO format)',
    },
  ],
  examples: [
    {
      command: 'fspec record-metric velocity 23',
      description: 'Record velocity',
      output: '✓ Recorded metric: velocity = 23',
    },
  ],
  relatedCommands: ['query-metrics', 'generate-summary-report'],
};

export default config;

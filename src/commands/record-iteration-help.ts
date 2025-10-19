import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'record-iteration',
  description: 'Record an iteration or sprint with metadata',
  usage: 'fspec record-iteration <name> [options]',
  arguments: [
    {
      name: 'name',
      description: 'Iteration name (e.g., "Sprint 1", "Week 42")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--start <date>',
      description: 'Start date (ISO format)',
    },
    {
      flag: '--end <date>',
      description: 'End date (ISO format)',
    },
  ],
  examples: [
    {
      command:
        'fspec record-iteration "Sprint 1" --start 2025-10-01 --end 2025-10-15',
      description: 'Record iteration',
      output: '✓ Recorded iteration "Sprint 1"',
    },
  ],
  relatedCommands: ['query-metrics', 'generate-summary-report'],
};

export default config;

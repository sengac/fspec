import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-summary-report',
  description: 'Generate a comprehensive project summary report',
  usage: 'fspec generate-summary-report [options]',
  whenToUse:
    'Use to create status reports for stakeholders showing progress, metrics, and health indicators.',
  options: [
    {
      flag: '--output <file>',
      description: 'Output file path (markdown format)',
    },
    {
      flag: '--format <format>',
      description: 'Output format: markdown, json, or html',
    },
  ],
  examples: [
    {
      command: 'fspec generate-summary-report --output report.md',
      description: 'Generate report',
      output:
        '✓ Generated summary report: report.md\n  42 work units, 87% complete',
    },
  ],
  relatedCommands: ['query-metrics', 'board', 'query-work-units'],
};

export default config;

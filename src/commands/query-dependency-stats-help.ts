import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-dependency-stats',
  description: 'Show dependency statistics and potential blockers',
  usage: 'fspec query-dependency-stats [options]',
  whenToUse:
    'Use to identify dependency bottlenecks, blocked work units, and critical paths.',
  options: [
    {
      flag: '--show-critical-path',
      description: 'Highlight critical path through dependencies',
    },
  ],
  examples: [
    {
      command: 'fspec query-dependency-stats',
      description: 'Show dependency stats',
      output: 'Total dependencies: 42\nBlocking work units: 5\nBlocked work units: 8\nWork units with no dependencies: 20',
    },
  ],
  relatedCommands: ['export-dependencies', 'add-dependency'],
};

export default config;

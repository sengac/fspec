import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-work-unit-estimate',
  description: 'Set story point estimate for a work unit using Fibonacci scale',
  usage: 'fspec update-work-unit-estimate <id> <points>',
  whenToUse:
    'Use after Example Mapping when you have enough information to estimate complexity. Use Fibonacci sequence: 1, 2, 3, 5, 8, 13.',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'points',
      description: 'Story points (Fibonacci: 1, 2, 3, 5, 8, 13, 21)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec update-work-unit-estimate AUTH-001 5',
      description: 'Set estimate',
      output: '✓ Updated estimate for AUTH-001 to 5 points',
    },
  ],
  relatedCommands: ['show-work-unit', 'query-estimate-accuracy'],
  notes: [
    'Use Fibonacci sequence for estimates',
    'Estimate after completing Example Mapping',
    '1=trivial, 3=small, 5=medium, 8=large, 13=very large',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-assumption',
  description: 'Add an assumption to a work unit during specification',
  usage: 'fspec add-assumption <work-unit-id> <assumption>',
  whenToUse:
    'Use to document assumptions made during specification that may need validation later.',
  arguments: [
    {
      name: 'work-unit-id',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'assumption',
      description: 'Assumption text',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-assumption AUTH-001 "Users have valid email addresses"',
      description: 'Add assumption',
      output: '✓ Assumption added successfully',
    },
  ],
  relatedCommands: ['add-rule', 'add-question', 'show-work-unit'],
};

export default config;

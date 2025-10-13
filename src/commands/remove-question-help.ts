import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-question',
  description: 'Remove a question from Example Mapping by index',
  usage: 'fspec remove-question <workUnitId> <index>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description: 'Question index (from show-work-unit)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec remove-question AUTH-001 0',
      description: 'Remove question at index 0',
      output: '✓ Removed question from AUTH-001',
    },
  ],
  relatedCommands: ['add-question', 'answer-question', 'show-work-unit'],
};

export default config;

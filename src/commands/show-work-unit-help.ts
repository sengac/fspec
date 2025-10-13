import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-work-unit',
  description: 'Display detailed information about a work unit including Example Mapping data',
  usage: 'fspec show-work-unit <id>',
  whenToUse:
    'Use to view complete details of a work unit: status, description, Example Mapping (rules, examples, questions), dependencies, and metadata.',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID (e.g., AUTH-001)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-work-unit AUTH-001',
      description: 'Show work unit details',
      output: 'AUTH-001\nStatus: specifying\n\nUser login feature\n\nRules:\n  1. Must validate email format\n\nQuestions:\n  [0] @human: Should we support OAuth?\n\nCreated: 13/10/2025',
    },
  ],
  relatedCommands: ['list-work-units', 'update-work-unit-status', 'add-question', 'add-rule'],
  notes: [
    'Shows all Example Mapping data (rules, examples, questions, assumptions)',
    'Shows dependency relationships',
    'Shows linked feature files',
  ],
};

export default config;

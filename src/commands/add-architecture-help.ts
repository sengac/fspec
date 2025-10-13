import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-architecture',
  description: 'Add architecture notes to a feature file using doc strings',
  usage: 'fspec add-architecture <file> <notes>',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'notes',
      description: 'Architecture notes text',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-architecture spec/features/login.feature "Uses bcrypt for password hashing"',
      description: 'Add architecture notes',
      output: '✓ Added architecture notes to spec/features/login.feature',
    },
  ],
  relatedCommands: ['create-feature', 'show-feature'],
  notes: [
    'Notes are added as doc strings (""") after Feature line',
    'Use for technical implementation details',
  ],
};

export default config;

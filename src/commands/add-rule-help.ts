import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-rule',
  description: 'Add a business rule to a work unit during Example Mapping',
  usage: 'fspec add-rule <workUnitId> <rule>',
  whenToUse:
    'Use during specifying phase when capturing business rules discovered through Example Mapping conversations.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'rule',
      description: 'Business rule description',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-rule AUTH-001 "Email must be valid format"',
      description: 'Add business rule',
      output: '✓ Rule added successfully',
    },
  ],
  relatedCommands: ['add-question', 'add-example', 'generate-scenarios', 'remove-rule'],
};

export default config;

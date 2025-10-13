import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-example',
  description: 'Add a concrete example to a work unit during Example Mapping',
  usage: 'fspec add-example <workUnitId> <example>',
  whenToUse:
    'Use during specifying phase to capture concrete examples that illustrate rules and will become test scenarios.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'example',
      description: 'Concrete example description',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-example AUTH-001 "Login with user@example.com and correct password"',
      description: 'Add example',
      output: '✓ Example added successfully',
    },
  ],
  relatedCommands: ['add-rule', 'add-question', 'generate-scenarios', 'remove-example'],
};

export default config;

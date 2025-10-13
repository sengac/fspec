import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'answer-question',
  description: 'Answer a question from Example Mapping and optionally convert to rule or assumption',
  usage: 'fspec answer-question <workUnitId> <index> [options]',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description: 'Question index number (from show-work-unit)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--answer <answer>',
      description: 'The answer text',
    },
    {
      flag: '--add-to-rules',
      description: 'Also add answer as a rule',
    },
    {
      flag: '--add-to-assumptions',
      description: 'Also add answer as an assumption',
    },
  ],
  examples: [
    {
      command: 'fspec answer-question AUTH-001 0 --answer "Yes, support Google OAuth"',
      description: 'Answer question',
      output: '✓ Answered question: "Should we support OAuth?"\n  Answer: "Yes, support Google OAuth"',
    },
    {
      command: 'fspec answer-question AUTH-001 0 --answer "Email required" --add-to-rules',
      description: 'Answer and add as rule',
      output: '✓ Answered question and added to rules',
    },
  ],
  relatedCommands: ['add-question', 'add-rule', 'add-assumption'],
};

export default config;

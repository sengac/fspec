import type { CommandHelpConfig } from '../utils/help-formatter';

const addQuestionHelp: CommandHelpConfig = {
  name: 'add-question',
  description:
    'Add a question to a work unit during Example Mapping discovery phase',
  usage: 'fspec add-question <workUnitId> <question>',
  whenToUse:
    'Use during Example Mapping in the specifying phase when you encounter uncertainties, ambiguities, or need clarification from the human about requirements, rules, or expected behavior.',
  commonPatterns: [
    'Use @human: prefix for questions directed at the human (e.g., "@human: Should we support OAuth?")',
    'Ask one focused question at a time rather than multiple questions in one',
    'Frame questions to elicit concrete examples or rules',
    'Questions should be answered before moving to testing phase',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID (e.g., AUTH-001)',
      required: true,
    },
    {
      name: 'question',
      description: 'The question text (use @human: prefix for human-directed questions)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec add-question AUTH-001 "@human: Should we support OAuth providers?"',
      description: 'Add question during Example Mapping',
      output: '✓ Question added successfully',
    },
    {
      command: 'fspec answer-question AUTH-001 0 --answer "Yes, support Google and GitHub OAuth"',
      description: 'Answer the question (index 0)',
      output: '✓ Answered question: "@human: Should we support OAuth providers?"\n  Answer: "Yes, support Google and GitHub OAuth"',
    },
  ],
  typicalWorkflow:
    '1. Add question during discovery\n  2. Human answers question\n  3. Convert answer to rule or assumption\n  4. Continue until no questions remain\n  5. Move to testing phase',
  relatedCommands: [
    'answer-question',
    'add-rule',
    'add-example',
    'generate-scenarios',
    'show-work-unit',
  ],
  notes: [
    'Questions must be answered before transitioning from specifying to testing',
    'Use show-work-unit to see all questions and their status',
    'Questions capture uncertainties that become rules or assumptions once answered',
  ],
};

export default addQuestionHelp;

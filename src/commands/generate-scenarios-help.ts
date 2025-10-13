import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-scenarios',
  description: 'Generate Gherkin scenarios from Example Mapping data (rules, examples, questions)',
  usage: 'fspec generate-scenarios <workUnitId>',
  whenToUse:
    'Use after completing Example Mapping when all questions are answered and ready to create feature file scenarios.',
  prerequisites: [
    'Work unit must have rules and examples',
    'All questions should be answered',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec generate-scenarios AUTH-001',
      description: 'Generate scenarios',
      output: '✓ Generated 3 scenarios\n\nScenario: Login with valid email...\nScenario: Login with invalid email...',
    },
  ],
  relatedCommands: ['add-rule', 'add-example', 'add-question', 'show-work-unit'],
  notes: [
    'Generates scenarios based on examples',
    'Uses rules as Given steps',
    'Output can be copied to feature file',
  ],
};

export default config;

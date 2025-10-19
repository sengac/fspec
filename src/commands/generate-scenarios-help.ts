import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-scenarios',
  description:
    'Generate Gherkin scenarios from Example Mapping data (rules, examples, questions)',
  usage: 'fspec generate-scenarios <workUnitId> [--feature=<name>]',
  whenToUse:
    'Use after completing Example Mapping when all questions are answered and ready to create feature file scenarios.',
  prerequisites: [
    'Work unit must have rules and examples',
    'All questions should be answered',
    'Work unit must have a title (for default naming) OR --feature flag must be provided',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      name: '--feature=<name>',
      description:
        'Override feature file name (without .feature extension). Defaults to work unit title converted to kebab-case.',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec generate-scenarios AUTH-001',
      description:
        'Generate scenarios using work unit title as feature file name',
      output:
        '✓ Generated 3 scenarios in spec/features/user-authentication.feature\n\nScenario: Login with valid email...\nScenario: Login with invalid email...',
    },
    {
      command: 'fspec generate-scenarios AUTH-001 --feature=login',
      description: 'Generate scenarios with explicit feature file name',
      output: '✓ Generated 3 scenarios in spec/features/login.feature',
    },
  ],
  relatedCommands: [
    'add-rule',
    'add-example',
    'add-question',
    'show-work-unit',
    'create-feature',
  ],
  notes: [
    'Feature files are named after CAPABILITIES (what IS), not work unit IDs',
    'Defaults to work unit title converted to kebab-case (e.g., "User Authentication" → user-authentication.feature)',
    'Use --feature flag to override the default name',
    'Throws error if work unit has no title and --feature not provided',
    'Generates scenarios based on examples',
    'Scenario titles are automatically cleaned (removes prefixes like REPRODUCTION:, MISSING:, ERROR WHEN:, etc.)',
    'Original example text is preserved as a comment above each scenario',
    'Uses intelligent step extraction to generate Given/When/Then steps from example text',
    'Falls back to [placeholders] if example cannot be parsed (triggers prefill detection)',
  ],
};

export default config;

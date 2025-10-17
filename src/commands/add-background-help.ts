import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-background',
  description: 'Add or update Background (user story) section in a feature file',
  usage: 'fspec add-background <feature> <text>',
  whenToUse:
    'Use this command to add a Background section containing the user story to a feature file. Background sections provide context for all scenarios in the feature.',
  prerequisites: ['Feature file must exist in spec/features/ directory'],
  arguments: [
    {
      name: 'feature',
      description: 'Feature file name or path (e.g., "login" or "spec/features/login.feature")',
      required: true,
    },
    {
      name: 'text',
      description: 'User story text in format: "As a [role] I want to [action] So that [benefit]"',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec add-background login "As a user\\nI want to login securely\\nSo that I can access my account"',
      description: 'Add user story to login feature',
      output: '✓ Added background to login',
    },
    {
      command: 'fspec add-background user-auth "As a developer\\nI want to implement JWT authentication\\nSo that users can access protected resources"',
      description: 'Add background to user-auth feature',
      output: '✓ Added background to user-auth',
    },
    {
      command: 'fspec add-background spec/features/api.feature "As an API consumer\\nI want RESTful endpoints\\nSo that I can integrate with the system"',
      description: 'Add background using full feature path',
      output: '✓ Added background to spec/features/api.feature',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Background text cannot be empty',
      fix: 'Provide text for the Background section as the second argument',
    },
    {
      error: 'Error: Feature file not found: login',
      fix: 'Ensure the feature file exists in spec/features/ directory. Create with: fspec create-feature login',
    },
    {
      error: 'Error: Invalid Gherkin syntax in feature file: ...',
      fix: 'Feature file has syntax errors. Run: fspec validate spec/features/<feature>.feature',
    },
  ],
  typicalWorkflow:
    '1. Create feature: fspec create-feature login → 2. Add background: fspec add-background login "As a user..." → 3. Add scenarios → 4. Validate: fspec validate',
  commonPatterns: [
    {
      pattern: 'User Story Format',
      example:
        'fspec add-background login "As a [role]\\nI want to [action]\\nSo that [benefit]"',
    },
    {
      pattern: 'Update Existing Background',
      example:
        '# Running add-background again replaces the existing Background\nfspec add-background login "As a premium user\\nI want to login with SSO\\nSo that I can use corporate credentials"',
    },
  ],
  relatedCommands: ['create-feature', 'show-feature', 'add-scenario', 'validate'],
  notes: [
    'Background sections appear before all scenarios in a feature',
    'If a Background already exists, it will be replaced',
    'Background title is always "User Story"',
    'Use \\n for multi-line user stories in the command',
    'The feature file is validated before and after modification',
  ],
};

export default config;

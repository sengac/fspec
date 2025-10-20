import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-step',
  description:
    'Update step text or keyword in a scenario by finding and replacing the current step',
  usage: 'fspec update-step <feature> <scenario> <current-step> [options]',
  whenToUse:
    'Use this command to modify a step in a scenario, either changing its text, keyword (Given/When/Then/And/But), or both. The command finds the step by matching the current text and validates that the result is still valid Gherkin.',
  prerequisites: [
    'Feature file must exist',
    'Scenario must exist in the feature file',
    'Current step text must match exactly (with or without keyword)',
  ],
  arguments: [
    {
      name: 'feature',
      description:
        'Feature file name or path (e.g., "user-login" or "spec/features/user-login.feature")',
      required: true,
    },
    {
      name: 'scenario',
      description: 'Scenario name (must match exactly)',
      required: true,
    },
    {
      name: 'current-step',
      description:
        'Current step text to find (with or without keyword, e.g., "Given I am on the login page" or "I am on the login page")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--text <text>',
      description:
        'New step text (without keyword, or with keyword to override)',
    },
    {
      flag: '--keyword <keyword>',
      description: 'New step keyword (Given, When, Then, And, But)',
    },
  ],
  examples: [
    {
      command:
        'fspec update-step user-login "Valid user login" "Given I am on the login page" --text "I navigate to the login page"',
      description: 'Update step text, keep keyword',
      output:
        "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
    },
    {
      command:
        'fspec update-step user-login "Valid user login" "Given I am logged out" --keyword When',
      description: 'Change step keyword from Given to When',
      output:
        "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
    },
    {
      command:
        'fspec update-step user-login "Valid user login" "I enter credentials" --text "I submit the login form" --keyword When',
      description: 'Update both text and keyword',
      output:
        "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
    },
    {
      command:
        'fspec update-step spec/features/auth/login.feature "Login scenario" "Given I am logged in" --text "the user is authenticated"',
      description: 'Update using full path',
      output:
        "✓ Successfully updated step in scenario 'Login scenario' in login.feature",
    },
  ],
  commonErrors: [
    {
      error: 'No updates specified. Use --text and/or --keyword',
      fix: 'Provide at least one of --text or --keyword options',
    },
    {
      error: 'Feature file not found: spec/features/user-login.feature',
      fix: 'Check feature name or provide full path. Use: fspec list-features',
    },
    {
      error: "Scenario 'Login scenario' not found in feature file",
      fix: 'Verify scenario name matches exactly. Use: fspec show-feature <feature>',
    },
    {
      error:
        "Step 'Given I am logged in' not found in scenario 'Login scenario'",
      fix: 'Check step text matches exactly (case-sensitive). Use: fspec show-feature <feature>',
    },
    {
      error: 'Update would result in invalid Gherkin: ...',
      fix: 'Ensure new step text follows Gherkin syntax rules (e.g., starts with keyword, valid format)',
    },
  ],
  typicalWorkflow:
    '1. View scenario steps: fspec show-feature <feature> → 2. Update step: fspec update-step <feature> <scenario> <current-step> --text <new-text> → 3. Verify change: fspec show-feature <feature> → 4. Validate: fspec validate',
  commonPatterns: [
    {
      pattern: 'Refine step wording',
      example:
        '# Original step is unclear\nfspec update-step user-auth "Login" "I log in" --text "I submit valid credentials"\n\n# Verify change\nfspec show-feature user-auth',
    },
    {
      pattern: 'Change step type',
      example:
        '# Convert Given to When for better flow\nfspec update-step checkout "Purchase" "Given I complete checkout" --keyword When\n\n# Result: "When I complete checkout"',
    },
    {
      pattern: 'Batch update multiple steps',
      example:
        '# Update multiple steps in a scenario\nfspec update-step login "Valid login" "I enter email" --text "I enter my email address"\nfspec update-step login "Valid login" "I click login" --text "I submit the login form"\nfspec validate',
    },
  ],
  relatedCommands: ['add-step', 'delete-step', 'show-feature', 'validate'],
  notes: [
    'Current step can be specified with or without keyword (e.g., "Given I am logged in" or "I am logged in")',
    'Step matching is case-sensitive and whitespace-sensitive',
    'The command validates that the updated step results in valid Gherkin before saving',
    'Preserves indentation and formatting of the feature file',
    'If --text includes a keyword (e.g., "When I submit"), only the text part is used (keyword comes from --keyword)',
    'Must provide at least one of --text or --keyword',
  ],
};

export default config;

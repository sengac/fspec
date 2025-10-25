import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'search-scenarios',
  description: 'Search for scenarios across all feature files by text or regex pattern',
  usage: 'fspec search-scenarios --query=<pattern> [options]',
  options: [
    {
      flag: '--query <pattern>',
      description: 'Search pattern (literal text or regex)',
      required: true,
    },
    {
      flag: '--regex',
      description: 'Enable regex pattern matching (default: literal)',
    },
    {
      flag: '--json',
      description: 'Output results in JSON format',
    },
  ],
  whenToUse: [
    'Finding scenarios with specific keywords across multiple features',
    'Locating test scenarios that mention particular functionality',
    'Searching for scenarios related to a specific component or feature',
    'Pattern-based scenario discovery for refactoring or analysis',
  ],
  examples: [
    {
      command: 'fspec search-scenarios --query="validation"',
      description: 'Find scenarios containing "validation"',
      output: `┌────────────────────────────────────────────────────────────┐
│ Scenario                    │ Feature File              │
├────────────────────────────────────────────────────────────┤
│ Validate user input         │ user-registration.feature │
│ Validation error messages   │ form-validation.feature   │
└────────────────────────────────────────────────────────────┘`,
    },
    {
      command: 'fspec search-scenarios --query="valid.*" --regex',
      description: 'Find scenarios matching regex pattern',
      output: `Found 15 scenarios matching pattern: valid.*
  - Validate user credentials
  - Valid email format check
  - Validation workflow`,
    },
    {
      command: 'fspec search-scenarios --query="login" --json',
      description: 'Output in JSON format',
      output: `{
  "scenarios": [
    {
      "name": "Login with valid credentials",
      "featureFile": "spec/features/user-auth.feature",
      "workUnitId": "AUTH-001"
    }
  ]
}`,
    },
  ],
  commonPatterns: [
    {
      title: 'Find authentication-related scenarios',
      commands: [
        'fspec search-scenarios --query="auth"',
        'fspec search-scenarios --query="login|logout|signin" --regex',
      ],
    },
    {
      title: 'Find error handling scenarios',
      commands: [
        'fspec search-scenarios --query="error"',
        'fspec search-scenarios --query="fail|invalid|wrong" --regex',
      ],
    },
  ],
  relatedCommands: ['get-scenarios', 'search-implementation', 'compare-implementations'],
  notes: [
    'Searches scenario names only (not step text)',
    'Regex mode uses JavaScript RegExp syntax',
    'Case-insensitive by default',
    'Results include feature file path and work unit ID for traceability',
  ],
};

export default config;

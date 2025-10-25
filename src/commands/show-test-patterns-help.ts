import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-test-patterns',
  description: 'Analyze and display common testing patterns across work units',
  usage: 'fspec show-test-patterns --tag=<tag> [options]',
  options: [
    {
      flag: '--tag <tag>',
      description: 'Filter work units by tag (e.g., @high, @cli)',
      required: true,
    },
    {
      flag: '--include-coverage',
      description: 'Include test file paths and coverage information',
    },
    {
      flag: '--json',
      description: 'Output results in JSON format',
    },
  ],
  whenToUse: [
    'Identifying common testing patterns across similar features',
    'Ensuring test consistency for work units with same tag',
    'Finding test gaps or inconsistencies',
    'Discovering reusable test utilities or helpers',
  ],
  examples: [
    {
      command: 'fspec show-test-patterns --tag=@high',
      description: 'Show testing patterns for high-priority features',
      output: `Testing Patterns for @high tagged work units:

Common Patterns:
  ✓ All use Vitest framework
  ✓ All include integration tests
  ✓ 90% use beforeEach/afterEach setup
  ✓ 85% use custom test utilities

Test Structure:
  - Average scenarios per feature: 5.2
  - Average test file size: 250 lines
  - Coverage: 95% average`,
    },
    {
      command: 'fspec show-test-patterns --tag=@cli --include-coverage',
      description: 'Show patterns with coverage details',
      output: `Testing Patterns for @cli commands:

CLI-001: Create Feature
  Tests: src/__tests__/create-feature.test.ts (45-120)
  Pattern: Commander.js + Vitest + temp directories

CLI-002: Validate
  Tests: src/__tests__/validate.test.ts (23-89)
  Pattern: Commander.js + Vitest + temp directories

✓  Consistent Patterns Detected:
  - All CLI tests use temporary directories for isolation
  - All use Commander.js for argument parsing
  - All tests clean up temp files in afterEach`,
    },
    {
      command: 'fspec show-test-patterns --tag=@authentication --json',
      description: 'Output patterns in JSON format',
      output: `{
  "tag": "@authentication",
  "workUnits": 5,
  "patterns": {
    "framework": "Vitest",
    "commonUtilities": ["setupTestUser", "mockAuth"],
    "averageCoverage": 92
  },
  "testFiles": [...]
}`,
    },
  ],
  commonPatterns: [
    {
      title: 'Review CLI test consistency',
      commands: [
        'fspec show-test-patterns --tag=@cli --include-coverage',
      ],
    },
    {
      title: 'Analyze critical feature testing',
      commands: [
        'fspec show-test-patterns --tag=@critical',
        'fspec show-test-patterns --tag=@high',
      ],
    },
  ],
  relatedCommands: ['show-coverage', 'compare-implementations', 'search-scenarios'],
  notes: [
    'Analyzes test files from coverage data',
    'Identifies common testing patterns and utilities',
    'Helps maintain test consistency across similar features',
    'Detects outliers or inconsistent testing approaches',
  ],
};

export default config;

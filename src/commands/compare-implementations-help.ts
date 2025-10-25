import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'compare-implementations',
  description: 'Compare implementation approaches across work units to identify patterns and inconsistencies',
  usage: 'fspec compare-implementations --tag=<tag> [options]',
  options: [
    {
      flag: '--tag <tag>',
      description: 'Filter work units by tag (e.g., @authentication, @cli)',
      required: true,
    },
    {
      flag: '--show-coverage',
      description: 'Include test and implementation file paths from coverage data',
    },
    {
      flag: '--json',
      description: 'Output results in JSON format',
    },
  ],
  whenToUse: [
    'Reviewing implementation consistency across similar features',
    'Identifying naming convention differences',
    'Detecting architectural pattern divergence',
    'Finding opportunities for code reuse or refactoring',
  ],
  examples: [
    {
      command: 'fspec compare-implementations --tag=@authentication',
      description: 'Compare authentication implementations',
      output: `Comparing 5 work units tagged with @authentication:

AUTH-001: User Login
  Pattern: JWT-based authentication
  Naming: camelCase

AUTH-002: OAuth Integration
  Pattern: Token-based authentication
  Naming: camelCase

⚠️  Naming Convention Differences:
  - AUTH-003 uses snake_case instead of camelCase
  - Consider standardizing on camelCase

✓  Architectural Consistency:
  - All use token-based auth pattern
  - Consistent error handling approach`,
    },
    {
      command: 'fspec compare-implementations --tag=@cli --show-coverage',
      description: 'Compare with coverage information',
      output: `CLI-001: Create Feature Command
  Tests: src/__tests__/create-feature.test.ts
  Implementation: src/commands/create-feature.ts
  Pattern: Commander.js with async/await

CLI-002: Validate Command
  Tests: src/__tests__/validate.test.ts
  Implementation: src/commands/validate.ts
  Pattern: Commander.js with async/await

✓  Consistent test and implementation patterns across all CLI commands`,
    },
  ],
  commonPatterns: [
    {
      title: 'Find inconsistencies in CLI commands',
      commands: [
        'fspec compare-implementations --tag=@cli --show-coverage',
      ],
    },
    {
      title: 'Review authentication approaches',
      commands: [
        'fspec compare-implementations --tag=@authentication',
        'fspec compare-implementations --tag=@security',
      ],
    },
  ],
  relatedCommands: ['search-implementation', 'show-test-patterns', 'search-scenarios'],
  notes: [
    'Automatically detects naming convention differences (camelCase, snake_case, kebab-case)',
    'Highlights architectural pattern divergence',
    'Uses coverage files to analyze actual implementation',
    'Side-by-side comparison helps identify best practices',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-coverage',
  description:
    'Display coverage report showing scenario-to-test-to-implementation traceability',
  usage: 'fspec show-coverage [feature-name] [options]',
  whenToUse:
    'Use this command to view coverage status for features, identify uncovered scenarios (gaps), verify traceability links, and track reverse ACDD mapping progress. Run regularly to find scenarios that need test/implementation links.',
  arguments: [
    {
      name: 'feature-name',
      description:
        'Feature file name (without path or extension), e.g., "user-authentication". If omitted, shows project-wide coverage.',
      required: false,
    },
  ],
  options: [
    {
      flag: '--format <format>',
      description: 'Output format: text or json (default: text)',
    },
    {
      flag: '--output <file>',
      description: 'Write output to file instead of stdout',
    },
  ],
  examples: [
    {
      command: 'fspec show-coverage user-authentication',
      description: 'Show coverage for specific feature',
      output:
        'Coverage Report: user-authentication.feature\nCoverage: 50% (1/2 scenarios)\n\n## Scenarios\n### ✅ Login with valid credentials (FULLY COVERED)\n- **Test**: src/__tests__/auth.test.ts:45-62\n- **Implementation**: src/auth/login.ts:10,11,12,23,24\n\n### ❌ Login with invalid credentials (NOT COVERED)\n- No test mappings',
    },
    {
      command: 'fspec show-coverage',
      description: 'Show project-wide coverage report',
      output:
        'Project Coverage Report\nOverall Coverage: 65% (13/20 scenarios)\n\nFeatures Overview:\n- user-authentication.feature: 50% (1/2) ⚠️\n- user-logout.feature: 100% (1/1) ✅\n- dashboard.feature: 67% (2/3) ⚠️\n- admin-tools.feature: 100% (4/4) ✅',
    },
    {
      command: 'fspec show-coverage user-authentication --format=json',
      description: 'Get coverage in JSON format for programmatic access',
      output:
        '{\n  "feature": "user-authentication",\n  "totalScenarios": 2,\n  "coveredScenarios": 1,\n  "coveragePercent": 50,\n  "scenarios": [...]\n}',
    },
    {
      command: 'fspec show-coverage --output=coverage-report.txt',
      description: 'Save coverage report to file',
      output: '✓ Coverage report written to coverage-report.txt',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Feature file user-authentication.feature not found',
      fix: 'Check feature name matches file name. Run: fspec list-features',
    },
    {
      error:
        'Error: Coverage file user-authentication.feature.coverage not found',
      fix: 'Coverage file should auto-create with fspec create-feature. Try running create-feature again.',
    },
  ],
  typicalWorkflow:
    '1. fspec show-coverage → 2. Identify uncovered scenarios (gaps) → 3. Link tests/implementation → 4. Verify coverage increased',
  commonPatterns: [
    {
      pattern: 'Find Coverage Gaps',
      example:
        '# See project-wide coverage\nfspec show-coverage\n\n# Focus on specific feature\nfspec show-coverage user-authentication\n\n# Identify scenarios marked ❌ NOT COVERED\n# Link missing tests/implementation',
    },
    {
      pattern: 'Reverse ACDD Progress Tracking',
      example:
        '# Check what percentage of scenarios are mapped\nfspec show-coverage\n# Output: Overall Coverage: 20% (4/20 scenarios)\n\n# Map more scenarios...\nfspec link-coverage ...\n\n# Check progress\nfspec show-coverage\n# Output: Overall Coverage: 45% (9/20 scenarios)',
    },
    {
      pattern: 'Export Coverage for CI/CD',
      example:
        "# Export as JSON for automated checks\nfspec show-coverage --format=json --output=coverage.json\n\n# Parse in CI pipeline to enforce coverage thresholds\njq '.coveragePercent' coverage.json",
    },
  ],
  relatedCommands: [
    'link-coverage',
    'audit-coverage',
    'unlink-coverage',
    'list-features',
  ],
  notes: [
    'Coverage symbols: ✅ FULLY COVERED, ⚠️ PARTIALLY COVERED (test only), ❌ NOT COVERED',
    'Project-wide report (no args) shows overall coverage percentage',
    'Use --format=json for programmatic access in CI/CD pipelines',
    'Coverage percentages based on scenarios, not lines of code',
    'Run regularly to track reverse ACDD mapping progress',
  ],
};

export default config;

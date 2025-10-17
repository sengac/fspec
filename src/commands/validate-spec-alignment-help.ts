import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-spec-alignment',
  description:
    'Validate alignment between feature specifications, tests, and implementation',
  usage: 'fspec validate-spec-alignment [feature-files...] [options]',
  whenToUse:
    'Use in the validating phase to ensure specifications, tests, and implementation are aligned. This command checks that scenarios have corresponding tests and implementation, helping maintain ACDD discipline.',
  prerequisites: [
    'Feature files must exist in spec/features/',
    'Work units should be tagged in scenarios with @WORK-UNIT-ID',
  ],
  arguments: [
    {
      name: 'feature-files',
      description:
        'Feature files to validate (default: all in spec/features). Can be file paths or feature names.',
      required: false,
    },
  ],
  options: [
    {
      flag: '--fix',
      description:
        'Automatically fix alignment issues (e.g., create missing coverage mappings)',
    },
  ],
  examples: [
    {
      command: 'fspec validate-spec-alignment',
      description: 'Validate alignment for all feature files',
      output:
        '✓ All specs are aligned with tests and implementation\n\nAlignment summary:\n  Features checked: 12\n  Scenarios checked: 45\n  Scenarios with tests: 45\n  Test coverage: 100%',
    },
    {
      command: 'fspec validate-spec-alignment spec/features/auth/login.feature',
      description: 'Validate specific feature file',
      output: '✓ All specs are aligned with tests and implementation',
    },
    {
      command:
        'fspec validate-spec-alignment spec/features/auth/*.feature --fix',
      description: 'Validate and auto-fix alignment issues',
      output:
        '✓ Fixed 2 alignment issues\n✓ All specs are aligned with tests and implementation',
    },
    {
      command: 'fspec validate-spec-alignment',
      description: 'Example with alignment issues found',
      output:
        '✗ Found 3 alignment issues\n  - Scenario "Valid user login" has no test mapping\n  - Scenario "Invalid credentials" test has no implementation\n  - Feature file "checkout.feature" has no @WORK-UNIT-ID tags',
    },
  ],
  commonErrors: [
    {
      error: 'Found 3 alignment issues',
      fix: 'Review listed issues and either:\n1. Add test mappings: fspec link-coverage <feature> --scenario "<name>" --test-file <file> --test-lines <range>\n2. Add implementation: fspec link-coverage <feature> --scenario "<name>" --test-file <file> --impl-file <file> --impl-lines <lines>\n3. Tag scenarios: fspec add-tag-to-scenario <feature> "<scenario>" @WORK-UNIT-ID',
    },
    {
      error: 'Feature file not found: spec/features/nonexistent.feature',
      fix: 'Check feature file path or use: fspec list-features',
    },
  ],
  typicalWorkflow:
    '1. Complete implementation → 2. Link coverage: fspec link-coverage <feature> → 3. Validate alignment: fspec validate-spec-alignment → 4. Fix any issues → 5. Move to done: fspec update-work-unit-status <id> done',
  commonPatterns: [
    {
      pattern: 'Validate before moving to done',
      example:
        '# Check alignment before marking work unit done\nfspec validate-spec-alignment\n\n# If issues found, fix them\nfspec link-coverage user-auth --scenario "Valid login" --test-file src/__tests__/auth.test.ts --test-lines 10-25\n\n# Re-validate\nfspec validate-spec-alignment\n\n# Move to done\nfspec update-work-unit-status AUTH-001 done',
    },
    {
      pattern: 'Auto-fix alignment issues',
      example:
        '# Run with --fix to automatically resolve issues\nfspec validate-spec-alignment --fix\n\n# Review changes\nfspec show-coverage user-auth',
    },
    {
      pattern: 'Validate specific epic',
      example:
        '# Validate all features in an epic\nfspec list-features --epic AUTH | xargs fspec validate-spec-alignment',
    },
  ],
  relatedCommands: [
    'validate',
    'link-coverage',
    'show-coverage',
    'audit-coverage',
    'update-work-unit-status',
  ],
  notes: [
    'Checks that feature scenarios have corresponding test mappings',
    'Verifies that tests have implementation mappings',
    'Ensures scenarios are tagged with work unit IDs for traceability',
    '--fix option attempts to automatically resolve common issues',
    'Exit code 1 if alignment issues found (suitable for CI/CD)',
    'Part of ACDD workflow - run before moving to done state',
  ],
};

export default config;

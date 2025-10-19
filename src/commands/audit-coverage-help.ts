import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'audit-coverage',
  description:
    'Verify that test files and implementation files referenced in coverage mappings actually exist',
  usage: 'fspec audit-coverage <feature-name> [options]',
  whenToUse:
    'Use this command after refactoring to verify file paths are still correct, or periodically to detect broken coverage links. Essential before deploying or after moving files. Catches issues where tests/implementation were deleted but coverage file was not updated.',
  arguments: [
    {
      name: 'feature-name',
      description:
        'Feature file name (without path or extension), e.g., "user-authentication"',
      required: true,
    },
  ],
  options: [
    {
      flag: '--fix',
      description: 'Automatically remove broken mappings from coverage file',
    },
  ],
  examples: [
    {
      command: 'fspec audit-coverage user-authentication',
      description: 'Audit coverage for specific feature',
      output:
        'Auditing: user-authentication.feature\n\n✓ Login with valid credentials\n  ✓ Test file exists: src/__tests__/auth.test.ts\n  ✓ Implementation file exists: src/auth/login.ts\n\n✗ Login with invalid credentials\n  ✗ Test file missing: src/__tests__/auth-error.test.ts\n  ✓ Implementation file exists: src/auth/login.ts\n\nAudit Summary:\n- 2 scenarios checked\n- 1 fully valid\n- 1 has broken links\n\nRun with --fix to remove broken mappings',
    },
    {
      command: 'fspec audit-coverage user-authentication --fix',
      description: 'Audit and automatically fix broken links',
      output:
        'Auditing: user-authentication.feature\n\n✗ Login with invalid credentials\n  ✗ Test file missing: src/__tests__/auth-error.test.ts\n  Removed broken test mapping\n\n✓ Fixed 1 broken mapping',
    },
    {
      command: 'fspec audit-coverage dashboard',
      description: 'Audit coverage after moving files',
      output:
        'Auditing: dashboard.feature\n\n✓ View dashboard metrics\n  ✓ Test file exists: src/__tests__/dashboard.test.ts\n  ✓ Implementation file exists: src/dashboard/metrics.ts\n\nAll mappings valid! ✅',
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
      fix: 'Coverage file should auto-create with fspec create-feature.',
    },
  ],
  typicalWorkflow:
    '1. Refactor/move files → 2. fspec audit-coverage → 3. Fix broken paths manually OR use --fix → 4. Verify with fspec show-coverage',
  commonPatterns: [
    {
      pattern: 'After Refactoring',
      example:
        '# Moved test files to new directory structure\nmv src/__tests__/auth/* src/auth/__tests__/\n\n# Audit all features to find broken links\nfspec list-features --format=json | jq -r \'.[].name\' | while read feature; do\n  fspec audit-coverage "$feature"\ndone',
    },
    {
      pattern: 'Fix Broken Links Automatically',
      example:
        '# After deleting obsolete test files\nfspec audit-coverage user-authentication\n# Shows broken links\n\n# Auto-fix (removes broken mappings)\nfspec audit-coverage user-authentication --fix',
    },
    {
      pattern: 'CI/CD Validation',
      example:
        '# In CI pipeline, fail if any coverage links are broken\nfspec audit-coverage user-authentication || exit 1',
    },
  ],
  relatedCommands: ['show-coverage', 'link-coverage', 'unlink-coverage'],
  notes: [
    'Does NOT validate that line numbers are correct, only that files exist',
    'Use --fix to automatically remove broken mappings (backup first!)',
    'Run after moving or renaming files to catch stale references',
    'Essential part of refactoring workflow to maintain coverage integrity',
    'Does not modify feature files, only .coverage files',
  ],
};

export default config;

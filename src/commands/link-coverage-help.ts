import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'link-coverage',
  description:
    'Link Gherkin scenarios to test files and implementation code for full traceability',
  usage: 'fspec link-coverage <feature-name> [options]',
  whenToUse:
    'Use this command IMMEDIATELY after writing tests or implementation code to maintain scenario-to-test-to-code traceability. CRITICAL for reverse ACDD to track what has been mapped and what remains. Essential for refactoring safety and gap detection.',
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
      flag: '--scenario <name>',
      description: 'Scenario name to link (required)',
    },
    {
      flag: '--test-file <path>',
      description:
        'Test file path (required for linking test, e.g., "src/__tests__/auth.test.ts")',
    },
    {
      flag: '--test-lines <range>',
      description:
        'Test line range (e.g., "45-62" or "45,46,47"). Used when linking test file.',
    },
    {
      flag: '--impl-file <path>',
      description:
        'Implementation file path (e.g., "src/auth/login.ts"). Used to link implementation.',
    },
    {
      flag: '--impl-lines <lines>',
      description:
        'Implementation line numbers (e.g., "10,11,12,23,24"). Used with --impl-file.',
    },
    {
      flag: '--skip-validation',
      description:
        'Skip file path validation (useful for skeleton tests in reverse ACDD)',
    },
    {
      flag: '--skip-step-validation',
      description:
        'Skip step comment validation (ONLY allowed for task work units - story and bug work units require MANDATORY step validation)',
    },
  ],
  examples: [
    {
      command:
        'fspec link-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --test-lines 45-62',
      description: 'Link test file to scenario (after writing tests)',
      output:
        '✓ Linked test to scenario "Login with valid credentials"\n  Test: src/__tests__/auth.test.ts:45-62',
    },
    {
      command:
        'fspec link-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10-24',
      description:
        'Link implementation to existing test mapping (after implementing)',
      output:
        '✓ Linked implementation to scenario "Login with valid credentials"\n  Test: src/__tests__/auth.test.ts:45-62\n  Implementation: src/auth/login.ts:10,11,12,23,24',
    },
    {
      command:
        'fspec link-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --test-lines 45-62 --impl-file src/auth/login.ts --impl-lines 10-24',
      description: 'Link both test and implementation at once',
      output:
        '✓ Linked test and implementation to scenario "Login with valid credentials"\n  Test: src/__tests__/auth.test.ts:45-62\n  Implementation: src/auth/login.ts:10,11,12,23,24',
    },
    {
      command:
        'fspec link-coverage user-login --scenario "Login with valid credentials" --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 --skip-validation',
      description:
        'Link skeleton test in reverse ACDD (use --skip-validation for unimplemented tests)',
      output:
        '✓ Linked skeleton test to scenario "Login with valid credentials" (validation skipped)\n  Test: src/__tests__/auth-login.test.ts:13-27',
    },
  ],
  commonErrors: [
    {
      error:
        'Error: Scenario "Login with valid credentials" not found in feature',
      fix: 'Check scenario name matches exactly. Run: fspec show-feature user-authentication',
    },
    {
      error: 'Error: Test file src/__tests__/auth.test.ts does not exist',
      fix: 'Verify file path is correct. Use --skip-validation for reverse ACDD skeleton tests.',
    },
    {
      error: 'Error: Cannot link implementation without existing test mapping',
      fix: 'Link test file first, then link implementation to that test mapping.',
    },
    {
      error: 'Step validation failed: Missing step comment "When I click the login button"',
      fix: 'Add step comments to test file: // @step When I click the login button\nNote: --skip-step-validation is ONLY allowed for task work units',
    },
  ],
  typicalWorkflow:
    '1. Write tests (red phase) → 2. fspec link-coverage --test-file → 3. Implement code (green phase) → 4. fspec link-coverage --impl-file → 5. Verify with fspec show-coverage',
  commonPatterns: [
    {
      pattern: 'Forward ACDD Workflow',
      example:
        '# After writing tests\nnpm test  # Tests fail (red)\nfspec link-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --test-lines 45-62\n\n# After implementing\nnpm test  # Tests pass (green)\nfspec link-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10-24',
    },
    {
      pattern: 'Reverse ACDD Workflow',
      example:
        '# Link skeleton test (not implemented yet)\nfspec link-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts --test-lines 13-27 --skip-validation\n\n# Link existing implementation\nfspec link-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/routes/auth.ts --impl-lines 45-67 --skip-validation',
    },
  ],
  relatedCommands: [
    'show-coverage',
    'audit-coverage',
    'unlink-coverage',
    'create-feature',
  ],
  notes: [
    'Coverage files (.feature.coverage) are auto-created by fspec create-feature',
    'ALWAYS link coverage immediately after writing tests or code (do not batch)',
    'Use --skip-validation in reverse ACDD for skeleton tests and forward planning',
    'Coverage tracking is CRITICAL for reverse ACDD to track mapping progress',
    'Line ranges: "45-62" for range, "10,11,12,23,24" for specific lines',
    'Step validation: Test files must include step comments (// @step Given...) matching feature file steps',
    'Step comments support both @step prefix (recommended) and plain format (backward compatible)',
    'Parameterized steps ({int}, {string}) match via hybrid similarity algorithm',
    'Step validation fails with helpful system-reminder showing exact text to add to test file',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'unlink-coverage',
  description:
    'Remove test or implementation links from scenario coverage mappings',
  usage: 'fspec unlink-coverage <feature-name> [options]',
  whenToUse:
    'Use this command when tests or implementation are deleted, refactored, or when coverage mappings are incorrect and need to be reset. Useful before re-linking with correct file paths.',
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
      description: 'Scenario name to unlink (required)',
    },
    {
      flag: '--test-file <path>',
      description:
        'Test file path to remove. If specified with --impl-file, removes only impl mapping.',
    },
    {
      flag: '--impl-file <path>',
      description:
        'Implementation file path to remove. Removes implementation mapping from test.',
    },
    {
      flag: '--all',
      description: 'Remove all mappings for the scenario (reset to uncovered)',
    },
  ],
  examples: [
    {
      command:
        'fspec unlink-coverage user-authentication --scenario "Login with valid credentials" --all',
      description: 'Remove all coverage mappings for a scenario',
      output:
        '✓ Removed all coverage mappings for scenario "Login with valid credentials"',
    },
    {
      command:
        'fspec unlink-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts',
      description: 'Remove entire test mapping (including implementation links)',
      output:
        '✓ Removed test mapping src/__tests__/auth.test.ts from scenario "Login with valid credentials"',
    },
    {
      command:
        'fspec unlink-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts',
      description: 'Remove only implementation mapping, keep test mapping',
      output:
        '✓ Removed implementation src/auth/login.ts from test mapping src/__tests__/auth.test.ts',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Scenario "Login with valid credentials" not found in feature',
      fix: 'Check scenario name matches exactly. Run: fspec show-feature user-authentication',
    },
    {
      error:
        'Error: Test file src/__tests__/auth.test.ts not found in scenario mappings',
      fix: 'This test is not linked to the scenario. Run: fspec show-coverage user-authentication',
    },
    {
      error: 'Error: Must specify --scenario with --test-file or --impl-file',
      fix: 'Add --scenario flag to specify which scenario to unlink from.',
    },
  ],
  typicalWorkflow:
    '1. Identify incorrect mapping with fspec show-coverage → 2. fspec unlink-coverage → 3. Re-link with correct paths using fspec link-coverage',
  commonPatterns: [
    {
      pattern: 'Reset Scenario Coverage',
      example:
        '# Remove all mappings to start fresh\nfspec unlink-coverage user-auth --scenario "Login" --all\n\n# Re-link with correct paths\nfspec link-coverage user-auth --scenario "Login" --test-file <correct-path> --test-lines <range>',
    },
    {
      pattern: 'Remove Implementation Only',
      example:
        '# Implementation was refactored, need to re-link\nfspec unlink-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/old-login.ts\n\n# Link new implementation\nfspec link-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/new-login.ts --impl-lines 5-15',
    },
    {
      pattern: 'Clean Up After Deletion',
      example:
        '# Test file was deleted\nfspec unlink-coverage user-auth --scenario "Login" --test-file src/__tests__/old-auth.test.ts\n\n# Or use audit-coverage --fix\nfspec audit-coverage user-auth --fix',
    },
  ],
  relatedCommands: ['link-coverage', 'show-coverage', 'audit-coverage'],
  notes: [
    'Use --all to completely reset scenario coverage (makes it uncovered)',
    'Unlinking test file also removes all its implementation mappings',
    'Unlinking only implementation keeps test mapping intact',
    'Consider using audit-coverage --fix instead for batch cleanup',
    'Does not modify feature files or test/implementation files, only .coverage files',
  ],
};

export default config;

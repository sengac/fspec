import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-hooks',
  description:
    'Validate hook configuration and verify that all hook scripts exist',
  usage: 'fspec validate-hooks',
  whenToUse:
    'Use this command to verify that your hook configuration is valid and all hook scripts exist at the specified paths. Essential before relying on hooks for workflow automation.',
  arguments: [],
  options: [],
  examples: [
    {
      command: 'fspec validate-hooks',
      description: 'When all hooks are valid',
      output: '✓ All hooks are valid',
    },
    {
      command: 'fspec validate-hooks',
      description: 'When hook scripts are missing',
      output: `✗ Hook validation failed

Hook command not found: spec/hooks/validate-feature.sh
Hook command not found: spec/hooks/lint.sh

Fix these issues before using hooks.`,
    },
    {
      command: 'fspec validate-hooks',
      description: 'When no hooks are configured',
      output: 'No hooks configured (nothing to validate)',
    },
  ],
  commonErrors: [
    {
      error: 'Hook command not found: spec/hooks/my-hook.sh',
      fix: 'Create the hook script at the specified path or update the hook configuration to point to the correct path. Remember: paths must be relative to project root.',
    },
    {
      error: 'Failed to load hook configuration',
      fix: 'Check that spec/fspec-hooks.json exists and contains valid JSON. Run fspec add-hook to create the file if missing.',
    },
  ],
  typicalWorkflow:
    'Create hook script → chmod +x script → fspec add-hook → fspec validate-hooks → Test hook execution',
  relatedCommands: ['list-hooks', 'add-hook', 'remove-hook'],
  notes: [
    'Validates that all hook scripts exist at specified paths',
    'Does NOT execute hooks (only checks existence)',
    'Does NOT validate hook script syntax or permissions',
    'Exit code 0 = valid, non-zero = errors found',
    'Run after adding or modifying hooks',
  ],
  prerequisites: [
    'Hook configuration file exists (spec/fspec-hooks.json)',
    'At least one hook is configured',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-tags',
  description:
    'Validate that all tags in feature files are registered in spec/tags.json and enforce tag placement rules',
  usage: 'fspec validate-tags',
  whenToUse:
    'Use to ensure all tags used in feature files are properly registered and correctly placed. Validates that work unit ID tags appear only at feature level (not scenario level). Run before committing to catch unregistered tags and placement violations.',
  examples: [
    {
      command: 'fspec validate-tags',
      description: 'Validate all tags',
      output:
        '✓ All tags in spec/features/login.feature are registered\n✓ All tags in spec/features/signup.feature are registered\n\n✓ 2 files passed',
    },
  ],
  commonErrors: [
    {
      error: 'Tag @my-tag is not registered',
      fix: 'Register the tag: fspec register-tag @my-tag "Category" "Description"',
    },
    {
      error:
        'Work unit ID tag @AUTH-001 must be at feature level, not scenario level',
      fix: 'Move the work unit ID tag from scenario-level to feature-level tags. Use coverage files (*.feature.coverage) for fine-grained scenario traceability instead of scenario-level work unit tags.',
    },
  ],
  relatedCommands: ['register-tag', 'list-tags', 'check'],
  notes: [
    'Part of fspec check command',
    'Exit code 0 = all valid, non-zero = unregistered tags found or placement violations',
    'Work unit ID tags (e.g., AUTH-001, COV-005) must be at feature level only',
    'Use coverage files for linking scenarios to implementation (two-tier linking system)',
  ],
};

export default config;

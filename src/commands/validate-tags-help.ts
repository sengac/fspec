import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-tags',
  description: 'Validate that all tags in feature files are registered in spec/tags.json',
  usage: 'fspec validate-tags',
  whenToUse:
    'Use to ensure all tags used in feature files are properly registered. Run before committing to catch unregistered tags.',
  examples: [
    {
      command: 'fspec validate-tags',
      description: 'Validate all tags',
      output: '✓ All tags in spec/features/login.feature are registered\n✓ All tags in spec/features/signup.feature are registered\n\n✓ 2 files passed',
    },
  ],
  commonErrors: [
    {
      error: 'Tag @my-tag is not registered',
      fix: 'Register the tag: fspec register-tag @my-tag "Category" "Description"',
    },
  ],
  relatedCommands: ['register-tag', 'list-tags', 'check'],
  notes: [
    'Part of fspec check command',
    'Exit code 0 = all valid, non-zero = unregistered tags found',
  ],
};

export default config;

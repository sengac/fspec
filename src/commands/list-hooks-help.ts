import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-hooks',
  description: 'List all configured lifecycle hooks',
  usage: 'fspec list-hooks',
  whenToUse:
    'Use this command to see what hooks are configured for your project, including their event names and hook names. Useful for understanding the current automation setup and debugging hook execution.',
  arguments: [],
  options: [],
  examples: [
    {
      command: 'fspec list-hooks',
      description: 'List all configured hooks',
      output: `Configured Hooks:

pre-update-work-unit-status:
  - validate-feature-file
  - check-blockers

post-implementing:
  - run-tests
  - lint-code

post-validating:
  - notify-slack`,
    },
    {
      command: 'fspec list-hooks',
      description: 'When no hooks are configured',
      output: 'No hooks are configured',
    },
  ],
  commonErrors: [
    {
      error: 'No hooks are configured',
      fix: 'This is not an error - it means you have no hooks configured yet. Create spec/fspec-hooks.json to add hooks.',
    },
  ],
  typicalWorkflow:
    'fspec list-hooks → Review configured hooks → fspec validate-hooks → fspec add-hook (if needed)',
  relatedCommands: ['validate-hooks', 'add-hook', 'remove-hook'],
  notes: [
    'Reads from spec/fspec-hooks.json',
    'Shows event names and hook names only (not full configuration)',
    'Use validate-hooks to check if hook scripts exist',
    'Hooks are organized by event (pre-/post- command pattern)',
  ],
};

export default config;

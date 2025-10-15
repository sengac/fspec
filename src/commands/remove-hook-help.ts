import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-hook',
  description: 'Remove a lifecycle hook from the configuration',
  usage: 'fspec remove-hook <event> <name>',
  whenToUse:
    'Use this command to remove a hook from your configuration. Useful when you no longer need a specific automation or want to temporarily disable a hook.',
  arguments: [
    {
      name: 'event',
      description:
        'Event name of the hook to remove (e.g., pre-update-work-unit-status, post-implementing)',
      required: true,
    },
    {
      name: 'name',
      description: 'Name of the hook to remove (e.g., validate-feature, run-tests)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec remove-hook pre-implementing validate-code',
      description: 'Remove a pre-hook',
      output: '✓ Hook removed: pre-implementing/validate-code',
    },
    {
      command: 'fspec remove-hook post-implementing run-tests',
      description: 'Remove a post-hook',
      output: '✓ Hook removed: post-implementing/run-tests',
    },
    {
      command: 'fspec remove-hook pre-update-work-unit-status check-ready',
      description: 'Remove a quality gate hook',
      output: '✓ Hook removed: pre-update-work-unit-status/check-ready',
    },
  ],
  commonErrors: [
    {
      error: 'Hook not found',
      fix: 'Verify the event and hook name are correct using fspec list-hooks. Event and name must match exactly.',
    },
    {
      error: 'Hook configuration file not found',
      fix: 'No hooks are configured. Nothing to remove.',
    },
  ],
  typicalWorkflow:
    'fspec list-hooks → Identify hook to remove → fspec remove-hook <event> <name> → Verify with list-hooks',
  relatedCommands: ['add-hook', 'list-hooks', 'validate-hooks'],
  notes: [
    'Removes hook from spec/fspec-hooks.json',
    'Does NOT delete the hook script file',
    'Event array is removed if it becomes empty',
    'Use fspec list-hooks to see available hooks',
    'Case-sensitive: event and name must match exactly',
  ],
  prerequisites: [
    'Hook configuration file exists (spec/fspec-hooks.json)',
    'Hook exists with the specified event and name',
  ],
};

export default config;

import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-schedule',
  description:
    'Remove a scheduled job from spec/schedules.json. Permanently deletes the schedule entry.',
  usage: 'fspec remove-schedule <name>',
  whenToUse:
    'Use when a scheduled job is no longer needed and should be permanently removed. For temporary disabling, use pause-schedule instead.',
  whenNotToUse:
    'Do not use if you only want to temporarily stop a schedule. Use pause-schedule to suspend it and resume-schedule to reactivate later.',
  arguments: [
    {
      name: 'name',
      description:
        'The name of the schedule to remove (must match an existing schedule)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec remove-schedule nightly-review',
      description: 'Remove the nightly-review schedule',
      output: "✓ Schedule 'nightly-review' removed successfully",
    },
    {
      command: 'fspec remove-schedule daily-tests',
      description: 'Remove the daily-tests schedule',
      output: "✓ Schedule 'daily-tests' removed successfully",
    },
  ],
  prerequisites: [
    'Schedule must exist in spec/schedules.json',
    'spec/schedules.json must exist (created by add-schedule)',
  ],
  typicalWorkflow:
    'fspec list-schedules → identify obsolete schedule → fspec remove-schedule <name> → fspec list-schedules (verify)',
  commonErrors: [
    {
      error: "Schedule 'nightly-review' does not exist",
      fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
    },
  ],
  relatedCommands: [
    'add-schedule - Create a new schedule',
    'list-schedules - List all configured schedules',
    'pause-schedule - Temporarily disable a schedule (non-destructive)',
    'resume-schedule - Re-enable a paused schedule',
  ],
  notes: [
    'Removal is permanent — there is no undo',
    'Consider using pause-schedule if you may need the schedule again',
    'The schedule entry is deleted from spec/schedules.json atomically',
  ],
};

export default config;

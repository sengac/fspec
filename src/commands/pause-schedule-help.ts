import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'pause-schedule',
  description:
    'Pause a scheduled job by setting its status to "paused". The schedule remains in spec/schedules.json but will not trigger until resumed.',
  usage: 'fspec pause-schedule <name>',
  whenToUse:
    'Use when you want to temporarily stop a schedule from triggering without removing it. Useful during maintenance windows, deployments, or when investigating issues.',
  whenNotToUse:
    'Do not use if you want to permanently remove the schedule. Use remove-schedule instead.',
  arguments: [
    {
      name: 'name',
      description:
        'The name of the schedule to pause (must be currently active)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec pause-schedule nightly-review',
      description: 'Pause the nightly-review schedule',
      output: "✓ Schedule 'nightly-review' paused successfully",
    },
    {
      command: 'fspec pause-schedule daily-tests',
      description: 'Pause during a maintenance window',
      output: "✓ Schedule 'daily-tests' paused successfully",
    },
  ],
  prerequisites: [
    'Schedule must exist in spec/schedules.json',
    'Schedule must currently have status "active"',
  ],
  typicalWorkflow:
    'fspec list-schedules → fspec pause-schedule <name> (before maintenance) → perform maintenance → fspec resume-schedule <name> (after maintenance)',
  commonErrors: [
    {
      error: "Schedule 'nightly-review' does not exist",
      fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
    },
    {
      error: "Schedule 'nightly-review' is already paused",
      fix: 'The schedule is already paused. Use resume-schedule to reactivate it.',
    },
  ],
  relatedCommands: [
    'resume-schedule - Resume a paused schedule',
    'list-schedules - List all schedules and their status',
    'remove-schedule - Permanently remove a schedule',
    'add-schedule - Create a new schedule',
  ],
  notes: [
    'Pausing is non-destructive — the schedule configuration is preserved',
    'A paused schedule will not trigger until explicitly resumed',
    'Use list-schedules to verify the schedule status after pausing',
  ],
};

export default config;

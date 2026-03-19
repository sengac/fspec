import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'resume-schedule',
  description:
    'Resume a paused scheduled job by setting its status back to "active". The schedule will begin triggering again according to its cron expression.',
  usage: 'fspec resume-schedule <name>',
  whenToUse:
    'Use after a maintenance window or investigation is complete to re-enable a previously paused schedule.',
  whenNotToUse:
    'Do not use on schedules that are already active. Check status first with list-schedules.',
  arguments: [
    {
      name: 'name',
      description:
        'The name of the schedule to resume (must be currently paused)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec resume-schedule nightly-review',
      description: 'Resume the nightly-review schedule after maintenance',
      output: "✓ Schedule 'nightly-review' resumed successfully",
    },
    {
      command: 'fspec resume-schedule daily-tests',
      description: 'Re-enable daily test runs',
      output: "✓ Schedule 'daily-tests' resumed successfully",
    },
  ],
  prerequisites: [
    'Schedule must exist in spec/schedules.json',
    'Schedule must currently have status "paused"',
  ],
  typicalWorkflow:
    'fspec list-schedules → confirm schedule is paused → fspec resume-schedule <name> → fspec list-schedules (verify active)',
  commonErrors: [
    {
      error: "Schedule 'nightly-review' does not exist",
      fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
    },
    {
      error: "Schedule 'nightly-review' is already active",
      fix: 'The schedule is already active and will trigger normally. No action needed.',
    },
  ],
  relatedCommands: [
    'pause-schedule - Pause an active schedule',
    'list-schedules - List all schedules and their status',
    'add-schedule - Create a new schedule',
    'remove-schedule - Permanently remove a schedule',
  ],
  notes: [
    'Resuming restores the schedule to "active" status',
    'The schedule will trigger on its next matching cron time after resuming',
    'Missed runs during the paused period are not retroactively executed',
  ],
};

export default config;

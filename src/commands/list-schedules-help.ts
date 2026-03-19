import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-schedules',
  description:
    'List all configured scheduled jobs from spec/schedules.json. Shows schedule name, cron expression, timezone, job type, status, and last run information.',
  usage: 'fspec list-schedules [options]',
  whenToUse:
    'Use to review all configured schedules, check their status (active/paused), verify cron expressions, or find schedule names for other schedule commands.',
  arguments: [],
  options: [
    {
      flag: '--json',
      description: 'Output schedule data as JSON instead of a formatted table.',
    },
  ],
  examples: [
    {
      command: 'fspec list-schedules',
      description: 'List all schedules in table format',
      output:
        'Name            Cron          Timezone          Type    Status  Last Run\n' +
        '----------------------------------------------------------------------------------------------------\n' +
        'nightly-review  0 2 * * *     UTC               agent   active  2025-01-15 02:00\n' +
        'daily-tests     30 6 * * 1-5  America/New_York  shell   active  2025-01-15 06:30\n' +
        '\nTotal: 2 schedule(s)',
    },
    {
      command: 'fspec list-schedules --json',
      description: 'List schedules as JSON for programmatic use',
      output:
        '[{"name":"nightly-review","cron":"0 2 * * *","timezone":"UTC",...}]',
    },
    {
      command: 'fspec list-schedules',
      description: 'When no schedules are configured',
      output:
        'No schedules configured.\nUse `fspec add-schedule` to create a schedule.',
    },
  ],
  prerequisites: [],
  typicalWorkflow:
    'fspec list-schedules → review status → fspec pause-schedule / fspec resume-schedule / fspec remove-schedule as needed',
  commonErrors: [
    {
      error: 'No schedules configured',
      fix: "This is not an error. Create a schedule with 'fspec add-schedule'.",
    },
  ],
  relatedCommands: [
    'add-schedule - Create a new schedule',
    'remove-schedule - Remove a schedule',
    'pause-schedule - Pause an active schedule',
    'resume-schedule - Resume a paused schedule',
  ],
  notes: [
    'Returns an empty list (not an error) when no schedules exist',
    'The --json flag is useful for scripting and CI/CD integration',
    'Status column shows "active" (green) or "paused" (yellow)',
    'Last Run shows the timestamp of the most recent execution',
    'Schedules are stored in spec/schedules.json',
  ],
};

export default config;

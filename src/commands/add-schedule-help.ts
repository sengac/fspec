import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-schedule',
  description:
    'Add a new scheduled job to spec/schedules.json. Supports two job types: agent (spawns an AI agent session) and shell (executes a shell command). Schedules use standard 5-field cron expressions with IANA timezones.',
  usage:
    'fspec add-schedule -n <name> -c <cron> -z <timezone> -t <type> [options]',
  whenToUse:
    'Use when you need to set up recurring automated tasks such as nightly code reviews, daily test runs, periodic dependency checks, or scheduled report generation.',
  whenNotToUse:
    'Do not use for one-off tasks. Use lifecycle hooks instead if you need automation triggered by workflow state changes.',
  arguments: [],
  options: [
    {
      flag: '-n, --name <name>',
      description:
        'Schedule name in slug format (lowercase, hyphenated). Must be unique across all schedules.',
    },
    {
      flag: '-c, --cron <expression>',
      description:
        'Standard 5-field cron expression (minute hour day-of-month month day-of-week). Example: "0 2 * * *" for daily at 2 AM.',
    },
    {
      flag: '-z, --timezone <tz>',
      description:
        'IANA timezone string for cron evaluation. Example: "America/New_York", "UTC", "Europe/London".',
    },
    {
      flag: '-t, --type <type>',
      description:
        'Job type: "agent" (AI agent session) or "shell" (command execution).',
    },
    {
      flag: '-r, --role <role>',
      description:
        'Agent role/system prompt (required for agent type). Defines the agent persona.',
    },
    {
      flag: '-p, --prompt <prompt>',
      description:
        'Initial prompt sent to the agent session (required for agent type).',
    },
    {
      flag: '--command <command>',
      description: 'Shell command to execute (required for shell type).',
    },
    {
      flag: '-o, --overlap <policy>',
      description:
        'What to do if the previous run is still active. "skip" (default) drops the run, "queue" waits.',
      defaultValue: 'skip',
    },
  ],
  examples: [
    {
      command:
        'fspec add-schedule -n nightly-review -c "0 2 * * *" -z UTC -t agent -r "Security reviewer" -p "Review src/ for vulnerabilities"',
      description: 'Add an agent schedule that runs nightly at 2 AM UTC',
      output:
        "✓ Schedule 'nightly-review' added successfully\n  Type: agent\n  Cron: 0 2 * * *\n  Timezone: UTC",
    },
    {
      command:
        'fspec add-schedule -n daily-tests -c "30 6 * * 1-5" -z America/New_York -t shell --command "<test-command>"',
      description:
        'Add a shell schedule that runs tests weekdays at 6:30 AM Eastern',
      output:
        "✓ Schedule 'daily-tests' added successfully\n  Type: shell\n  Cron: 30 6 * * 1-5\n  Timezone: America/New_York",
    },
    {
      command:
        'fspec add-schedule -n weekly-deps -c "0 9 * * 1" -z Europe/London -t shell --command "npx depcheck" -o queue',
      description: 'Add a weekly dependency audit with queue overlap policy',
      output:
        "✓ Schedule 'weekly-deps' added successfully\n  Type: shell\n  Cron: 0 9 * * 1\n  Timezone: Europe/London",
    },
  ],
  prerequisites: [
    'Project must be initialized (spec/ directory exists)',
    'Schedule name must be unique (not already in spec/schedules.json)',
    'Cron expression must be valid 5-field standard syntax',
    'Timezone must be a valid IANA timezone string',
  ],
  typicalWorkflow:
    'fspec add-schedule → fspec list-schedules → fspec pause-schedule (if needed) → fspec resume-schedule → fspec remove-schedule (cleanup)',
  commonErrors: [
    {
      error: "Schedule 'nightly-review' already exists",
      fix: "Choose a different name or remove the existing schedule first with 'fspec remove-schedule nightly-review'.",
    },
    {
      error: 'Agent schedules require both role and prompt',
      fix: 'When using -t agent, you must provide both -r/--role and -p/--prompt.',
    },
    {
      error: 'Shell schedules require a command',
      fix: 'When using -t shell, you must provide --command.',
    },
    {
      error: "Invalid schedule name 'My Schedule'",
      fix: "Names must be lowercase, hyphenated slugs (e.g., 'my-schedule', 'nightly-review').",
    },
    {
      error: 'Invalid cron expression',
      fix: 'Use standard 5-field format: minute(0-59) hour(0-23) day(1-31) month(1-12) weekday(0-7).',
    },
  ],
  relatedCommands: [
    'list-schedules - List all configured schedules',
    'remove-schedule - Remove a schedule',
    'pause-schedule - Pause a schedule',
    'resume-schedule - Resume a paused schedule',
    'add-hook - Add lifecycle hooks (event-driven alternative)',
  ],
  notes: [
    'Schedules are stored in spec/schedules.json',
    'The file is created automatically if it does not exist',
    'Agent schedules spawn an AI agent session with the given role and prompt',
    'Shell schedules execute the command in the project root directory',
    'Overlap policy controls concurrent execution: skip (default) or queue',
    'New schedules are created with status "active" by default',
    'Cron expressions use 5-field standard syntax (no seconds field)',
  ],
};

export default config;

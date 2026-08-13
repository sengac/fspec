//! `add-schedule` help configuration — Rust port of
//! `src/commands/add-schedule-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical to
//! `node dist/index.js add-schedule --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-schedule.txt`

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const EXAMPLE_1_OUTPUT: &str =
    "✓ Schedule 'nightly-review' added successfully\n  Type: agent\n  Cron: 0 2 * * *\n  Timezone: UTC";

const EXAMPLE_2_OUTPUT: &str =
    "✓ Schedule 'daily-tests' added successfully\n  Type: shell\n  Cron: 30 6 * * 1-5\n  Timezone: America/New_York";

const EXAMPLE_3_OUTPUT: &str =
    "✓ Schedule 'weekly-deps' added successfully\n  Type: shell\n  Cron: 0 9 * * 1\n  Timezone: Europe/London";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-schedule -n nightly-review -c \"0 2 * * *\" -z UTC -t agent -r \"Security reviewer\" -p \"Review src/ for vulnerabilities\"",
        description: Some("Add an agent schedule that runs nightly at 2 AM UTC"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec add-schedule -n daily-tests -c \"30 6 * * 1-5\" -z America/New_York -t shell --command \"<test-command>\"",
        description: Some("Add a shell schedule that runs tests weekdays at 6:30 AM Eastern"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec add-schedule -n weekly-deps -c \"0 9 * * 1\" -z Europe/London -t shell --command \"npx depcheck\" -o queue",
        description: Some("Add a weekly dependency audit with queue overlap policy"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "-n, --name <name>",
        description: "Schedule name in slug format (lowercase, hyphenated). Must be unique across all schedules.",
        default_value: None,
    },
    CommandOption {
        flag: "-c, --cron <expression>",
        description: "Standard 5-field cron expression (minute hour day-of-month month day-of-week). Example: \"0 2 * * *\" for daily at 2 AM.",
        default_value: None,
    },
    CommandOption {
        flag: "-z, --timezone <tz>",
        description: "IANA timezone string for cron evaluation. Example: \"America/New_York\", \"UTC\", \"Europe/London\".",
        default_value: None,
    },
    CommandOption {
        flag: "-t, --type <type>",
        description: "Job type: \"agent\" (AI agent session) or \"shell\" (command execution).",
        default_value: None,
    },
    CommandOption {
        flag: "-r, --role <role>",
        description: "Agent role/system prompt (required for agent type). Defines the agent persona.",
        default_value: None,
    },
    CommandOption {
        flag: "-p, --prompt <prompt>",
        description: "Initial prompt sent to the agent session (required for agent type).",
        default_value: None,
    },
    CommandOption {
        flag: "--command <command>",
        description: "Shell command to execute (required for shell type).",
        default_value: None,
    },
    CommandOption {
        flag: "-o, --overlap <policy>",
        description: "What to do if the previous run is still active. \"skip\" (default) drops the run, \"queue\" waits.",
        default_value: Some("skip"),
    },
];

const PREREQUISITES: &[&str] = &[
    "Project must be initialized (spec/ directory exists)",
    "Schedule name must be unique (not already in spec/schedules.json)",
    "Cron expression must be valid 5-field standard syntax",
    "Timezone must be a valid IANA timezone string",
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Schedule 'nightly-review' already exists",
        fix: "Choose a different name or remove the existing schedule first with 'fspec remove-schedule nightly-review'.",
    },
    CommonError {
        error: "Agent schedules require both role and prompt",
        fix: "When using -t agent, you must provide both -r/--role and -p/--prompt.",
    },
    CommonError {
        error: "Shell schedules require a command",
        fix: "When using -t shell, you must provide --command.",
    },
    CommonError {
        error: "Invalid schedule name 'My Schedule'",
        fix: "Names must be lowercase, hyphenated slugs (e.g., 'my-schedule', 'nightly-review').",
    },
    CommonError {
        error: "Invalid cron expression",
        fix: "Use standard 5-field format: minute(0-59) hour(0-23) day(1-31) month(1-12) weekday(0-7).",
    },
];

const RELATED: &[&str] = &[
    "list-schedules - List all configured schedules",
    "remove-schedule - Remove a schedule",
    "pause-schedule - Pause a schedule",
    "resume-schedule - Resume a paused schedule",
    "add-hook - Add lifecycle hooks (event-driven alternative)",
];

const NOTES: &[&str] = &[
    "Schedules are stored in spec/schedules.json",
    "The file is created automatically if it does not exist",
    "Agent schedules spawn an AI agent session with the given role and prompt",
    "Shell schedules execute the command in the project root directory",
    "Overlap policy controls concurrent execution: skip (default) or queue",
    "New schedules are created with status \"active\" by default",
    "Cron expressions use 5-field standard syntax (no seconds field)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-schedule",
    description: "Add a new scheduled job to spec/schedules.json. Supports two job types: agent (spawns an AI agent session) and shell (executes a shell command). Schedules use standard 5-field cron expressions with IANA timezones.",
    usage: Some("fspec add-schedule -n <name> -c <cron> -z <timezone> -t <type> [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when you need to set up recurring automated tasks such as nightly code reviews, daily test runs, periodic dependency checks, or scheduled report generation.",
    ),
    when_not_to_use: Some(
        "Do not use for one-off tasks. Use lifecycle hooks instead if you need automation triggered by workflow state changes.",
    ),
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "fspec add-schedule → fspec list-schedules → fspec pause-schedule (if needed) → fspec resume-schedule → fspec remove-schedule (cleanup)",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};

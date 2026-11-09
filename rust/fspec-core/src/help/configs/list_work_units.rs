//! `list-work-units` help configuration (RPC-253, CLI-017).
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! The fixture rust/fspec/tests/fixtures/help/list-work-units.txt pins the
//! rendered text byte for byte.

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const fn opt(flag: &'static str, description: &'static str) -> CommandOption {
    CommandOption {
        flag,
        description,
        default_value: None,
    }
}

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-work-units --status done --done-since 2d --group-by day",
        description: Some("Count the cards done in the last 2 days per local day"),
        output: Some("DAY         COUNT\n2026-11-06  4\n2026-11-07  2\n\n6 work units"),
    },
    CommandExample {
        command: "fspec list-work-units --status testing,specifying --has-open-questions",
        description: Some("Cards in testing or specifying with open questions"),
        output: None,
    },
    CommandExample {
        command: "fspec list-work-units --type bug --created-since 2026-10-30 --status backlog,specifying,testing,implementing,validating,blocked",
        description: Some("Bugs created since 30 October (local time) that are not done"),
        output: None,
    },
    CommandExample {
        command: "fspec list-work-units --blocked-by BUG-123 --format ids",
        description: Some("Ids of the cards blocked by BUG-123"),
        output: Some("BUG-124\nBUG-130"),
    },
    CommandExample {
        command: "fspec list-work-units --has-assumptions --fields id,title,assumptions",
        description: Some("Cards with assumptions (provisional decisions)"),
        output: None,
    },
    CommandExample {
        command: "fspec list-work-units --work-unit AUTH-001 --fields id,openQuestions,assumptions --format json",
        description: Some("One card's open questions and assumptions"),
        output: None,
    },
    CommandExample {
        command: "fspec list-work-units --entered validating --entered-since 2026-11-07 --format ids",
        description: Some("Cards that entered validating since local midnight of 7 November"),
        output: None,
    },
    CommandExample {
        command: "fspec list-work-units --done-since 1w --group-by type --format count",
        description: Some("Cards done in the last week, counted per type"),
        output: Some("bug 5\nstory 3"),
    },
];

const OPTIONS: &[CommandOption] = &[
    opt("-s, --status <status>", "Statuses (comma list): backlog, specifying, testing, implementing, validating, done, blocked"),
    opt("-t, --type <type>", "Types (comma list): story, task, bug"),
    opt("--prefix <prefix>", "Prefixes registered in spec/prefixes.json (comma list)"),
    opt("-e, --epic <epic>", "Epics registered in spec/epics.json (comma list)"),
    opt("--work-unit <workUnitId>", "Cards by id, or inclusive ranges of one prefix (comma list), e.g. BUG-600..BUG-620"),
    opt("-p, --parent <workUnitId>", "Cards whose parent is this card"),
    opt("--has-children", "Cards with child cards"),
    opt("--estimate-min <n>", "Estimate at least n (a card without an estimate never matches)"),
    opt("--estimate-max <n>", "Estimate at most n"),
    opt("--has-estimate", "Cards with an estimate"),
    opt("--no-estimate", "Cards without an estimate"),
    opt("--created-since <time>", "Created at or after <time>"),
    opt("--created-until <time>", "Created before <time> (a date covers that whole local day)"),
    opt("--updated-since <time>", "Updated at or after <time>"),
    opt("--updated-until <time>", "Updated before <time>"),
    opt("--done-since <time>", "Done cards whose last transition into done is at or after <time>"),
    opt("--done-until <time>", "Done cards whose last transition into done is before <time>"),
    opt("--entered <status>", "Cards with a stateHistory transition into <status>"),
    opt("--entered-since <time>", "With --entered: a transition at or after <time>"),
    opt("--entered-until <time>", "With --entered: a transition before <time>"),
    opt("--text <text>", "Case-insensitive substring of the title or description"),
    opt("--title <text>", "Case-insensitive substring of the title"),
    opt("--has-open-questions", "Cards with a question that is neither answered nor deleted"),
    opt("--has-assumptions", "Cards with assumptions (provisional decisions)"),
    opt("--has-rules", "Cards with rules (deleted items do not count)"),
    opt("--no-rules", "Cards without rules"),
    opt("--has-examples", "Cards with examples"),
    opt("--has-attachments", "Cards with attachments"),
    opt("--has-architecture-notes", "Cards with architecture notes"),
    opt("--tag <@tag>", "Cards with a feature file whose Feature line holds the card's @ID and whose Feature line or any scenario carries <@tag>"),
    opt("--depends-on <workUnitId>", "Cards whose dependsOn holds the card (comma list); any: dependsOn is not empty"),
    opt("--blocked-by <workUnitId>", "Cards whose blockedBy holds the card (comma list); any: not empty"),
    opt("--blocks <workUnitId>", "Cards whose blocks holds the card (comma list); any: not empty"),
    opt("--relates-to <workUnitId>", "Cards whose relatesTo holds the card (comma list); any: not empty"),
    opt("--blocked", "Status blocked, or blocked by a card that is not done"),
    opt("--fields <fields>", "Output fields (comma list): id, title, type, status, epic, prefix, description, estimate, parent, children, createdAt, updatedAt, doneAt, enteredAt:<status>, ageInStatus, openQuestions, assumptions, rules, examples, architectureNotes, attachments, dependsOn, blocks, blockedBy, relatesTo, dependencies (default id,type,status,epic,title)"),
    opt("--format <format>", "table (default), ids, json, jsonl, count or csv"),
    opt("--group-by <key>", "status, type, prefix, epic, day or week: counts per group (json: the groups with their cards); day and week use the local time of the query's one time filter"),
    opt("--sort <field>", "Sort by an output field (times chronologically, absent values last)"),
    opt("--desc", "Reverse the --sort order"),
    opt("--limit <n>", "Keep the first n cards after sorting"),
];

const RELATED: &[&str] = &[
    "show-work-unit",
    "board",
    "create-story",
    "create-bug",
    "create-task",
    "update-work-unit-status",
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Unknown status 'donee'. Valid statuses: backlog, specifying, testing, implementing, validating, done, blocked",
        fix: "Use one of the listed values; types, prefixes, epics, fields, formats, group-by values and sort keys are checked the same way",
    },
    CommonError {
        error: "Invalid time 'yesterday' for --created-since",
        fix: "Use YYYY-MM-DD, an ISO-8601 timestamp, a duration back from now (30m, 6h, 2d, 1w) or now",
    },
    CommonError {
        error: "--group-by day needs exactly one kind of time filter (created, updated, done or entered)",
        fix: "Add one time filter, e.g. --done-since 2d, so the day of that time is the group key",
    },
];

const NOTES: &[&str] = &[
    "Filters are exact (no fuzzy matching): a comma list or a repeated flag matches any of its values, and different filters must all match.",
    "A time is a date (YYYY-MM-DD, the start of that day in the local time zone), an ISO-8601 timestamp (local unless it carries Z or an offset), a duration back from now (Nm, Nh, Nd = N x 24 h, Nw) or now. Since is inclusive, until is exclusive.",
    "Times print in the local time zone with its offset; a field the card does not have is left out of its JSON object and empty in table and csv.",
    "Never read or parse spec/work-units.json directly (no jq, grep or scripts): query it with list-work-units and read one card with show-work-unit.",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-work-units",
    description: "Query the board: filter work units by status, type, prefix, epic, ids, time ranges, stateHistory, text, content and links, then pick fields, sort, limit, group and format the result",
    usage: Some("fspec list-work-units [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command for every question about the board: which cards are in a status, what was done or created in a time range, which cards entered a status, which have open questions or assumptions, what blocks what. Use show-work-unit to read one card in full.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: Some(
        "backlog → specifying → testing → implementing → validating → done. Use --status to see each stage, --done-since or --entered for what moved recently.",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};

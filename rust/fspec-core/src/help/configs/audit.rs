//! `audit` help configuration (AUDIT-003).

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--area <name>",
        description: "Check only this area: gherkin, tags, format, coverage or spec (repeatable; default: all). With --start, meaning adds one review step per scenario (it needs tools.test.command) and spec also adds one stale-text review step per feature",
        default_value: None,
    },
    CommandOption {
        flag: "--feature <name>",
        description: "Audit only this feature (repeatable)",
        default_value: None,
    },
    CommandOption {
        flag: "--work-unit <workUnitId>",
        description: "Audit only the work unit's linked features, or else the features tagged with its ID",
        default_value: None,
    },
    CommandOption {
        flag: "--changed",
        description: "Audit only the features the staged changes touch (their feature or coverage file is staged, or they map a staged test or implementation file), reading every file from the git index; the pre-commit hook fspec init installs runs it",
        default_value: None,
    },
    CommandOption {
        flag: "--fix",
        description: "Apply only the exact mechanical repairs (each problem whose command is fspec audit --fix), print each change, then report what is left",
        default_value: None,
    },
    CommandOption {
        flag: "--json",
        description: "Print {problems, exitCode} as JSON",
        default_value: None,
    },
    CommandOption {
        flag: "--start",
        description: "Start an audit session over the scope: one step per problem whose fix kind is guided (its repair needs judgment), in report order, plus the review steps of --area meaning and --area spec. Prints step 1",
        default_value: None,
    },
    CommandOption {
        flag: "--status",
        description: "Show the scope's audit session: every step with its state and the current step",
        default_value: None,
    },
    CommandOption {
        flag: "--continue",
        description: "Finish the current step: a problem step advances only when the audit no longer reports its problem (exit 1 while it does); a review step records when it was attested",
        default_value: None,
    },
    CommandOption {
        flag: "--reset",
        description: "Delete the scope's audit session",
        default_value: None,
    },
    CommandOption {
        flag: "--complete",
        description: "Delete the scope's audit session once every step is done",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec audit",
        description: Some("Audit the whole project"),
        output: Some("coverage drifted user-login \"Log in\" tests/login_test.rs:10-20: no @step comment for 'Then I see the board' in the mapped lines -> fspec audit --fix --feature user-login\n\n1 problem(s) in 1 of 12 feature(s)"),
    },
    CommandExample {
        command: "fspec audit --feature user-login --area coverage",
        description: Some("Audit one feature's coverage"),
        output: Some("No problems found in 1 feature(s)"),
    },
    CommandExample {
        command: "fspec audit --fix --feature user-login",
        description: Some("Apply the exact mechanical repairs, such as relinking a test whose @step block moved or shifting an implementation mapping whose code moved unchanged"),
        output: Some("fixed coverage drifted user-login \"Log in\" tests/login_test.rs:10-14 -> tests/login_test.rs:22-26\nfixed coverage impl-text-changed user-login \"Log in\" src/login.rs:10-24 -> src/login.rs:30-44\n\nNo problems found in 1 feature(s)"),
    },
    CommandExample {
        command: "fspec audit --changed",
        description: Some("Audit what the next commit holds (the commit gate)"),
        output: Some("No problems found in 2 feature(s)"),
    },
    CommandExample {
        command: "fspec audit --start --feature user-login",
        description: Some("Walk the problems that need judgment step by step"),
        output: Some("Audit session started for --feature user-login: 1 step(s)\n\nStep 1 of 1: coverage drifted user-login \"Log in\" tests/login_test.rs:10-20\n  Fact: no @step comment for 'Then I see the board' in the mapped lines; the exact @step block occurs 2 times (tests/login_test.rs:30-34, tests/login_test.rs:60-64)\n  Repair with: fspec link-coverage user-login --scenario \"Log in\" --test-file tests/login_test.rs --test-lines <lines> --replace\n  Decide the values the command needs from the facts above and the files they name; --continue checks this problem again and advances only when the audit no longer reports it.\n\nWhen this step is done, run: fspec audit --continue --feature user-login"),
    },
    CommandExample {
        command: "fspec audit --start --area meaning --work-unit AUTH-001",
        description: Some("Review that each scenario's test needs its implementation (one step per scenario, run in a scratch git worktree)"),
        output: Some("Audit session started for --area meaning --work-unit AUTH-001: 2 step(s)\n\nStep 1 of 2: meaning user-login \"Log in\"\n  Test mappings: tests/login_test.rs:30-34\n  Implementation mappings: src/login.rs:10-24\n  Test command (tools.test.command): cargo test\n  ..."),
    },
    CommandExample {
        command: "fspec audit --continue --feature user-login",
        description: Some("Finish the current step"),
        output: Some("Step 1 of 1 done: coverage drifted user-login \"Log in\" tests/login_test.rs:10-20\n\nEvery step is done: run fspec audit --complete --feature user-login"),
    },
    CommandExample {
        command: "fspec audit --work-unit AUTH-001 --json",
        description: Some("Audit a work unit's features as JSON"),
        output: Some("{\"problems\":[],\"exitCode\":0}"),
    },
];

const RELATED: &[&str] = &[
    "validate",
    "validate-tags",
    "link-coverage",
    "show-coverage",
    "generate-coverage",
];

const NOTES: &[&str] = &[
    "Areas: gherkin (the feature parses and passes validate's checks), tags (registered and correctly placed), format (the file equals the formatter's output), coverage (missing test or implementation files, ranges outside their file, steps without an exactly matching @step comment in the mapped lines, test files another feature maps, uncovered scenarios, coverage files out of sync), spec (features without scenarios, coverage files without their feature, features whose every mapped test file is missing)",
    "Every problem is an exact fact; nothing is guessed. Each prints on one line: <area> <code> <feature> [\"<scenario>\"] [<location>]: <fact> -> <command>",
    "@step counts only as the first token after a comment marker (//, #, --, *, /*) at the start of a line",
    "Implementation mappings record their first and last line text (firstLine, lastLine); a mapping whose lines no longer read that text is reported, and --fix shifts it when the first line occurs exactly once and the last line matches at the same offset",
    "--fix relinks a drifted, out-of-range or missing test mapping to the scenario's exact @step block (its @step comments in step order with no other @step comment between) when the block occurs exactly once in the scenario's mapped test files, or else in the files the working tree adds (untracked or staged as added) that no other feature maps; it keeps the implementation mappings. Otherwise the problem names how often the block occurs and where",
    "--fix also removes coverage entries whose scenario is not in the feature and deletes coverage files without their feature",
    "fspec init and fspec --sync-version install a git pre-commit hook that runs fspec audit --changed and blocks the commit unless it exits 0; a pre-commit hook fspec did not write is left alone, and fspec prints the line to add to it: fspec audit --changed || exit 1",
    "Fix kinds: mechanical (fspec audit --fix repairs it exactly), command (the concrete command repairs it) and guided (the repair needs judgment, such as a @step block found more or less than once or a command with a <placeholder>): a guided problem names fspec audit --start --feature <feature>, and its session step prints the facts and the repair command",
    "Audit sessions work like fspec reverse: --status, --continue, --reset and --complete take the same --area, --feature and --work-unit options as --start, and each scope has its own session, so agents auditing different scopes in one tree never share one. fspec decides nothing that needs judgment and runs no tests: a meaning step prints the scenario's mappings, the configured tools.test.command and the scratch-worktree procedure, and the agent runs it",
    "Exit code 0 means no problems, 1 problems, 2 an error; --continue exits 1 while the step's problem is still reported",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "audit",
    description: "Report every exact coverage, spec, Gherkin, tag and format problem with the command that fixes it",
    usage: Some("fspec audit [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Run it before committing and when a card reaches validating, to find drifted coverage mappings and spec problems."),
    when_not_to_use: Some("For one feature file in the edit loop, fspec validate and fspec validate-tags are quicker."),
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};

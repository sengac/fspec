//! `context` help configuration (CLI-019).
//!
//! Feature: spec/features/load-acceptance-criteria-with-fspec-context.feature
//!
//! The fixture rust/fspec/tests/fixtures/help/context.txt pins the rendered
//! text byte for byte.

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const fn opt(flag: &'static str, description: &'static str) -> CommandOption {
    CommandOption {
        flag,
        description,
        default_value: None,
    }
}

const OPTIONS: &[CommandOption] = &[
    opt(
        "--tags <expression>",
        "Cucumber tag expression with and, or, not and parentheses, e.g. '@auth and not @wip'. A scenario also has the tags of its Feature and Rule",
    ),
    opt(
        "--text <text>",
        "Only scenarios whose name, step text, doc string or table cell contains the text (case-insensitive)",
    ),
    opt(
        "--regex <pattern>",
        "Only scenarios whose name, step text, doc string or table cell matches the regular expression ((?i) ignores case)",
    ),
    opt(
        "--feature <name>",
        "Only the feature file with this name (e.g. user-login or user-login.feature)",
    ),
    CommandOption {
        flag: "--format <format>",
        description: "Output format: markdown, text or json",
        default_value: Some("markdown"),
    },
    CommandOption {
        flag: "--page <number>",
        description: "The page to print",
        default_value: Some("1"),
    },
    CommandOption {
        flag: "--page-size <chars>",
        description: "The most characters of acceptance criteria on one page; a scenario is never split",
        default_value: Some("40000"),
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec context --tags @AUTH-001",
        description: Some("Load the acceptance criteria of card AUTH-001 before working on it"),
        output: None,
    },
    CommandExample {
        command: "fspec context --tags '@auth and not @wip'",
        description: Some("Scenarios tagged @auth (on the scenario, its Rule or its Feature) that are not @wip"),
        output: Some("## spec/features/user-login.feature\n\n```gherkin\n@auth\nFeature: User login\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I see my board\n```\n\n### Login with valid credentials\n\nLocation: spec/features/user-login.feature:10\nInherited tags: @auth\n\n```gherkin\n@smoke\nScenario: Login with valid credentials\n  Given I am on the login page\n  When I enter valid credentials\n  Then I see my board\n```\n\n---\n\npage 1 of 1"),
    },
    CommandExample {
        command: "fspec context --tags '(@smoke or @critical) and not @deprecated' --page 2",
        description: Some("The second page of smoke or critical scenarios that are not deprecated"),
        output: None,
    },
    CommandExample {
        command: "fspec context --text 'invalid password'",
        description: Some("Scenarios that mention a phrase, ignoring case"),
        output: None,
    },
    CommandExample {
        command: "fspec context --regex 'log(in|out)' --feature user-login",
        description: Some("Scenarios of one feature that match a regular expression"),
        output: None,
    },
    CommandExample {
        command: "fspec context --tags @AUTH-001 --format json --page-size 20000",
        description: Some("One JSON page (page, pages, nextCommand, features, items) of at most 20000 characters"),
        output: None,
    },
];

const ERRORS: &[CommonError] = &[
    CommonError {
        error: "invalid tag expression '@a not @b': ...",
        fix: "Join tags with and, or and not, e.g. '@a and not @b', and quote the expression",
    },
    CommonError {
        error: "page 3 does not exist: there are 2 pages",
        fix: "Use a page from 1 to the page count, or follow the 'Next page:' command",
    },
    CommonError {
        error: "feature 'x' does not exist in spec/features/",
        fix: "Run fspec list-features for the feature names",
    },
];

const NOTES: &[&str] = &[
    "Each acceptance criterion is one scenario with all its steps, doc strings, data tables and Examples, its tags, its inherited Feature and Rule tags and its file:line",
    "A page prints each feature's header (tags, name, description and Background) once, above that feature's first scenario on the page",
    "Scenarios are ordered by feature file path, then line. A scenario larger than --page-size is alone on its page",
    "Every page ends with 'page N of M' and, except the last, 'Next page:' with the exact command for the next page",
    "All given filters must match; matching is exact (no semantic search)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "context",
    description: "Load acceptance criteria (scenarios with their steps, tags and location) selected by a cucumber tag expression, in pages",
    usage: Some("fspec context [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: &["show-feature", "list-features", "show-work-unit"],
    when_to_use: Some("Before working on a card, load its acceptance criteria with fspec context --tags @<workUnitId>, and read every page. Use it to find the scenarios of a tag, feature or phrase"),
    when_not_to_use: Some("To change scenarios (use add-scenario, update-scenario or update-step) or to see coverage (use show-coverage)"),
    prerequisites: &["spec/features exists"],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: ERRORS,
    notes: NOTES,
};

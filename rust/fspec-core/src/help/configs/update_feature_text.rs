//! `update-feature-text` help configuration (BUG-507).

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "feature",
    description:
        "Feature file name or path (e.g., \"user-login\" or \"spec/features/user-login.feature\")",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--find <text>",
        description: "Exact text to replace; it must occur exactly once in the feature file and may span lines",
        default_value: None,
    },
    CommandOption {
        flag: "--replace <text>",
        description: "Replacement text",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec update-feature-text user-login --find \"checked by the old service\" --replace \"checked by the auth service\"",
        description: Some("Fix a stale sentence in the description or a # comment"),
        output: Some("✓ Updated text on line 9 of user-login.feature"),
    },
    CommandExample {
        command: "fspec update-feature-text slash-commands --find \"| /role | ClearRole |\" --replace \"| /role | OpenRoleDialog |\"",
        description: Some("Fix a data table or Examples cell"),
        output: Some("✓ Updated text on line 49 of slash-commands.feature"),
    },
];

const RELATED: &[&str] = &["update-step", "update-scenario", "show-feature"];

const NOTES: &[&str] = &[
    "Edits the Feature title, description, # comments, Rule blocks and data table or Examples cells",
    "Text that touches a step line or a Scenario/Background header is refused: use update-step or update-scenario, which keep @step comments and coverage in sync",
    "No match or several matches is an error; the file is written only when the result is still valid Gherkin with the same scenarios and steps",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-feature-text",
    description: "Replace one exact piece of non-step text in a feature file",
    usage: Some("fspec update-feature-text <feature> --find <text> --replace <text>"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command to fix stale feature titles, descriptions, comments, rules or table cells without hand-editing the .feature file."),
    when_not_to_use: Some("Do not use it for steps or scenario names: use update-step or update-scenario."),
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};

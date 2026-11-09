//! BUG-566 — help configs and feature steps carry no TypeScript-era leftovers.
//!
//! Feature: spec/features/help-and-steps-carry-no-typescript-leftovers.feature
//!
//! Exact checks, no semantic judgement:
//!
//! - every text field of every `CommandHelpConfig` is non-empty and is not the
//!   literal `undefined` (the TypeScript formatter printed `undefined` for keys
//!   its configs did not set);
//! - the help every command prints contains no `undefined` word and no
//!   doubled `fspec fspec ` (TS related commands carried their own prefix);
//! - in every scenario and background that is not tagged `@deprecated` (on the
//!   scenario or the feature), each `fspec <command>` mention in a step or its
//!   doc string names a registered command. Mentions are extracted with
//!   CLI-016's rules (`guidance_contract_tests::mentions_in`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::CommandFactory;
use codelet_fspec_core::help::{CommandHelpConfig, CommonPatternEntry};

use crate::guidance_contract_tests::{
    files_under, mentions_in, registered_commands, relative, rust_root, with_big_stack,
};
use crate::Cli;

/// Every text field of `config`, labelled for the failure message.
fn text_fields(config: &CommandHelpConfig) -> Vec<(String, &'static str)> {
    let mut fields = vec![
        ("name".to_string(), config.name),
        ("description".to_string(), config.description),
    ];
    let optional = [
        ("usage", config.usage),
        ("whenToUse", config.when_to_use),
        ("whenNotToUse", config.when_not_to_use),
        ("typicalWorkflow", config.typical_workflow),
    ];
    fields.extend(
        optional
            .into_iter()
            .filter_map(|(label, value)| value.map(|v| (label.to_string(), v))),
    );
    for (i, arg) in config.arguments.iter().enumerate() {
        fields.push((format!("arguments[{i}].name"), arg.name));
        fields.push((format!("arguments[{i}].description"), arg.description));
    }
    for (i, opt) in config.options.iter().enumerate() {
        fields.push((format!("options[{i}].flag"), opt.flag));
        fields.push((format!("options[{i}].description"), opt.description));
        if let Some(default) = opt.default_value {
            fields.push((format!("options[{i}].defaultValue"), default));
        }
    }
    for (i, ex) in config.examples.iter().enumerate() {
        fields.push((format!("examples[{i}].command"), ex.command));
        if let Some(d) = ex.description {
            fields.push((format!("examples[{i}].description"), d));
        }
        if let Some(o) = ex.output {
            fields.push((format!("examples[{i}].output"), o));
        }
    }
    for (i, related) in config.related_commands.iter().enumerate() {
        fields.push((format!("relatedCommands[{i}]"), related));
    }
    for (i, p) in config.prerequisites.iter().enumerate() {
        fields.push((format!("prerequisites[{i}]"), p));
    }
    for (i, pattern) in config.common_patterns.iter().enumerate() {
        match pattern {
            CommonPatternEntry::Bullet(text) => {
                fields.push((format!("commonPatterns[{i}]"), text));
            }
            CommonPatternEntry::Structured(p) => {
                fields.push((format!("commonPatterns[{i}].pattern"), p.pattern));
                fields.push((format!("commonPatterns[{i}].example"), p.example));
                if let Some(d) = p.description {
                    fields.push((format!("commonPatterns[{i}].description"), d));
                }
            }
        }
    }
    for (i, e) in config.common_errors.iter().enumerate() {
        fields.push((format!("commonErrors[{i}].error"), e.error));
        fields.push((format!("commonErrors[{i}].fix"), e.fix));
    }
    for (i, n) in config.notes.iter().enumerate() {
        fields.push((format!("notes[{i}]"), n));
    }
    fields
}

/// True when `text` holds `undefined` as a whole word.
fn has_undefined_word(text: &str) -> bool {
    text.match_indices("undefined").any(|(at, word)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// The problems in one help config's text fields.
fn config_field_problems(config: &CommandHelpConfig) -> Vec<String> {
    text_fields(config)
        .into_iter()
        .filter_map(|(label, value)| match value.trim() {
            "" => Some(format!("help config {}: {label} is empty", config.name)),
            "undefined" => Some(format!(
                "help config {}: {label} is the literal 'undefined'",
                config.name
            )),
            _ => None,
        })
        .collect()
}

/// True when the tag list holds `@deprecated`.
fn is_deprecated(tags: &[String]) -> bool {
    tags.iter()
        .any(|t| t.trim_start_matches('@').eq_ignore_ascii_case("deprecated"))
}

/// The `fspec <command>` mentions in the steps of one feature file's
/// non-deprecated scenarios and background that name an unregistered command.
fn step_command_problems(source: &str, content: &str) -> Vec<String> {
    let Ok(feature) = codelet_fspec_core::io::gherkin::parse_feature_lenient(content) else {
        return vec![format!("{source}: does not parse")];
    };
    if is_deprecated(&feature.tags) {
        return Vec::new();
    }
    let commands = registered_commands();
    let mut steps = Vec::new();
    if let Some(background) = &feature.background {
        steps.extend(background.steps.iter());
    }
    for scenario in feature.scenarios.iter().filter(|s| !is_deprecated(&s.tags)) {
        steps.extend(scenario.steps.iter());
    }
    let mut problems = Vec::new();
    for step in steps {
        let mut texts = vec![(step.position.line, step.value.clone())];
        if let Some(doc) = &step.docstring {
            texts.push((step.position.line + 1, doc.clone()));
        }
        for (line, text) in texts {
            for mention in mentions_in(source, &text, line) {
                let command = mention
                    .text
                    .trim_start_matches("fspec ")
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    // A step quotes text around the mention ("Try: fspec
                    // list-epics"): a closing quote or `...` glued to the
                    // command token is not part of it.
                    .trim_end_matches(['"', '\'', '`', '.'])
                    .to_string();
                if !commands.contains(&command) {
                    problems.push(format!(
                        "{}:{}: `{}` names the unknown command '{command}'",
                        mention.source, mention.line, mention.text
                    ));
                }
            }
        }
    }
    problems
}

// Scenario: No help config field holds the literal 'undefined' or is empty
#[test]
fn no_help_config_field_holds_undefined_or_is_empty() {
    with_big_stack(|| {
        // @step Given every command's CommandHelpConfig and the help every command prints
        let root = Cli::command();
        let names: Vec<String> = root
            .get_subcommands()
            .map(|sub| sub.get_name().to_string())
            .collect();
        let configs: Vec<&CommandHelpConfig> = names
            .iter()
            .filter_map(|name| crate::ts_help_config(name))
            .collect();
        assert!(configs.len() > 150, "expected the help configs");

        // @step When each text field and each printed help is checked
        let mut problems: Vec<String> = configs
            .iter()
            .flat_map(|config| config_field_problems(config))
            .collect();
        for name in &names {
            if let Some(help) = crate::ts_help_text(name) {
                if has_undefined_word(&help) {
                    problems.push(format!("fspec {name} --help prints 'undefined'"));
                }
                if help.contains("fspec fspec ") {
                    problems.push(format!("fspec {name} --help prints 'fspec fspec '"));
                }
            }
        }

        // @step Then no field is empty or the literal 'undefined' and no help prints 'undefined'
        assert!(
            problems.is_empty(),
            "{} help leftovers:\n{}",
            problems.len(),
            problems.join("\n")
        );
    });
}

// Scenario: An 'undefined' common-error fix is reported naming the command and the field
#[test]
fn an_undefined_fix_is_reported_naming_the_command_and_field() {
    use codelet_fspec_core::help::CommonError;
    // @step Given a help config for "demo" whose first common error has the fix "undefined"
    const ERRORS: &[CommonError] = &[CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "undefined",
    }];
    let config = CommandHelpConfig {
        name: "demo",
        description: "Demo command",
        usage: None,
        arguments: &[],
        options: &[],
        examples: &[],
        related_commands: &[],
        when_to_use: Some(""),
        when_not_to_use: None,
        prerequisites: &[],
        common_patterns: &[],
        typical_workflow: None,
        common_errors: ERRORS,
        notes: &[],
    };

    // @step When its fields are checked
    let problems = config_field_problems(&config);

    // @step Then the problems name "commonErrors[0].fix is the literal 'undefined'" and "whenToUse is empty"
    assert_eq!(
        problems,
        vec![
            "help config demo: whenToUse is empty".to_string(),
            "help config demo: commonErrors[0].fix is the literal 'undefined'".to_string(),
        ]
    );
}

// Scenario: Feature steps name only registered fspec commands
#[test]
fn feature_steps_name_only_registered_commands() {
    with_big_stack(|| {
        // @step Given every feature file under spec/features
        let dir = rust_root()
            .parent()
            .expect("repository root")
            .join("spec/features");
        let mut files = Vec::new();
        files_under(&dir, &mut files);
        let features: Vec<_> = files
            .into_iter()
            .filter(|f| f.extension().is_some_and(|e| e == "feature"))
            .collect();
        assert!(features.len() > 2000, "expected the feature files");

        // @step When each fspec mention in the steps of its non-deprecated scenarios is checked against the registry
        let mut problems = Vec::new();
        for path in &features {
            let content = std::fs::read_to_string(path).expect("read feature");
            problems.extend(step_command_problems(&relative(path), &content));
        }

        // @step Then every mention names a registered command
        assert!(
            problems.is_empty(),
            "{} feature steps name unknown fspec commands:\n{}",
            problems.len(),
            problems.join("\n")
        );
    });
}

// Scenario: A step naming a renamed command fails while a deprecated scenario is skipped
#[test]
fn a_renamed_command_fails_while_a_deprecated_scenario_is_skipped() {
    with_big_stack(|| {
        // @step Given a feature whose scenario step runs the renamed command generate-tags and whose @deprecated scenario runs the never-ported command record-tokens
        let feature = concat!(
            "Feature: Tags\n",
            "\n",
            "  Scenario: Generate TAGS.md\n",
            "    When I run `fspec generate-tags`\n",
            "    Then TAGS.md exists\n",
            "\n",
            "  @deprecated\n",
            "  Scenario: Record tokens\n",
            "    When I run \"fspec record-tokens AUTH-001 45000\"\n",
            "    Then the command should succeed\n",
        );

        // @step When the feature's steps are checked
        let problems = step_command_problems("demo.feature", feature);

        // @step Then one problem names line 4 and the unknown command 'generate-tags'
        assert_eq!(
            problems,
            vec![
                "demo.feature:4: `fspec generate-tags` names the unknown command 'generate-tags'"
                    .to_string()
            ]
        );
    });
}

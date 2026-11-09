//! CLI-020 — one name for a work-unit reference on every surface.
//!
//! Feature: spec/features/one-name-for-a-work-unit-reference-on-every-surface.feature
//!
//! Exact checks, no semantic judgement. A name is a card-id name when, with
//! its case and every character other than a letter or digit dropped, it is
//! one of [`CARD_ID_SPELLINGS`]. Every such name must be spelled exactly
//! `workUnitId`:
//!
//! - every clap value name of every fspec subcommand (a positional without a
//!   value name shows its upper-cased id, so that is checked too);
//! - every `<...>` or `[...]` placeholder in the help every command prints
//!   (`<a|b>` is checked by its first alternative, `<a...>` by `a`);
//! - every placeholder in the init docs, their fixtures and the bootstrap doc.
//!
//! No subcommand has an `--id` or `--work-unit-id` flag, and no printed help
//! or doc names either flag.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use clap::CommandFactory;

use crate::guidance_contract_tests::{relative, rust_root, with_big_stack};
use crate::Cli;

/// The one spelling of a card-id name.
const CANON: &str = "workUnitId";

/// Lower-cased, letters-and-digits-only spellings that name a card id.
const CARD_ID_SPELLINGS: &[&str] = &[
    "workunitid",
    "workunitids",
    "workunit",
    "id",
    "ids",
    "parent",
    "parentid",
    "targetid",
    "sourceid",
    "dependsonid",
    "fromid",
    "toid",
    "otherid",
];

/// Long flags that must not exist (CLI-020: `--work-unit` is the card flag).
const FORBIDDEN_FLAGS: &[&str] = &["id", "work-unit-id"];

fn normalize(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn is_card_id_name(name: &str) -> bool {
    CARD_ID_SPELLINGS.contains(&normalize(name).as_str())
}

/// Problems with the value names and long flags of `cmd` and its
/// subcommands, each naming the command path and the offending name.
fn clap_problems(path: &str, cmd: &clap::Command, out: &mut Vec<String>) {
    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if id == "help" || id == "version" {
            continue;
        }
        if let Some(long) = arg.get_long() {
            if FORBIDDEN_FLAGS.contains(&long) {
                out.push(format!("{path}: flag --{long}"));
            }
        }
        if !arg.get_action().takes_values() {
            continue;
        }
        let names: Vec<String> = match arg.get_value_names() {
            Some(names) => names.iter().map(ToString::to_string).collect(),
            None => vec![id.to_uppercase()],
        };
        for name in names {
            if is_card_id_name(&name) && name != CANON {
                out.push(format!("{path}: argument {id} has value name {name}"));
            }
        }
    }
    for sub in cmd.get_subcommands() {
        clap_problems(&format!("{path} {}", sub.get_name()), sub, out);
    }
}

/// Every `<...>` and `[...]` placeholder in `text`, with its line number.
fn placeholders(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut at = 0;
        while at < chars.len() {
            let close = match chars[at] {
                '<' => '>',
                '[' => ']',
                _ => {
                    at += 1;
                    continue;
                }
            };
            let end = chars[at + 1..]
                .iter()
                .position(|c| *c == close || c.is_whitespace() || *c == chars[at]);
            match end {
                Some(len) if chars[at + 1 + len] == close && len > 0 => {
                    found.push((index + 1, chars[at..=at + 1 + len].iter().collect()));
                    at += len + 2;
                }
                _ => at += 1,
            }
        }
    }
    found
}

/// The card-id name a placeholder names: `<a|b>` by `a`, `<a...>` by `a`.
fn placeholder_name(token: &str) -> &str {
    let inner = &token[1..token.len() - 1];
    let first = inner.split('|').next().unwrap_or(inner);
    first.trim_end_matches('.')
}

/// Problems with the placeholders and flags in `text`, labelled `source`.
fn text_problems(source: &str, text: &str, out: &mut Vec<String>) {
    for (line, token) in placeholders(text) {
        let name = placeholder_name(&token);
        let exact = token == format!("<{CANON}>") || token == format!("[{CANON}]");
        if is_card_id_name(name) && !exact {
            out.push(format!("{source}:{line}: placeholder {token}"));
        }
    }
    for (index, line) in text.lines().enumerate() {
        for flag in FORBIDDEN_FLAGS {
            let needle = format!("--{flag}");
            let hit = line.match_indices(&needle).any(|(at, _)| {
                !line[at + needle.len()..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
            });
            if hit {
                out.push(format!("{source}:{}: flag {needle}", index + 1));
            }
        }
    }
}

/// The help every fspec subcommand prints, labelled `fspec <sub> --help`.
fn printed_help() -> Vec<(String, String)> {
    Cli::command()
        .get_subcommands()
        .filter_map(|sub| {
            let name = sub.get_name().to_string();
            crate::ts_help_text(&name).map(|help| (format!("fspec {name} --help"), help))
        })
        .collect()
}

/// The init docs, their fixtures and the bootstrap doc.
fn docs() -> Vec<(String, String)> {
    let core = rust_root().join("fspec-core");
    let mut paths: Vec<std::path::PathBuf> = ["plain_cli", "plain_ext", "plain_ide", "sysrem_cli"]
        .iter()
        .map(|kind| core.join(format!("src/commands/init_doc_{kind}.md")))
        .collect();
    paths.push(core.join("src/commands/bootstrap_doc.txt"));
    let fixtures = core.join("tests/fixtures/init_docs");
    let mut entries: Vec<_> = std::fs::read_dir(&fixtures)
        .expect("read init doc fixtures")
        .map(|entry| entry.expect("fixture entry").path())
        .collect();
    entries.sort();
    paths.extend(entries);
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).expect("read doc");
            (relative(Path::new(path)), text)
        })
        .collect()
}

fn report(problems: &[String]) -> String {
    format!("{} card-id names:\n{}", problems.len(), problems.join("\n"))
}

// Scenario: The guard test rejects every other card-id name
#[test]
fn the_guard_rejects_every_other_card_id_name() {
    with_big_stack(|| {
        // @step Given every fspec subcommand, argument and help page
        let root = Cli::command();
        let helps = printed_help();
        assert!(helps.len() > 150, "expected the help pages");
        let docs = docs();

        // @step Then every clap value name for a card id is workUnitId
        let mut clap = Vec::new();
        clap_problems("fspec", &root, &mut clap);
        clap.retain(|p| !p.contains(": flag --"));
        assert!(clap.is_empty(), "{}", report(&clap));

        // @step And every help placeholder for a card id is <workUnitId>
        let mut text = Vec::new();
        for (source, body) in helps.iter().chain(docs.iter()) {
            text_problems(source, body, &mut text);
        }
        let flags_in_text: Vec<String> = text
            .iter()
            .filter(|p| p.contains(": flag --"))
            .cloned()
            .collect();
        text.retain(|p| !p.contains(": flag --"));
        assert!(text.is_empty(), "{}", report(&text));

        // @step And no subcommand has an --id or --work-unit-id flag
        let mut flags = Vec::new();
        clap_problems("fspec", &root, &mut flags);
        flags.retain(|p| p.contains(": flag --"));
        flags.extend(flags_in_text);
        assert!(flags.is_empty(), "{}", report(&flags));

        // @step And a value name WORK_UNIT_ID, a placeholder <id> for a card or a --work-unit-id flag fails the guard with a message naming the command and the offending name
        let bad = clap::Command::new("demo")
            .arg(clap::Arg::new("work_unit_id").value_name("WORK_UNIT_ID"))
            .arg(clap::Arg::new("legacy").long("work-unit-id"));
        let mut caught = Vec::new();
        clap_problems("fspec demo", &bad, &mut caught);
        text_problems(
            "fspec demo --help",
            "USAGE\n  fspec demo <id>\n",
            &mut caught,
        );
        assert_eq!(
            caught,
            vec![
                "fspec demo: argument work_unit_id has value name WORK_UNIT_ID".to_string(),
                "fspec demo: flag --work-unit-id".to_string(),
                "fspec demo --help:2: placeholder <id>".to_string(),
            ]
        );
    });
}

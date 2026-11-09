//! CLI-016 — every fspec command, flag and example named in guidance or help
//! parses with the real CLI.
//!
//! Feature: spec/features/guidance-mentions-parse-with-the-real-cli.feature
//!
//! Sources: the bootstrap text, the init docs and every non-test string
//! literal of fspec-core and fspec; every help config. Each `fspec <command>`
//! mention is parsed with `Cli::try_parse_from` (parse only, nothing runs).
//! Exact extraction rules, no semantic judgement:
//!
//! - a mention is the word `fspec ` (not preceded by a letter, digit, `-`,
//!   `_`, `/` or `.`) followed by a command token that starts with a letter
//!   and contains a hyphen or is a registered subcommand;
//! - it ends at a line break (a line ending in ` \` continues on the next),
//!   at the next `fspec `, at a `|`, at ` #` (shell comment), at two
//!   consecutive spaces (a reference column), at ` (`, ` - `, ` → `, `. `,
//!   `, `, ` > ` or ` -->`, after a command token with punctuation glued to
//!   it, or — when the character before
//!   `fspec` is a backtick or quote — at the next same character; trailing
//!   `.`, `,`, `;`, `:` and `)` are dropped;
//! - tokens split on whitespace with `"..."` / `'...'` kept together;
//!   `<...>`, `{...}` and `__NAME__` placeholders become `X`; `[...]` groups
//!   and `...` are dropped;
//! - an unknown command or help topic and an unknown flag fail; in prose a
//!   missing required argument, a dummy value a flag rejects and a word after
//!   the arguments ("run fspec init first") are accepted. Help config
//!   examples must parse fully.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use clap::CommandFactory;
use syn::visit_mut::VisitMut;

use crate::Cli;

/// One `fspec ...` mention and where it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mention {
    pub(crate) source: String,
    pub(crate) line: usize,
    pub(crate) text: String,
}

thread_local! {
    /// The real clap tree, built once per thread; parsing reuses it
    /// (`try_get_matches_from_mut` is what `Cli::try_parse_from` runs).
    static CLI: std::cell::RefCell<clap::Command> = std::cell::RefCell::new(Cli::command());
}

/// Every registered subcommand name and alias (plus clap's `help`).
pub(crate) fn registered_commands() -> BTreeSet<String> {
    CLI.with(|cli| {
        let mut names = BTreeSet::from(["help".to_string()]);
        for sub in cli.borrow().get_subcommands() {
            names.insert(sub.get_name().to_string());
            names.extend(sub.get_all_aliases().map(str::to_string));
        }
        names
    })
}

/// Run `body` on a thread with room for clap's deep command tree.
pub(crate) fn with_big_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(body)
        .expect("spawn test thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}

const TRAILING: &[char] = &['.', ',', ';', ':', ')'];

/// Text that ends a mention (outside a "..." value): a table cell, a shell
/// comment, a reference column (two spaces), a parenthetical, a description
/// or next step, sentence punctuation, a shell redirect, an HTML comment end.
const STOPS: &[&str] = &[
    "|", " #", "  ", " (", " - ", ". ", ", ", " → ", " > ", " -->",
];

/// True when `fspec` at byte `at` of `line` is a whole word.
fn is_word_start(line: &str, at: usize) -> bool {
    match line[..at].chars().next_back() {
        None => true,
        Some(c) => !(c.is_alphanumeric() || matches!(c, '-' | '_' | '/' | '.')),
    }
}

/// Extract the mentions of one text, numbering lines from `first_line`.
pub(crate) fn mentions_in(source: &str, text: &str, first_line: usize) -> Vec<Mention> {
    if !text.contains("fspec ") {
        return Vec::new();
    }
    let commands = registered_commands();
    let mut found = Vec::new();
    // A line ending in ` \` continues on the next line (shell continuation).
    let mut lines: Vec<(usize, String)> = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (offset, raw) in text.split('\n').enumerate() {
        let (start, mut joined) = pending.take().unwrap_or((offset, String::new()));
        joined.push_str(raw);
        match joined.strip_suffix(" \\") {
            Some(head) => pending = Some((start, format!("{head} "))),
            None => lines.push((start, joined)),
        }
    }
    lines.extend(pending);
    for (offset, line) in &lines {
        let (offset, line) = (*offset, line.as_str());
        let starts: Vec<usize> = line
            .match_indices("fspec ")
            .map(|(at, _)| at)
            .filter(|at| is_word_start(line, *at))
            .collect();
        for (n, &at) in starts.iter().enumerate() {
            let rest_start = at + "fspec ".len();
            let mut end = starts.get(n + 1).copied().unwrap_or(line.len());
            if let Some(open) = line[..at].chars().next_back() {
                if matches!(open, '`' | '\'' | '"') {
                    if let Some(close) = line[rest_start..].find(open) {
                        end = end.min(rest_start + close);
                    }
                }
            }
            // Stops count only outside "..." values, so a quoted argument
            // may hold them.
            let mut in_quotes = false;
            for (pos, c) in line[rest_start..end].char_indices() {
                if c == '"' {
                    in_quotes = !in_quotes;
                }
                let rest = &line[rest_start + pos..end];
                if !in_quotes && STOPS.iter().any(|stop| rest.starts_with(stop)) {
                    end = rest_start + pos;
                    break;
                }
            }
            let mut body = line[rest_start..end].trim_end();
            while body.ends_with(TRAILING) {
                body = body.trim_end_matches(TRAILING).trim_end();
            }
            let Some(raw_command) = body.split_whitespace().next() else {
                continue;
            };
            let command = raw_command.trim_end_matches(TRAILING);
            // Punctuation glued to the command token ends the mention there.
            if command.len() < raw_command.len() {
                body = command;
            }
            let starts_alpha = command
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic());
            if starts_alpha && (command.contains('-') || commands.contains(command)) {
                found.push(Mention {
                    source: source.to_string(),
                    line: first_line + offset,
                    text: format!("fspec {body}"),
                });
            }
        }
    }
    found
}

/// Replace `<...>`, `{...}` and `__NAME__` placeholders with `X`.
fn fill_placeholders(token: &str) -> String {
    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(c) = chars.next() {
        let close = match c {
            '<' => Some('>'),
            '{' => Some('}'),
            _ => None,
        };
        if let Some(close) = close {
            let inner: String = chars.clone().take_while(|x| *x != close).collect();
            if chars.clone().any(|x| x == close) {
                for _ in 0..=inner.chars().count() {
                    chars.next();
                }
                out.push('X');
                continue;
            }
        }
        out.push(c);
    }
    let bytes = out.as_bytes();
    if out.len() > 4 && out.starts_with("__") && out.ends_with("__") && bytes[2] != b'_' {
        return "X".to_string();
    }
    out
}

/// Shell-like split, placeholders filled, optional groups and `...` dropped.
fn tokens(mention: &str) -> Vec<String> {
    let mut raw: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut quote: Option<char> = None;
    for c in mention.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                quoted = true;
            }
            None if c.is_whitespace() => {
                if !current.is_empty() || quoted {
                    raw.push((std::mem::take(&mut current), quoted));
                }
                quoted = false;
            }
            None => current.push(c),
        }
    }
    if !current.is_empty() || quoted {
        raw.push((current, quoted));
    }
    let mut out = Vec::new();
    let mut depth = 0usize;
    for (token, was_quoted) in raw {
        if was_quoted {
            if depth == 0 {
                out.push(fill_placeholders(&token));
            }
            continue;
        }
        let opens = token.starts_with('[');
        if opens {
            depth += 1;
        }
        let closes = depth > 0 && token.ends_with(']');
        if depth == 0 {
            let token = token.trim_end_matches("...");
            if !token.is_empty() {
                out.push(fill_placeholders(token));
            }
        }
        if closes {
            depth -= 1;
        }
    }
    out
}

/// Why a mention is rejected, or `None` when it parses (in prose mode a
/// missing argument or a rejected dummy value is accepted).
fn rejection(mention: &str, fully: bool) -> Option<String> {
    let args = tokens(mention);
    // BUG-590: an add-hook mention names an event add-hook accepts (a
    // placeholder event, the dummy X, is not checked).
    if args.get(1).map(String::as_str) == Some("add-hook") {
        if let Some(event) = args.get(2).filter(|e| *e != "X" && !e.starts_with('-')) {
            if let Err(reason) = codelet_fspec_core::command_hooks::check_hook_event(event) {
                return Some(reason);
            }
        }
    }
    let parsed = CLI.with(|cli| cli.borrow_mut().try_get_matches_from_mut(&args).map(|_| ()));
    match parsed {
        Ok(_) => None,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => None,
            ErrorKind::InvalidSubcommand => Some(format!("unknown command: {}", first_line(&err))),
            ErrorKind::UnknownArgument if fully || invalid_arg(&err).starts_with('-') => {
                Some(format!("unknown argument: {}", first_line(&err)))
            }
            _ if fully => Some(format!("does not parse: {}", first_line(&err))),
            _ => None,
        },
    }
}

/// The argument clap rejected (`--platform`, or a stray word in prose).
fn invalid_arg(err: &clap::Error) -> String {
    match err.get(clap::error::ContextKind::InvalidArg) {
        Some(clap::error::ContextValue::String(arg)) => arg.clone(),
        _ => String::new(),
    }
}

fn first_line(err: &clap::Error) -> String {
    err.to_string()
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn violations(mentions: &[Mention], fully: bool) -> Vec<String> {
    mentions
        .iter()
        .filter_map(|m| {
            rejection(&m.text, fully)
                .map(|why| format!("{}:{}: `{}`: {why}", m.source, m.line, m.text))
        })
        .collect()
}

// ───────────────────────── sources ─────────────────────────

pub(crate) fn rust_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("canonicalize rust/")
}

pub(crate) fn relative(path: &Path) -> String {
    let repo = rust_root()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(crate) fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("read source dir")
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Collects the string literals of non-test code: doc attributes and items
/// under `#[cfg(test)]` are skipped; literals inside macro calls count.
struct Literals {
    found: Vec<(usize, String)>,
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.parse_args::<syn::Ident>()
                .map(|i| i == "test")
                .unwrap_or(false)
    })
}

impl Literals {
    fn tokens(&mut self, stream: proc_macro2::TokenStream) {
        for tree in stream {
            match tree {
                proc_macro2::TokenTree::Group(g) => self.tokens(g.stream()),
                proc_macro2::TokenTree::Literal(lit) => {
                    if let Ok(s) = syn::parse_str::<syn::LitStr>(&lit.to_string()) {
                        self.found.push((lit.span().start().line, s.value()));
                    }
                }
                _ => {}
            }
        }
    }
}

impl VisitMut for Literals {
    fn visit_attribute_mut(&mut self, _: &mut syn::Attribute) {}
    fn visit_item_mut(&mut self, item: &mut syn::Item) {
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Mod(m) => &m.attrs,
            syn::Item::Fn(f) => &f.attrs,
            syn::Item::Impl(i) => &i.attrs,
            syn::Item::Const(c) => &c.attrs,
            syn::Item::Static(s) => &s.attrs,
            syn::Item::Struct(s) => &s.attrs,
            syn::Item::Enum(e) => &e.attrs,
            syn::Item::Use(u) => &u.attrs,
            syn::Item::Macro(m) => &m.attrs,
            _ => &[],
        };
        if !is_cfg_test(attrs) {
            syn::visit_mut::visit_item_mut(self, item);
        }
    }
    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        if !is_cfg_test(&f.attrs) {
            syn::visit_mut::visit_impl_item_fn_mut(self, f);
        }
    }
    fn visit_lit_str_mut(&mut self, lit: &mut syn::LitStr) {
        self.found.push((lit.span().start().line, lit.value()));
    }
    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        self.tokens(mac.tokens.clone());
    }
}

fn is_test_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    name.contains("test")
}

/// Every mention in the guidance sources.
pub(crate) fn source_mentions() -> Vec<Mention> {
    let root = rust_root();
    let mut found = Vec::new();
    let commands = root.join("fspec-core/src/commands");
    let mut docs = vec![commands.join("bootstrap_doc.txt")];
    let mut listing: Vec<PathBuf> = std::fs::read_dir(&commands)
        .expect("read commands dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.starts_with("init_doc_") && name.ends_with(".md")
        })
        .collect();
    listing.sort();
    docs.extend(listing);
    for doc in docs {
        let text = std::fs::read_to_string(&doc).expect("read guidance doc");
        found.extend(mentions_in(&relative(&doc), &text, 1));
    }
    for crate_src in ["fspec-core/src", "fspec/src"] {
        let mut files = Vec::new();
        files_under(&root.join(crate_src), &mut files);
        for file in files {
            if file.extension().and_then(|e| e.to_str()) != Some("rs") || is_test_file(&file) {
                continue;
            }
            let text = std::fs::read_to_string(&file).expect("read source");
            let Ok(mut ast) = syn::parse_file(&text) else {
                panic!("{} does not parse as Rust", relative(&file));
            };
            let mut literals = Literals { found: Vec::new() };
            literals.visit_file_mut(&mut ast);
            for (line, value) in literals.found {
                found.extend(mentions_in(&relative(&file), &value, line));
            }
        }
    }
    found
}

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn every_mention_in_the_guidance_sources_parses_with_the_real_cli() {
    with_big_stack(|| {
        // @step Given the bootstrap text, the init docs and the non-test string literals of fspec-core and fspec
        let mentions = source_mentions();
        assert!(
            mentions.len() > 500,
            "expected the guidance mentions, got {}",
            mentions.len()
        );

        // @step When every fspec mention in them is parsed with the real CLI
        let bad = violations(&mentions, false);

        // @step Then no mention names an unknown command, help topic or flag
        assert!(
            bad.is_empty(),
            "{} guidance mentions do not parse:\n{}",
            bad.len(),
            bad.join("\n")
        );
    });
}

#[test]
fn an_unknown_command_fails_while_missing_arguments_pass_in_prose() {
    with_big_stack(|| {
        // @step Given a source line "run fspec add-rule <id>" and a source line that runs the removed command link-feature with fspec
        let text = "run fspec add-rule <id>\nrun fspec link-feature X";

        // @step When the mentions are checked
        let mentions = mentions_in("sample.md", text, 1);
        let bad = violations(&mentions, false);

        // @step Then the add-rule mention passes
        assert_eq!(mentions.len(), 2, "{mentions:?}");
        assert!(!bad.iter().any(|b| b.contains("add-rule")), "{bad:?}");

        // @step And the link-feature mention fails naming the source, the line and "unknown command"
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(
            bad[0].starts_with("sample.md:2: `fspec link-feature X`: unknown command"),
            "{bad:?}"
        );
    });
}

#[test]
fn an_unknown_flag_fails() {
    with_big_stack(|| {
        // @step Given a source line "fspec research --tool=stakeholder --platform=teams"
        let mentions = mentions_in(
            "reminder.rs",
            "fspec research --tool=stakeholder --platform=teams",
            31,
        );

        // @step When the mention is checked
        let bad = violations(&mentions, false);

        // @step Then it fails naming the source, the line and the flag --platform
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].starts_with("reminder.rs:31: "), "{bad:?}");
        assert!(
            bad[0].contains("unknown argument") && bad[0].contains("--platform"),
            "{bad:?}"
        );
    });
}

#[test]
fn mention_boundaries_and_placeholders_are_exact() {
    with_big_stack(|| {
        // @step Given source text with mentions in backticks, in quotes, in a table cell, at the end of a sentence and with placeholders and optional groups
        let text = "Run `fspec bootstrap` first.\n\
                Say 'fspec list-work-units --status backlog' and go.\n\
                | fspec show-epic <id> | shows one epic |\n\
                Then run fspec add-rule {id} \"a rule\" [--why <text>] ....\n\
                See `fspec show-work-unit __ID__`, then fspec board.";

        // @step When the mentions are extracted
        let mentions = mentions_in("sample.md", text, 1);
        let texts: Vec<&str> = mentions.iter().map(|m| m.text.as_str()).collect();

        // @step Then each mention ends at its closing backtick or quote, at the '|', or before the trailing punctuation
        assert_eq!(
            texts,
            vec![
                "fspec bootstrap",
                "fspec list-work-units --status backlog",
                "fspec show-epic <id>",
                "fspec add-rule {id} \"a rule\" [--why <text>]",
                "fspec show-work-unit __ID__",
                "fspec board",
            ]
        );

        // @step And placeholders become X while optional groups and ... are dropped
        assert_eq!(tokens(texts[2]), vec!["fspec", "show-epic", "X"]);
        assert_eq!(tokens(texts[3]), vec!["fspec", "add-rule", "X", "a rule"]);
        assert_eq!(tokens(texts[4]), vec!["fspec", "show-work-unit", "X"]);
        assert_eq!(
            tokens("fspec add-dependencies <id> <dep>..."),
            vec!["fspec", "add-dependencies", "X", "X"]
        );
        assert_eq!(
            tokens("fspec link-coverage f ..."),
            vec!["fspec", "link-coverage", "f"]
        );
    });
}

/// Long and short flags of one clap subcommand.
fn flags_of(command: &clap::Command) -> BTreeSet<String> {
    let mut flags = BTreeSet::from(["--help".to_string(), "-h".to_string()]);
    for arg in command.get_arguments() {
        if let Some(long) = arg.get_long() {
            flags.insert(format!("--{long}"));
        }
        if let Some(aliases) = arg.get_all_aliases() {
            flags.extend(aliases.into_iter().map(|a| format!("--{a}")));
        }
        if let Some(short) = arg.get_short() {
            flags.insert(format!("-{short}"));
        }
    }
    flags
}

#[test]
fn help_configs_name_only_real_examples_options_and_related_commands() {
    with_big_stack(|| {
        // @step Given every command's CommandHelpConfig
        let root = Cli::command();
        let commands = registered_commands();
        let configs: Vec<(&clap::Command, &codelet_fspec_core::help::CommandHelpConfig)> = root
            .get_subcommands()
            .filter_map(|sub| crate::ts_help_config(sub.get_name()).map(|c| (sub, c)))
            .collect();
        assert!(
            configs.len() > 150,
            "expected the help configs, got {}",
            configs.len()
        );

        // @step When its examples, option flags and related commands are checked against the CLI
        let mut bad = Vec::new();
        for (sub, config) in &configs {
            let source = format!("help config {}", config.name);
            for example in config.examples {
                let mentions = mentions_in(&source, example.command, 1);
                if mentions.is_empty() {
                    bad.push(format!(
                        "{source}: example `{}` names no fspec command",
                        example.command
                    ));
                }
                bad.extend(violations(&mentions, true));
            }
            let flags = flags_of(sub);
            for option in config.options {
                let named: Vec<&str> = option
                    .flag
                    .split([',', ' ', '='])
                    .filter(|f| f.starts_with('-'))
                    .collect();
                if named.is_empty() {
                    bad.push(format!("{source}: option `{}` names no flag", option.flag));
                }
                for flag in named {
                    if !flags.contains(flag) {
                        bad.push(format!(
                            "{source}: option {flag} does not exist on {}",
                            sub.get_name()
                        ));
                    }
                }
            }
            for related in config.related_commands {
                let name = related
                    .trim_start_matches("fspec ")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if !commands.contains(name) {
                    bad.push(format!(
                        "{source}: related command `{related}` does not exist"
                    ));
                }
            }
        }

        // @step Then every example parses fully, every option flag exists on its command and every related command exists
        assert!(
            bad.is_empty(),
            "{} help config problems:\n{}",
            bad.len(),
            bad.join("\n")
        );
    });
}

#[test]
fn help_fixtures_follow_their_configs() {
    with_big_stack(|| {
        // @step Given the help fixtures under rust/fspec/tests/fixtures/help
        let dir = rust_root().join("fspec/tests/fixtures/help");
        let mut fixtures = Vec::new();
        files_under(&dir, &mut fixtures);
        assert!(fixtures.len() > 150, "expected the help fixtures");

        // @step When each fixture is compared with the help rendered from its command's config
        let mut stale = Vec::new();
        for fixture in &fixtures {
            let name = fixture
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let on_disk = std::fs::read_to_string(fixture).expect("read fixture");
            match crate::ts_help_text(name) {
                Some(rendered) if rendered == on_disk => {}
                Some(_) => stale.push(format!(
                    "{} differs from the help rendered from its config",
                    relative(fixture)
                )),
                None => stale.push(format!("{} has no help config", relative(fixture))),
            }
        }

        // @step Then every fixture equals its rendered help
        assert!(
            stale.is_empty(),
            "{} stale help fixtures:\n{}",
            stale.len(),
            stale.join("\n")
        );
    });
}

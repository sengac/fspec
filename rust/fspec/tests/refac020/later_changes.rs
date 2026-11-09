//! BUG-583: approved changes made after the REFAC-020 freeze to the CLI
//! functions, recorded exactly so the frozen dispatch, startup and help
//! contract still compares everything else (as BUG-490 recorded commands and
//! fields in source.rs). Each recorded form names its card; any other change
//! fails the comparison with an error naming what changed.

use std::collections::BTreeMap;

use syn::visit_mut::VisitMut;

use crate::source::{self, tokens};

/// Functions added after the freeze, each compared by its own card's
/// feature: HOOK-018's `command_hook_target` and `run_cli` (main's routing
/// moved into `run_cli`), and CLI-016's `ts_help_text` and `ts_help_config`
/// (the help interception split out of `intercept_ts_help`).
pub const ADDED_FUNCTIONS: [&str; 4] = [
    "command_hook_target",
    "run_cli",
    "ts_help_text",
    "ts_help_config",
];

/// HOOK-018: `main` parses into `ArgMatches` (for the command hook target)
/// and then into `Cli`, where the frozen `main` called `Cli::try_parse`.
const HOOK018_PARSE: &str = r#"{
    let matches = match Cli::command().try_get_matches() {
        Ok(matches) => matches,
        Err(err) => return render_clap_error(err),
    };
    let cli = match <Cli as clap::FromArgMatches>::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(err) => return render_clap_error(err.format(&mut Cli::command())),
    };
}"#;

/// The frozen parse [`HOOK018_PARSE`] replaced.
const FROZEN_PARSE: &str = r#"{
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return render_clap_error(err),
    };
}"#;

/// HOOK-018: the command-hook wrapper that ends `main` around `run_cli`,
/// whose body is the frozen routing.
const HOOK018_WRAPPER: &str = r#"{
    let Some(target) = command_hook_target(&matches) else {
        return run_cli(cli).await;
    };
    let root = match target.project_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("{err:#}");
            return std::process::ExitCode::from(1);
        }
    };
    let pre = target.run(&root, HookPhase::Pre);
    let Some(pre) = pre else {
        return std::process::ExitCode::from(1);
    };
    if pre.blocked {
        return std::process::ExitCode::from(1);
    }
    let code = run_cli(cli).await;
    if code != std::process::ExitCode::SUCCESS {
        return code;
    }
    match target.run(&root, HookPhase::Post) {
        Some(post) if !post.blocked => code,
        _ => std::process::ExitCode::from(1),
    }
}"#;

/// CLI-016: `intercept_ts_help` prints [`ts_help_text`]'s text, where the
/// frozen one matched the command itself and printed the rendered config.
const CLI016_HELP_TAIL: &str = r#"{
    let text = ts_help_text(sub)?;
    print!("{text}");
    Some(0)
}"#;

/// CLI-016: commands without a help config fall through to `ts_help_config`,
/// rendered with the newline the frozen `println!` appended.
const CLI016_CONFIG_FALLBACK: &str =
    r#"ts_help_config(sub).map(|config| format!("{}\n", format_command_help(config)))"#;

fn statements(text: &str) -> Result<Vec<String>, String> {
    let block: syn::Block = syn::parse_str(text).map_err(|error| error.to_string())?;
    Ok(block.stmts.iter().map(tokens).collect())
}

fn find_run(haystack: &[String], needle: &[String]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn take(functions: &mut Vec<syn::ItemFn>, name: &str) -> Result<syn::ItemFn, String> {
    let index = functions
        .iter()
        .position(|function| function.sig.ident == name)
        .ok_or_else(|| format!("recorded function {name} missing from the maintained CLI"))?;
    Ok(functions.remove(index))
}

/// HOOK-018: rebuild the frozen view of `main` — the recorded parse becomes
/// `Cli::try_parse` again and the recorded hook wrapper ending `main` becomes
/// `run_cli`'s body. Fails when `main` no longer holds exactly those forms.
fn inline_command_hooks(
    functions: &mut [syn::ItemFn],
    run_cli: &syn::ItemFn,
) -> Result<(), String> {
    let main = functions
        .iter_mut()
        .find(|function| function.sig.ident == "main")
        .ok_or("function main missing from the maintained CLI")?;
    let mut body: Vec<syn::Stmt> = main.block.stmts.clone();
    let texts: Vec<String> = body.iter().map(tokens).collect();
    let parse = statements(HOOK018_PARSE)?;
    let at = find_run(&texts, &parse)
        .ok_or("function main: HOOK-018's recorded ArgMatches parse is not in main")?;
    let frozen: syn::Block = syn::parse_str(FROZEN_PARSE).map_err(|error| error.to_string())?;
    body.splice(at..at + parse.len(), frozen.stmts);
    let texts: Vec<String> = body.iter().map(tokens).collect();
    let wrapper = statements(HOOK018_WRAPPER)?;
    if !texts.ends_with(&wrapper) {
        return Err(
            "function main: main does not end with HOOK-018's recorded hook wrapper".into(),
        );
    }
    body.truncate(body.len() - wrapper.len());
    body.extend(run_cli.block.stmts.iter().cloned());
    main.block.stmts = body;
    Ok(())
}

/// The match on `sub` inside a function's body.
fn sub_match(function: &syn::ItemFn) -> Result<syn::ExprMatch, String> {
    struct Find(Option<syn::ExprMatch>);
    impl VisitMut for Find {
        fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
            if self.0.is_none() && tokens(&node.expr) == "sub" {
                self.0 = Some(node.clone());
            }
            syn::visit_mut::visit_expr_match_mut(self, node);
        }
    }
    let mut find = Find(None);
    find.visit_item_fn_mut(&mut function.clone());
    find.0
        .ok_or_else(|| format!("function {}: no match on sub", function.sig.ident))
}

fn between<'text>(text: &'text str, start: &str, end: &str) -> Option<&'text str> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(text[from..to].trim())
}

/// One help interception entry: the command and what `--help` prints for it.
fn entry(arm: &syn::Arm) -> Result<Option<(String, String)>, String> {
    let syn::Pat::Lit(pattern) = &arm.pat else {
        return Ok(None);
    };
    let syn::Lit::Str(command) = &pattern.lit else {
        return Err(format!(
            "help arm {} is not a command name",
            tokens(&arm.pat)
        ));
    };
    let body = tokens(&arm.body);
    let printed = if let Some(config) = between(&body, "configs :: ", ":: CONFIG") {
        format!("config {config}")
    } else if let Some(text) = between(&body, "print ! (\"{}\" , ", ")") {
        format!("static {text}")
    } else if let Some(text) = between(&body, "Some (", ". to_string ())") {
        format!("static {text}")
    } else {
        return Err(format!(
            "help arm {}: unrecognised body {body}",
            command.value()
        ));
    };
    Ok(Some((command.value(), printed)))
}

fn table(arms: &[syn::Arm]) -> Result<BTreeMap<String, String>, String> {
    let mut table = BTreeMap::new();
    for arm in arms {
        if let Some((command, printed)) = entry(arm)? {
            let removed = source::ADDED_COMMAND_NAMES
                .iter()
                .chain(source::REMOVED_COMMAND_NAMES)
                .any(|name| *name == command);
            if !removed {
                table.insert(command, printed);
            }
        }
    }
    Ok(table)
}

/// CLI-016: compare the frozen help interception as its exact table of
/// command to printed help (config or static text) and its unchanged argv
/// prelude, then drop `intercept_ts_help` from the token comparison.
fn compare_help_interception(
    expected: &mut Vec<syn::ItemFn>,
    actual: &mut Vec<syn::ItemFn>,
    text: &syn::ItemFn,
    config: &syn::ItemFn,
) -> Result<(), String> {
    let frozen = take(expected, "intercept_ts_help")?;
    let maintained = take(actual, "intercept_ts_help")?;
    let frozen_statements: Vec<String> = frozen.block.stmts.iter().map(tokens).collect();
    let maintained_statements: Vec<String> = maintained.block.stmts.iter().map(tokens).collect();
    let tail = statements(CLI016_HELP_TAIL)?;
    // The frozen body opens with the `use` its match needed and ends with
    // the match and its println!; the argv prelude between them is frozen.
    let prelude = frozen_statements
        .get(1..frozen_statements.len().saturating_sub(3))
        .unwrap_or_default();
    if maintained_statements[..] != [prelude, &tail[..]].concat()[..] {
        return Err(
            "function intercept_ts_help: argv prelude or CLI-016's recorded tail changed".into(),
        );
    }
    let text_match = sub_match(text)?;
    let fallback = text_match
        .arms
        .last()
        .filter(|arm| matches!(arm.pat, syn::Pat::Wild(_)))
        .map(|arm| tokens(&arm.body));
    let recorded: syn::Expr =
        syn::parse_str(CLI016_CONFIG_FALLBACK).map_err(|error| error.to_string())?;
    if fallback != Some(tokens(&recorded)) {
        return Err("function ts_help_text: CLI-016's recorded config fallback changed".into());
    }
    let mut maintained_table = table(&text_match.arms)?;
    maintained_table.extend(table(&sub_match(config)?.arms)?);
    let frozen_table = table(&sub_match(&frozen)?.arms)?;
    let changed: Vec<String> = frozen_table
        .iter()
        .filter(|(command, printed)| maintained_table.get(*command) != Some(printed))
        .map(|(command, printed)| {
            format!(
                "{command}: {printed} -> {:?}",
                maintained_table.get(command)
            )
        })
        .chain(
            maintained_table
                .keys()
                .filter(|command| !frozen_table.contains_key(*command))
                .map(|command| format!("{command}: added")),
        )
        .collect();
    if !changed.is_empty() {
        return Err(format!("help interception entries changed: {changed:?}"));
    }
    Ok(())
}

/// Apply every recorded later change: afterwards `actual` holds exactly the
/// frozen functions (minus `intercept_ts_help`, compared here as a table)
/// or the comparison fails naming the unrecorded difference.
pub fn frozen_view(
    expected: &mut Vec<syn::ItemFn>,
    actual: &mut Vec<syn::ItemFn>,
) -> Result<(), String> {
    let [hook_target, run_cli, text, config] = ADDED_FUNCTIONS;
    take(actual, hook_target)?;
    let run_cli = take(actual, run_cli)?;
    let text = take(actual, text)?;
    let config = take(actual, config)?;
    inline_command_hooks(actual, &run_cli)?;
    compare_help_interception(expected, actual, &text, &config)?;
    let unrecorded: Vec<String> = actual
        .iter()
        .map(|function| function.sig.ident.to_string())
        .filter(|name| !expected.iter().any(|function| function.sig.ident == name))
        .collect();
    if !unrecorded.is_empty() {
        return Err(format!(
            "functions added after the freeze and not recorded: {unrecorded:?}"
        ));
    }
    Ok(())
}

/// BUG-537: drops the `section_flag` binding from the show-foundation
/// dispatch arm and its `section . or (section_flag)` argument.
pub struct StripAddedSectionRouting;

impl VisitMut for StripAddedSectionRouting {
    fn visit_pat_struct_mut(&mut self, pattern: &mut syn::PatStruct) {
        let variant = pattern
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if variant.as_deref() == Some("ShowFoundation") {
            let trailing = pattern.fields.trailing_punct();
            let mut kept: syn::punctuated::Punctuated<syn::FieldPat, syn::Token![,]> = pattern
                .fields
                .iter()
                .filter(|field| {
                    !source::ADDED_SECTION_FIELDS.contains(&tokens(&field.member).as_str())
                })
                .cloned()
                .collect();
            if trailing && !kept.empty_or_trailing() {
                kept.push_punct(Default::default());
            }
            pattern.fields = kept;
        }
        syn::visit_mut::visit_pat_struct_mut(self, pattern);
    }

    /// `section: section.or(section_flag)` reads as the frozen shorthand
    /// `section`.
    fn visit_field_value_mut(&mut self, field: &mut syn::FieldValue) {
        if tokens(&field.member) == "section"
            && tokens(&field.expr) == "section . or (section_flag)"
        {
            field.expr = syn::parse_quote!(section);
            field.colon_token = None;
        }
        syn::visit_mut::visit_field_value_mut(self, field);
    }

    fn visit_expr_macro_mut(&mut self, expression: &mut syn::ExprMacro) {
        source::visit_forward_arguments(self, expression);
    }
}

/// CLI-020 (product owner, 2026-11-08): one name for a work-unit reference.
/// Every clap value name for a card id became `workUnitId`, query-metrics'
/// `--work-unit-id` became `--work-unit`, and query-work-units' `--prefix`
/// lost its `-p` (which means `--parent`). Recorded exactly: each pair is
/// applied to the frozen original, so any other change still fails.
const CLI020_RENAMES: &[(&str, &str)] = &[
    ("value_name = \"WORK_UNIT_ID\"", "value_name = \"workUnitId\""),
    ("value_name = \"work-unit-id\"", "value_name = \"workUnitId\""),
    ("value_name = \"DEPENDS_ON_ID\"", "value_name = \"workUnitId\""),
    ("value_name = \"TARGET_ID\"", "value_name = \"workUnitId\""),
    ("value_name = \"FROM_ID\"", "value_name = \"workUnitId\""),
    ("value_name = \"TO_ID\"", "value_name = \"workUnitId\""),
    ("value_name = \"PARENT\"", "value_name = \"workUnitId\""),
    ("num_args = 1.., value_name = \"IDS\"", "num_args = 1.., value_name = \"workUnitId\""),
    ("long = \"work-unit\", value_name = \"id\"", "long = \"work-unit\", value_name = \"workUnitId\""),
    (
        "/// `--work-unit-id <id>` — query metrics for a single unit.\n        #[arg(long = \"work-unit-id\", value_name = \"workUnitId\")]",
        "/// `--work-unit <workUnitId>` — query metrics for a single unit.\n        #[arg(long = \"work-unit\", value_name = \"workUnitId\")]",
    ),
    (
        "/// Filter by prefix.\n        #[arg(short = 'p', long, value_name = \"PREFIX\")]",
        "/// Filter by prefix (no short flag: -p means --parent, CLI-020).\n        #[arg(long, value_name = \"PREFIX\")]",
    ),
    (
        "QueryEstimationGuide {\n        /// Work unit ID.\n",
        "QueryEstimationGuide {\n        #[arg(value_name = \"workUnitId\")]\n",
    ),
    (
        "ShowEventStorm {\n        /// Work unit ID.\n",
        "ShowEventStorm {\n        #[arg(value_name = \"workUnitId\")]\n",
    ),
];

/// The frozen original with [`CLI020_RENAMES`] applied; a pair that no
/// longer matches is an error naming it.
pub fn cli020_renamed(original: String) -> Result<String, String> {
    let mut text = original;
    for (from, to) in CLI020_RENAMES {
        if !text.contains(from) {
            return Err(format!(
                "CLI-020 rename no longer matches the frozen original: {from}"
            ));
        }
        text = text.replace(from, to);
    }
    Ok(text)
}

//! `fspec context` (CLI-019): the acceptance criteria an LLM needs, selected
//! by a cucumber tag expression and exact filters, in pages of a character
//! budget.
//!
//! Feature: spec/features/load-acceptance-criteria-with-fspec-context.feature
//!
//! Replaces show-acceptance-criteria, get-scenarios and search-scenarios
//! (product owner, 2026-11-08). One criterion is one scenario with its steps,
//! doc strings, tables and Examples, its own and inherited tags and its
//! file:line; each page prints a feature's header once, above that
//! feature's first scenario on the page.
//!
//! Two front doors: the CLI bridge (`rust/fspec/src/context.rs`) and the LLM
//! dispatcher both send the same JSON args to [`run`], which returns the
//! rendered page.

mod render;
mod select;

use std::path::Path;

use serde::Deserialize;

use crate::error::FspecCoreError;

/// The page budget, in characters, when `pageSize` is not given.
pub const DEFAULT_PAGE_SIZE: usize = 40_000;

/// The JSON args of `context`; every field is optional and no other is accepted.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Args {
    tags: Option<String>,
    text: Option<String>,
    regex: Option<String>,
    feature: Option<String>,
    format: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

/// The output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Markdown,
    Text,
    Json,
}

impl Format {
    fn parse(format: Option<&str>) -> Result<Self, FspecCoreError> {
        match format {
            None | Some("markdown") => Ok(Self::Markdown),
            Some("text") => Ok(Self::Text),
            Some("json") => Ok(Self::Json),
            Some(other) => Err(invalid(format!(
                "unknown format '{other}': use markdown, text or json"
            ))),
        }
    }
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "context",
        reason,
    }
}

/// `value` when it is at least 1, `default` when it is absent.
fn at_least_one(value: Option<usize>, flag: &str, default: usize) -> Result<usize, FspecCoreError> {
    match value {
        None => Ok(default),
        Some(0) => Err(invalid(format!("{flag} must be at least 1"))),
        Some(n) => Ok(n),
    }
}

/// Dispatcher / CLI entry point: one rendered page.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;
    let format = Format::parse(args.format.as_deref())?;
    let page = at_least_one(args.page, "--page", 1)?;
    let budget = at_least_one(args.page_size, "--page-size", DEFAULT_PAGE_SIZE)?;
    let filters = select::Filters::new(
        args.tags.as_deref(),
        args.text.as_deref(),
        args.regex.as_deref(),
    )?;
    let selection = select::load(project_root, args.feature.as_deref(), &filters)?;
    let blocks = render::Blocks::new(&selection, format)?;
    let pages = paginate(&selection, &blocks, budget);
    let count = pages.len();
    let Some(items) = pages.get(page - 1) else {
        let there = if count == 1 {
            "there is 1 page".to_string()
        } else {
            format!("there are {count} pages")
        };
        return Err(invalid(format!("page {page} does not exist: {there}")));
    };
    let next = (page < count).then(|| next_command(&args, page + 1));
    render::page(&selection, &blocks, items, page, count, next.as_deref())
}

/// Split the selected criteria, in order, into pages: a page takes the next
/// criterion (plus its feature's header when the page has none of that
/// feature's criteria yet) while its body stays within `budget` characters.
/// A criterion is never split; one that does not fit an empty page is alone
/// on its page. No criterion is one empty page.
fn paginate(
    selection: &select::Selection,
    blocks: &render::Blocks,
    budget: usize,
) -> Vec<Vec<usize>> {
    let mut pages: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut used = 0usize;
    for (index, criterion) in selection.criteria.iter().enumerate() {
        let header = blocks.header_len(criterion.feature);
        let item = blocks.item_len(index);
        let has_feature = current
            .iter()
            .any(|&i| selection.criteria[i].feature == criterion.feature);
        let cost = item + if has_feature { 0 } else { header };
        if !current.is_empty() && used + cost > budget {
            pages.push(std::mem::take(&mut current));
            used = 0;
            current.push(index);
            used += item + header;
        } else {
            current.push(index);
            used += cost;
        }
    }
    if !current.is_empty() || pages.is_empty() {
        pages.push(current);
    }
    pages
}

/// `fspec context` with the given flags in a fixed order plus `--page next`.
fn next_command(args: &Args, next: usize) -> String {
    let page_size = args.page_size.map(|n| n.to_string());
    let flags = [
        ("--tags", args.tags.as_deref()),
        ("--text", args.text.as_deref()),
        ("--regex", args.regex.as_deref()),
        ("--feature", args.feature.as_deref()),
        ("--format", args.format.as_deref()),
        ("--page-size", page_size.as_deref()),
    ];
    let mut command = String::from("fspec context");
    for (flag, value) in flags {
        let Some(value) = value else { continue };
        let quoted = shell_quote(value);
        if value.starts_with('-') {
            command.push_str(&format!(" {flag}={quoted}"));
        } else {
            command.push_str(&format!(" {flag} {quoted}"));
        }
    }
    command.push_str(&format!(" --page {next}"));
    command
}

/// `value` as one shell word: bare when it has only letters, digits and
/// `@_./:-`, otherwise single-quoted.
fn shell_quote(value: &str) -> String {
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@_./:-".contains(c));
    if bare {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

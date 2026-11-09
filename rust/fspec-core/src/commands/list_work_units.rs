//! `list-work-units` — the board query command (RPC-253, CLI-017).
//!
//! Feature: spec/features/list-work-units-rust-port.feature
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! Loads `spec/work-units.json` once (auto-creating it and
//! `spec/prefixes.json` on first run), applies exact filters (OR within a
//! filter, AND across filters: status, type, prefix, epic, ids, parent,
//! estimate, created/updated/done/entered time ranges, text, content and
//! link filters, feature-file tags), then picks fields, sorts, limits,
//! groups and renders as table, ids, json, jsonl, count or csv. Every
//! unknown value is an exact error listing the valid ones. The clock and
//! local zone are injected through [`run_at`] (TEST-042).

use std::path::Path;

use crate::error::FspecCoreError;
use crate::io::ensure::{ensure_prefixes_file, ensure_work_units_file, read_epics_or_empty};

mod args;
mod card;
mod clock;
mod fields;
mod filter;
mod render;
mod time;

pub use clock::{BoardClock, BoardZone};

use args::Args;
use card::Card;
use fields::{compare, value, Field, DEFAULT_FIELDS};
use filter::{Known, Query, TimeKind};
use render::{Group, Row, FORMATS};

const STATUSES: &[&str] = &[
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];
const TYPES: &[&str] = &["story", "task", "bug"];
const GROUPS: &[&str] = &["status", "type", "prefix", "epic", "day", "week"];

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "list-work-units",
        reason,
    }
}

/// Dispatcher entry point: the query on the system clock and local zone.
/// The dispatcher passes the canonical project root alongside the raw JSON
/// args; we never call `std::env::current_dir()`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    run_at(args_json, project_root, &BoardClock::system()).await
}

/// [`run`] with an injected clock and local zone (CLI-017, TEST-042).
pub async fn run_at(
    args_json: &str,
    project_root: &Path,
    clock: &BoardClock,
) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;

    let data = ensure_work_units_file(project_root)?;
    let prefixes = ensure_prefixes_file(project_root)?;
    let epics = if args.epic.is_some() {
        read_epics_or_empty(project_root)?
            .epics
            .keys()
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let mut prefix_names: Vec<String> = prefixes.prefixes.keys().cloned().collect();
    prefix_names.sort();
    let mut epic_names: Vec<String> = epics;
    epic_names.sort();

    query(
        &args,
        &data.work_units,
        Known {
            board: &data.work_units,
            prefixes: prefix_names,
            epics: epic_names,
            project_root,
        },
        clock,
    )
    .map_err(invalid)
}

fn query(
    args: &Args,
    board: &indexmap::IndexMap<String, crate::types::work_unit::WorkUnit>,
    known: Known,
    clock: &BoardClock,
) -> Result<String, String> {
    let format = args.format.as_deref().unwrap_or("table");
    if !FORMATS.contains(&format) {
        return Err(format!(
            "Unknown format '{format}'. Valid formats: {}",
            FORMATS.join(", ")
        ));
    }
    if let Some(g) = args.group_by.as_deref() {
        if !GROUPS.contains(&g) {
            return Err(format!(
                "Unknown group-by '{g}'. Valid group-by values: {}",
                GROUPS.join(", ")
            ));
        }
    }
    let fields: Vec<Field> = match &args.fields {
        Some(list) => list
            .values()
            .iter()
            .map(|f| Field::parse(f, "field"))
            .collect::<Result<_, _>>()?,
        None => DEFAULT_FIELDS
            .iter()
            .map(|f| Field::parse(f, "field"))
            .collect::<Result<_, _>>()?,
    };
    let sort = args
        .sort
        .as_deref()
        .map(|s| Field::parse(s, "sort key"))
        .transpose()?;
    let q = Query::build(args, &known, clock)?;
    let day_kind = match args.group_by.as_deref() {
        Some(g @ ("day" | "week")) => match q.time_kinds().as_slice() {
            [kind] => Some(*kind),
            _ => {
                return Err(format!(
                    "--group-by {g} needs exactly one kind of time filter (created, updated, done or entered)"
                ))
            }
        },
        _ => None,
    };

    let mut matched: Vec<Card> = Vec::new();
    for wu in board.values() {
        let card = Card::new(wu);
        if q.matches(&card, board)? {
            matched.push(card);
        }
    }

    if let Some(sort) = &sort {
        let mut keyed = Vec::with_capacity(matched.len());
        for card in matched {
            keyed.push((value(&card, sort, clock)?, card));
        }
        keyed.sort_by(|(a, _), (b, _)| {
            let order = compare(a, b, clock);
            if args.desc && a.is_some() && b.is_some() {
                order.reverse()
            } else {
                order
            }
        });
        matched = keyed.into_iter().map(|(_, c)| c).collect();
    }
    if let Some(n) = args.limit {
        matched.truncate(n);
    }

    let row = |card: &Card| -> Result<Row, String> {
        Ok(Row {
            id: card.id().to_string(),
            values: fields
                .iter()
                .map(|f| value(card, f, clock))
                .collect::<Result<_, _>>()?,
        })
    };

    let Some(group_by) = args.group_by.as_deref() else {
        let rows = matched.iter().map(row).collect::<Result<Vec<_>, _>>()?;
        return render::plain(format, &fields, &rows, clock);
    };
    let mut groups: Vec<Group> = Vec::new();
    for card in &matched {
        let key = group_key(group_by, card, &q, day_kind, clock)?;
        let r = row(card)?;
        match groups.iter_mut().find(|g| g.key == key) {
            Some(g) => g.rows.push(r),
            None => groups.push(Group { key, rows: vec![r] }),
        }
    }
    if group_by == "status" {
        groups.sort_by_key(|g| {
            g.key
                .as_deref()
                .and_then(|k| STATUSES.iter().position(|s| *s == k))
        });
    } else {
        groups.sort_by(|a, b| match (&a.key, &b.key) {
            (Some(x), Some(y)) => x.cmp(y),
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
        });
    }
    render::grouped(format, group_by, &fields, &groups, clock)
}

fn group_key(
    group_by: &str,
    card: &Card,
    q: &Query,
    day_kind: Option<TimeKind>,
    clock: &BoardClock,
) -> Result<Option<String>, String> {
    let wu = card.wu;
    Ok(match (group_by, day_kind) {
        ("status", _) => Some(wu.status.as_str().to_string()),
        ("type", _) => Some(wu.type_str().to_string()),
        ("prefix", _) => Some(card.prefix().to_string()),
        ("epic", _) => wu.epic.clone().filter(|e| !e.is_empty()),
        ("day", Some(kind)) => q
            .time_of(card, kind)?
            .map(|t| clock.local_date(&t).format("%Y-%m-%d").to_string()),
        ("week", Some(kind)) => q
            .time_of(card, kind)?
            .map(|t| clock.local_date(&t).format("%G-W%V").to_string()),
        (other, _) => return Err(format!("Unknown group-by '{other}'")),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::args::Args;

    #[test]
    fn args_parse_with_defaults() {
        let a: Args = serde_json::from_str("{}").unwrap();
        assert!(a.status.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn list_args_take_a_comma_string_or_an_array() {
        let a: Args =
            serde_json::from_str(r#"{"status":"backlog, testing","type":["task","bug"]}"#).unwrap();
        assert_eq!(
            a.status.unwrap().values().to_vec(),
            vec!["backlog", "testing"]
        );
        assert_eq!(a.r#type.unwrap().values().to_vec(), vec!["task", "bug"]);
    }

    #[test]
    fn unknown_args_are_rejected() {
        assert!(serde_json::from_str::<Args>(r#"{"doneSinse":"2d"}"#).is_err());
    }
}

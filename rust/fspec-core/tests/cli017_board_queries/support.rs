//! Fixture boards and a fixed clock for the CLI-017 board-query tests.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use std::path::Path;

use chrono::{DateTime, Utc};
use codelet_fspec_core::commands::list_work_units::{run_at, BoardClock, BoardZone};
use serde_json::{json, Map, Value};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Failure = Box<dyn std::error::Error>;

/// Now for every scenario that names one.
pub const NOW: &str = "2026-11-07T06:00:00Z";

pub fn utc(text: &str) -> Result<DateTime<Utc>, Failure> {
    Ok(text.parse::<DateTime<Utc>>()?)
}

pub fn clock_in(zone: &str) -> Result<BoardClock, Failure> {
    let tz: chrono_tz::Tz = zone.parse().map_err(|e| format!("zone {zone}: {e}"))?;
    Ok(BoardClock::fixed(utc(NOW)?, BoardZone::Named(tz)))
}

pub fn brisbane() -> Result<BoardClock, Failure> {
    clock_in("Australia/Brisbane")
}

/// A card with the given id, type and status, created and updated at
/// 2026-10-01T00:00:00.000Z, with `extra` merged over it.
pub fn card(id: &str, kind: &str, status: &str, extra: Value) -> Value {
    let mut base = json!({
        "id": id,
        "title": format!("{id} title"),
        "type": kind,
        "status": status,
        "createdAt": "2026-10-01T00:00:00.000Z",
        "updatedAt": "2026-10-01T00:00:00.000Z",
    });
    if let (Some(obj), Value::Object(more)) = (base.as_object_mut(), extra) {
        for (k, v) in more {
            obj.insert(k, v);
        }
    }
    base
}

/// stateHistory entries from (state, timestamp) pairs.
pub fn history(entries: &[(&str, &str)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(state, at)| json!({"state": state, "timestamp": at}))
            .collect(),
    )
}

fn prefix_of(id: &str) -> &str {
    id.rsplit_once('-').map_or(id, |(p, _)| p)
}

/// Write spec/work-units.json, spec/prefixes.json (every card prefix plus
/// `extra_prefixes`) and spec/epics.json (`epics`).
pub fn write_board_with(
    root: &Path,
    cards: &[Value],
    extra_prefixes: &[&str],
    epics: &[&str],
) -> TestResult {
    let mut units = Map::new();
    let mut prefixes = Map::new();
    for c in cards {
        let id = c["id"].as_str().ok_or("card without id")?.to_string();
        let p = prefix_of(&id).to_string();
        prefixes.insert(
            p.clone(),
            json!({"prefix": p, "description": "fixture", "createdAt": "2026-10-01T00:00:00.000Z"}),
        );
        units.insert(id, c.clone());
    }
    for p in extra_prefixes {
        prefixes.insert(
            (*p).to_string(),
            json!({"prefix": p, "description": "fixture", "createdAt": "2026-10-01T00:00:00.000Z"}),
        );
    }
    let mut epic_map = Map::new();
    for e in epics {
        epic_map.insert(
            (*e).to_string(),
            json!({"id": e, "title": e, "createdAt": "2026-10-01T00:00:00.000Z", "workUnits": []}),
        );
    }
    let board = json!({
        "version": "0.7.1",
        "meta": {"version": "1.0.0", "lastUpdated": "2026-10-01T00:00:00.000Z"},
        "workUnits": units,
        "states": {"backlog": [], "specifying": [], "testing": [], "implementing": [],
                   "validating": [], "done": [], "blocked": []},
    });
    std::fs::create_dir_all(root.join("spec"))?;
    std::fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    std::fs::write(
        root.join("spec/prefixes.json"),
        serde_json::to_string_pretty(&json!({ "prefixes": prefixes }))?,
    )?;
    std::fs::write(
        root.join("spec/epics.json"),
        serde_json::to_string_pretty(&json!({ "epics": epic_map }))?,
    )?;
    Ok(())
}

pub fn board(cards: &[Value]) -> Result<tempfile::TempDir, Failure> {
    let dir = tempfile::tempdir()?;
    write_board_with(dir.path(), cards, &[], &["board-queries"])?;
    Ok(dir)
}

fn runtime() -> Result<tokio::runtime::Runtime, Failure> {
    Ok(tokio::runtime::Builder::new_current_thread().build()?)
}

/// Run the core query and return its output or its error text.
pub fn query(root: &Path, args: Value, clock: &BoardClock) -> Result<String, String> {
    let rt = runtime().map_err(|e| e.to_string())?;
    rt.block_on(run_at(&args.to_string(), root, clock))
        .map_err(|e| e.to_string())
}

/// Run a query that must succeed.
pub fn ok(root: &Path, args: Value, clock: &BoardClock) -> Result<String, Failure> {
    query(root, args, clock).map_err(|e| format!("query failed: {e}").into())
}

/// Run a query that must fail, returning the error text.
pub fn err(root: &Path, args: Value, clock: &BoardClock) -> Result<String, Failure> {
    match query(root, args, clock) {
        Ok(out) => Err(format!("query succeeded but should fail:\n{out}").into()),
        Err(e) => Ok(e),
    }
}

/// The ids printed by `--format ids`.
pub fn ids(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn json_out(out: &str) -> Result<Value, Failure> {
    Ok(serde_json::from_str(out)?)
}

/// The workUnits array of `--format json` output.
pub fn units(out: &str) -> Result<Vec<Value>, Failure> {
    let v = json_out(out)?;
    Ok(v["workUnits"]
        .as_array()
        .ok_or_else(|| format!("no workUnits array in {out}"))?
        .clone())
}

pub fn unit<'a>(list: &'a [Value], id: &str) -> Result<&'a Value, Failure> {
    list.iter()
        .find(|u| u["id"] == id)
        .ok_or_else(|| format!("{id} missing from {list:?}").into())
}

/// The (key, count) rows of a grouped table output (header and the
/// trailing total line skipped).
pub fn group_rows(out: &str) -> Vec<(String, String)> {
    out.lines()
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}

pub fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

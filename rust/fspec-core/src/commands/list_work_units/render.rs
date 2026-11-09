//! Rendering a board query (CLI-017): table, ids, json, jsonl, count and
//! csv, plain or grouped.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use serde_json::{json, Map, Value};

use super::clock::BoardClock;
use super::fields::{cell, json_value, Field, Val};

pub(super) const FORMATS: &[&str] = &["table", "ids", "json", "jsonl", "count", "csv"];

/// One matched card: its id and the value of each output field.
pub(super) struct Row {
    pub(super) id: String,
    pub(super) values: Vec<Option<Val>>,
}

/// A group: its key (`None` for cards without the grouped field) and rows.
pub(super) struct Group {
    pub(super) key: Option<String>,
    pub(super) rows: Vec<Row>,
}

fn object(fields: &[Field], row: &Row, clock: &BoardClock) -> Value {
    let mut obj = Map::new();
    for (f, v) in fields.iter().zip(&row.values) {
        if let Some(v) = v {
            obj.insert(f.name.clone(), json_value(v, clock));
        }
    }
    Value::Object(obj)
}

fn pretty(v: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| format!("failed to serialize result: {e}"))
}

/// Left-aligned columns two spaces apart; the last column is not padded.
fn columns(rows: &[Vec<String>]) -> String {
    let n = rows.first().map_or(0, Vec::len);
    let widths: Vec<usize> = (0..n)
        .map(|i| rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0))
        .collect();
    rows.iter()
        .map(|r| {
            let mut line = String::new();
            for (i, c) in r.iter().enumerate() {
                line.push_str(c);
                if i + 1 < n {
                    let pad = widths[i] - c.chars().count() + 2;
                    line.push_str(&" ".repeat(pad));
                }
            }
            line.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn total(n: usize) -> String {
    if n == 1 {
        "1 work unit".to_string()
    } else {
        format!("{n} work units")
    }
}

fn table_cell(v: &Option<Val>, clock: &BoardClock) -> String {
    v.as_ref()
        .map(|v| cell(v, clock).replace(['\n', '\r'], " "))
        .unwrap_or_default()
}

fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

pub(super) fn plain(
    format: &str,
    fields: &[Field],
    rows: &[Row],
    clock: &BoardClock,
) -> Result<String, String> {
    Ok(match format {
        "ids" => rows
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        "count" => rows.len().to_string(),
        "json" => pretty(&json!({
            "workUnits": rows.iter().map(|r| object(fields, r, clock)).collect::<Vec<_>>()
        }))?,
        "jsonl" => rows
            .iter()
            .map(|r| object(fields, r, clock).to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        "csv" => {
            let mut lines = vec![fields
                .iter()
                .map(|f| csv_cell(&f.name))
                .collect::<Vec<_>>()
                .join(",")];
            for r in rows {
                lines.push(
                    r.values
                        .iter()
                        .map(|v| csv_cell(&v.as_ref().map(|v| cell(v, clock)).unwrap_or_default()))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            lines.join("\n")
        }
        _ => {
            if rows.is_empty() {
                return Ok("No work units found".to_string());
            }
            let mut grid = vec![fields
                .iter()
                .map(|f| f.name.to_uppercase())
                .collect::<Vec<_>>()];
            for r in rows {
                grid.push(r.values.iter().map(|v| table_cell(v, clock)).collect());
            }
            format!("{}\n\n{}", columns(&grid), total(rows.len()))
        }
    })
}

const NONE_KEY: &str = "(none)";

pub(super) fn grouped(
    format: &str,
    group_by: &str,
    fields: &[Field],
    groups: &[Group],
    clock: &BoardClock,
) -> Result<String, String> {
    let key_text = |g: &Group| g.key.clone().unwrap_or_else(|| NONE_KEY.to_string());
    Ok(match format {
        "json" => pretty(&json!({
            "groupBy": group_by,
            "groups": groups.iter().map(|g| json!({
                "key": g.key,
                "count": g.rows.len(),
                "workUnits": g.rows.iter().map(|r| object(fields, r, clock)).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))?,
        "count" => groups
            .iter()
            .map(|g| format!("{} {}", key_text(g), g.rows.len()))
            .collect::<Vec<_>>()
            .join("\n"),
        "csv" => {
            let mut lines = vec![format!("{},count", csv_cell(group_by))];
            lines.extend(
                groups
                    .iter()
                    .map(|g| format!("{},{}", csv_cell(&key_text(g)), g.rows.len())),
            );
            lines.join("\n")
        }
        "table" => {
            if groups.is_empty() {
                return Ok("No work units found".to_string());
            }
            let mut grid = vec![vec![group_by.to_uppercase(), "COUNT".to_string()]];
            grid.extend(
                groups
                    .iter()
                    .map(|g| vec![key_text(g), g.rows.len().to_string()]),
            );
            let n: usize = groups.iter().map(|g| g.rows.len()).sum();
            format!("{}\n\n{}", columns(&grid), total(n))
        }
        _ => return Err("--group-by works with --format table, count, csv or json".to_string()),
    })
}

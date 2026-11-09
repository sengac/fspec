//! The output fields of a board query (CLI-017): the card's own fields and
//! the derived ones (doneAt, enteredAt:<status>, ageInStatus,
//! openQuestions, dependencies).
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use std::cmp::Ordering;

use serde_json::{json, Value};

use super::card::{Card, Time};
use super::clock::BoardClock;
use super::STATUSES;

/// Every field name, in the order the errors list them.
pub(super) const FIELD_NAMES: &[&str] = &[
    "id",
    "title",
    "type",
    "status",
    "epic",
    "prefix",
    "description",
    "estimate",
    "parent",
    "children",
    "createdAt",
    "updatedAt",
    "doneAt",
    "enteredAt:<status>",
    "ageInStatus",
    "openQuestions",
    "assumptions",
    "rules",
    "examples",
    "architectureNotes",
    "attachments",
    "dependsOn",
    "blocks",
    "blockedBy",
    "relatesTo",
    "dependencies",
];

pub(super) const DEFAULT_FIELDS: &[&str] = &["id", "type", "status", "epic", "title"];

const LINKS: &[&str] = &["dependsOn", "blocks", "blockedBy", "relatesTo"];

/// One output field: its name as given and the status of enteredAt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Field {
    pub(super) name: String,
    entered: Option<String>,
}

impl Field {
    pub(super) fn parse(name: &str, what: &str) -> Result<Self, String> {
        if let Some(status) = name.strip_prefix("enteredAt:") {
            if !STATUSES.contains(&status) {
                return Err(format!(
                    "Unknown status '{status}' in {name}. Valid statuses: {}",
                    STATUSES.join(", ")
                ));
            }
            return Ok(Self {
                name: name.to_string(),
                entered: Some(status.to_string()),
            });
        }
        if name == "enteredAt:<status>" || !FIELD_NAMES.contains(&name) {
            return Err(format!(
                "Unknown {what} '{name}'. Valid fields: {}",
                FIELD_NAMES.join(", ")
            ));
        }
        Ok(Self {
            name: name.to_string(),
            entered: None,
        })
    }
}

/// The value of a field on a card.
#[derive(Debug, Clone)]
pub(super) enum Val {
    Text(String),
    Num(i64),
    Time(Time),
    /// Seconds the card has spent in its status.
    Age(i64),
    /// Items with an id, e.g. open questions or rules.
    Items(Vec<(Option<u64>, String)>),
    Strings(Vec<String>),
    Links(Vec<(&'static str, Vec<String>)>),
}

fn non_empty(v: Vec<String>) -> Option<Val> {
    (!v.is_empty()).then_some(Val::Strings(v))
}

fn items(v: Vec<(Option<u64>, String)>) -> Option<Val> {
    (!v.is_empty()).then_some(Val::Items(v))
}

/// The value of `field` on `card`; `None` when the card does not have it.
pub(super) fn value(card: &Card, field: &Field, clock: &BoardClock) -> Result<Option<Val>, String> {
    let wu = card.wu;
    if let Some(status) = &field.entered {
        return Ok(card.entered_at(status)?.map(Val::Time));
    }
    let text = |s: &str| Some(Val::Text(s.to_string()));
    Ok(match field.name.as_str() {
        "id" => text(&wu.id),
        "title" => text(&wu.title),
        "type" => text(wu.type_str()),
        "status" => text(wu.status.as_str()),
        "epic" => wu.epic.as_deref().filter(|e| !e.is_empty()).and_then(text),
        "prefix" => text(card.prefix()),
        "description" => card.text_field("description").and_then(text),
        "estimate" => card.estimate().map(Val::Num),
        "parent" => card.text_field("parent").and_then(text),
        "children" | "attachments" | "dependsOn" | "blocks" | "blockedBy" | "relatesTo" => {
            non_empty(card.strings(&field.name))
        }
        "createdAt" => card.created()?.map(Val::Time),
        "updatedAt" => card.updated()?.map(Val::Time),
        "doneAt" => card.done_at()?.map(Val::Time),
        "ageInStatus" => card
            .in_status_since()?
            .map(|since| Val::Age((clock.now - since).num_seconds())),
        "openQuestions" => items(
            card.open_questions()?
                .into_iter()
                .map(|(id, t)| (Some(id), t))
                .collect(),
        ),
        "assumptions" => non_empty(
            card.active_items("assumptions")?
                .into_iter()
                .map(|(_, t)| t)
                .collect(),
        ),
        "rules" | "examples" | "architectureNotes" => items(card.active_items(&field.name)?),
        "dependencies" => {
            let links: Vec<(&'static str, Vec<String>)> = LINKS
                .iter()
                .map(|k| (*k, card.strings(k)))
                .filter(|(_, v)| !v.is_empty())
                .collect();
            (!links.is_empty()).then_some(Val::Links(links))
        }
        other => return Err(format!("Unknown field '{other}'")),
    })
}

/// `3d 4h 12m`, dropping leading zero units.
pub(super) fn age_text(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let s = seconds.unsigned_abs();
    let (d, h, m) = (s / 86_400, (s % 86_400) / 3_600, (s % 3_600) / 60);
    if d > 0 {
        format!("{sign}{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{sign}{h}h {m}m")
    } else {
        format!("{sign}{m}m")
    }
}

fn item_text((id, text): &(Option<u64>, String)) -> String {
    match id {
        Some(id) => format!("[{id}] {text}"),
        None => text.clone(),
    }
}

/// The value as one table or csv cell; lists are joined with `; `.
pub(super) fn cell(v: &Val, clock: &BoardClock) -> String {
    match v {
        Val::Text(s) => s.clone(),
        Val::Num(n) => n.to_string(),
        Val::Time(t) => clock.local_text(t),
        Val::Age(s) => age_text(*s),
        Val::Items(list) => list.iter().map(item_text).collect::<Vec<_>>().join("; "),
        Val::Strings(list) => list.join("; "),
        Val::Links(links) => links
            .iter()
            .map(|(k, ids)| format!("{k}: {}", ids.join(", ")))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

/// The value as JSON.
pub(super) fn json_value(v: &Val, clock: &BoardClock) -> Value {
    match v {
        Val::Text(s) => json!(s),
        Val::Num(n) => json!(n),
        Val::Time(t) => json!(clock.local_text(t)),
        Val::Age(s) => json!(age_text(*s)),
        Val::Items(list) => Value::Array(
            list.iter()
                .map(|(id, text)| match id {
                    Some(id) => json!({"id": id, "text": text}),
                    None => json!({ "text": text }),
                })
                .collect(),
        ),
        Val::Strings(list) => json!(list),
        Val::Links(links) => Value::Object(
            links
                .iter()
                .map(|(k, ids)| ((*k).to_string(), json!(ids)))
                .collect(),
        ),
    }
}

/// Sort order of two values of one field; absent values sort last.
pub(super) fn compare(a: &Option<Val>, b: &Option<Val>, clock: &BoardClock) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(Val::Num(x)), Some(Val::Num(y))) | (Some(Val::Age(x)), Some(Val::Age(y))) => x.cmp(y),
        (Some(Val::Time(x)), Some(Val::Time(y))) => x.cmp(y),
        (Some(x), Some(y)) => cell(x, clock).cmp(&cell(y, clock)),
    }
}

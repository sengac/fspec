//! BUG-633: a work unit's assumptions as stable-id items.
//!
//! Feature: spec/features/remove-and-restore-assumptions.feature
//!
//! Assumptions were plain strings. They are now stored like rules —
//! `{id, text, deleted, createdAt, deletedAt?}` with a `nextAssumptionId`
//! counter — so they can be soft-deleted and restored by a stable id.
//! Product owner, 2026-11-08: a card's string assumptions are converted
//! (each keeping its index as its id, `createdAt` the conversion time) the
//! first time a command writes that card's assumptions; until then every
//! reader takes a string at position `i` as active assumption `i`.

use serde_json::{Map, Value};

/// The work-unit key holding the assumptions.
pub const KEY: &str = "assumptions";
/// The work-unit key holding the next assumption id.
pub const NEXT_ID_KEY: &str = "nextAssumptionId";

/// One assumption as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    pub id: u64,
    pub text: String,
    pub deleted: bool,
    pub deleted_at: Option<String>,
}

/// Every assumption in `value` (the work unit's `assumptions`), in stored
/// order: a string at position `i` is active assumption `i`; an object
/// needs a numeric `id` and a string `text`. Anything else is an error
/// naming the work unit.
pub fn read(work_unit_id: &str, value: Option<&Value>) -> Result<Vec<Assumption>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| format!("Work unit '{work_unit_id}' has assumptions that are not a list"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            Value::String(text) => Ok(Assumption {
                id: index as u64,
                text: text.clone(),
                deleted: false,
                deleted_at: None,
            }),
            Value::Object(object) => {
                let id = object.get("id").and_then(Value::as_u64).ok_or_else(|| {
                    format!("Work unit '{work_unit_id}' has an assumption without a numeric id: {item}")
                })?;
                let text = object.get("text").and_then(Value::as_str).ok_or_else(|| {
                    format!("Work unit '{work_unit_id}' has an assumption without text: {item}")
                })?;
                Ok(Assumption {
                    id,
                    text: text.to_owned(),
                    deleted: object.get("deleted").and_then(Value::as_bool) == Some(true),
                    deleted_at: object
                        .get("deletedAt")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                })
            }
            other => Err(format!(
                "Work unit '{work_unit_id}' has an assumption that is neither text nor an object: {other}"
            )),
        })
        .collect()
}

/// The assumptions of `value` that are not deleted.
pub fn active(work_unit_id: &str, value: Option<&Value>) -> Result<Vec<Assumption>, String> {
    Ok(read(work_unit_id, value)?
        .into_iter()
        .filter(|assumption| !assumption.deleted)
        .collect())
}

fn object(id: u64, text: &str, created_at: &str) -> Value {
    let mut item = Map::new();
    item.insert("id".to_owned(), Value::from(id));
    item.insert("text".to_owned(), Value::String(text.to_owned()));
    item.insert("deleted".to_owned(), Value::Bool(false));
    item.insert("createdAt".to_owned(), Value::String(created_at.to_owned()));
    Value::Object(item)
}

/// Convert the work unit's string assumptions to objects in place (each
/// keeping its index as its id, created `now`) and make sure
/// `nextAssumptionId` is past every id; returns that next id. Called by
/// every command that writes a card's assumptions, before it changes them.
pub fn convert(
    work_unit_id: &str,
    extra: &mut Map<String, Value>,
    now: &str,
) -> Result<u64, String> {
    let existing = read(work_unit_id, extra.get(KEY))?;
    let items: Vec<Value> = match extra.get(KEY).and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                Value::String(text) => object(index as u64, text, now),
                other => other.clone(),
            })
            .collect(),
        None => Vec::new(),
    };
    let past_ids = existing.iter().map(|a| a.id + 1).max().unwrap_or(0);
    let stored_next = match extra.get(NEXT_ID_KEY) {
        None => 0,
        Some(value) => value.as_u64().ok_or_else(|| {
            format!("Work unit '{work_unit_id}' has a {NEXT_ID_KEY} that is not a whole number: {value}")
        })?,
    };
    let next = stored_next.max(past_ids);
    extra.insert(KEY.to_owned(), Value::Array(items));
    extra.insert(NEXT_ID_KEY.to_owned(), Value::from(next));
    Ok(next)
}

/// Append assumption `text` (converting the card first) and return the
/// new item's id.
pub fn push(
    work_unit_id: &str,
    extra: &mut Map<String, Value>,
    text: &str,
    now: &str,
) -> Result<u64, String> {
    let id = convert(work_unit_id, extra, now)?;
    if let Some(items) = extra.get_mut(KEY).and_then(Value::as_array_mut) {
        items.push(object(id, text, now));
    }
    extra.insert(NEXT_ID_KEY.to_owned(), Value::from(id + 1));
    Ok(id)
}

/// Append the assumptions of an imported example map (`incoming`, as
/// `export-example-map` writes them: strings or items) as new items with
/// the card's next ids, keeping a removed item removed. Returns how many
/// were imported. An `incoming` that is not a list imports nothing.
pub fn import(
    work_unit_id: &str,
    extra: &mut Map<String, Value>,
    incoming: Option<&Value>,
    now: &str,
) -> Result<usize, String> {
    let Some(incoming @ Value::Array(_)) = incoming else {
        return Ok(0);
    };
    let items = read(work_unit_id, Some(incoming))
        .map_err(|reason| format!("the imported example map: {reason}"))?;
    convert(work_unit_id, extra, now)?;
    for item in &items {
        let id = push(work_unit_id, extra, &item.text, now)?;
        if item.deleted {
            if let Some(entry) = extra
                .get_mut(KEY)
                .and_then(Value::as_array_mut)
                .and_then(|entries| entries.last_mut())
                .and_then(Value::as_object_mut)
                .filter(|entry| entry.get("id").and_then(Value::as_u64) == Some(id))
            {
                entry.insert("deleted".to_owned(), Value::Bool(true));
                if let Some(deleted_at) = &item.deleted_at {
                    entry.insert("deletedAt".to_owned(), Value::String(deleted_at.clone()));
                }
            }
        }
    }
    Ok(items.len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use serde_json::json;

    #[test]
    fn strings_read_as_active_items_at_their_index() {
        let value = json!(["A", {"id": 1, "text": "B", "deleted": true, "deletedAt": "t"}, "C"]);
        let all = read("X-1", Some(&value)).unwrap();
        assert_eq!(all.iter().map(|a| a.id).collect::<Vec<_>>(), vec![0, 1, 2]);
        let active = active("X-1", Some(&value)).unwrap();
        assert_eq!(
            active.iter().map(|a| a.text.as_str()).collect::<Vec<_>>(),
            vec!["A", "C"]
        );
    }

    #[test]
    fn convert_keeps_indices_and_sets_the_counter() {
        let mut extra = Map::new();
        extra.insert(KEY.into(), json!(["A", "B"]));
        convert("X-1", &mut extra, "now").unwrap();
        assert_eq!(
            extra[KEY],
            json!([
                {"id": 0, "text": "A", "deleted": false, "createdAt": "now"},
                {"id": 1, "text": "B", "deleted": false, "createdAt": "now"}
            ])
        );
        assert_eq!(extra[NEXT_ID_KEY], json!(2));
        assert_eq!(push("X-1", &mut extra, "C", "later").unwrap(), 2);
        assert_eq!(extra[NEXT_ID_KEY], json!(3));
    }

    #[test]
    fn a_malformed_item_is_an_error_naming_the_card() {
        let error = read("X-1", Some(&json!([7]))).unwrap_err();
        assert!(error.contains("X-1"), "{error}");
        let error = read("X-1", Some(&json!([{"text": "no id"}]))).unwrap_err();
        assert!(error.contains("numeric id"), "{error}");
    }
}

//! The facts a board query reads from one card (CLI-017): its times, its
//! stateHistory, its active items and its links. Times are parsed once, on
//! first use, so a malformed value fails only a query that needs it.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use std::cell::OnceCell;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::types::work_unit::{WorkUnit, WorkUnitStatus};

pub(super) type Time = DateTime<Utc>;

pub(super) struct Card<'a> {
    pub(super) wu: &'a WorkUnit,
    created: OnceCell<Result<Option<Time>, String>>,
    updated: OnceCell<Result<Option<Time>, String>>,
    history: OnceCell<Result<Vec<(String, Time)>, String>>,
}

fn parse_time(id: &str, field: &str, text: &str) -> Result<Time, String> {
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| format!("Work unit '{id}' has an invalid {field} '{text}': {e}"))
}

fn optional_time(id: &str, field: &str, text: &str) -> Result<Option<Time>, String> {
    if text.is_empty() {
        Ok(None)
    } else {
        parse_time(id, field, text).map(Some)
    }
}

impl<'a> Card<'a> {
    pub(super) fn new(wu: &'a WorkUnit) -> Self {
        Self {
            wu,
            created: OnceCell::new(),
            updated: OnceCell::new(),
            history: OnceCell::new(),
        }
    }

    pub(super) fn id(&self) -> &'a str {
        &self.wu.id
    }

    pub(super) fn prefix(&self) -> &'a str {
        self.wu
            .id
            .rsplit_once('-')
            .map_or(self.wu.id.as_str(), |(p, _)| p)
    }

    pub(super) fn created(&self) -> Result<Option<Time>, String> {
        self.created
            .get_or_init(|| optional_time(&self.wu.id, "createdAt", &self.wu.created_at))
            .clone()
    }

    pub(super) fn updated(&self) -> Result<Option<Time>, String> {
        self.updated
            .get_or_init(|| optional_time(&self.wu.id, "updatedAt", &self.wu.updated_at))
            .clone()
    }

    /// The (state, timestamp) entries of stateHistory, in board order.
    pub(super) fn history(&self) -> Result<&[(String, Time)], String> {
        self.history
            .get_or_init(|| self.read_history())
            .as_ref()
            .map(Vec::as_slice)
            .map_err(Clone::clone)
    }

    fn read_history(&self) -> Result<Vec<(String, Time)>, String> {
        let id = &self.wu.id;
        let Some(raw) = self.wu.extra.get("stateHistory") else {
            return Ok(Vec::new());
        };
        let entries = raw
            .as_array()
            .ok_or_else(|| format!("Work unit '{id}' has a stateHistory that is not a list"))?;
        entries
            .iter()
            .map(|e| {
                let state = e.get("state").and_then(Value::as_str);
                let at = e.get("timestamp").and_then(Value::as_str);
                match (state, at) {
                    (Some(state), Some(at)) => Ok((
                        state.to_string(),
                        parse_time(id, "stateHistory timestamp", at)?,
                    )),
                    _ => Err(format!(
                        "Work unit '{id}' has a stateHistory entry without state and timestamp: {e}"
                    )),
                }
            })
            .collect()
    }

    /// The last stateHistory entry, when the card is done and that entry is
    /// its transition into done.
    pub(super) fn done_at(&self) -> Result<Option<Time>, String> {
        if self.wu.status != WorkUnitStatus::Done {
            return Ok(None);
        }
        Ok(self
            .history()?
            .last()
            .filter(|(state, _)| state == "done")
            .map(|(_, at)| *at))
    }

    /// The last stateHistory transition into `status`.
    pub(super) fn entered_at(&self, status: &str) -> Result<Option<Time>, String> {
        Ok(self
            .history()?
            .iter()
            .rev()
            .find(|(state, _)| state == status)
            .map(|(_, at)| *at))
    }

    /// Since when the card has been in its current status: its last
    /// stateHistory entry when that is the current status, or its creation
    /// when it never changed status.
    pub(super) fn in_status_since(&self) -> Result<Option<Time>, String> {
        let history = self.history()?;
        match history.last() {
            None => self.created(),
            Some((state, at)) if state == self.wu.status.as_str() => Ok(Some(*at)),
            Some(_) => Ok(None),
        }
    }

    /// A list of strings such as dependsOn, children or attachments.
    pub(super) fn strings(&self, key: &str) -> Vec<String> {
        self.wu
            .extra
            .get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The items of rules, examples, architectureNotes or assumptions that
    /// are not deleted, as (id, text). A plain string item has no id.
    pub(super) fn active_items(&self, key: &str) -> Result<Vec<(Option<u64>, String)>, String> {
        // BUG-633: a string assumption at position i is assumption i.
        if key == super::super::assumption_items::KEY {
            return Ok(super::super::assumption_items::active(
                &self.wu.id,
                self.wu.extra.get(key),
            )?
            .into_iter()
            .map(|a| (Some(a.id), a.text))
            .collect());
        }
        let Some(raw) = self.wu.extra.get(key) else {
            return Ok(Vec::new());
        };
        let id = &self.wu.id;
        let items = raw
            .as_array()
            .ok_or_else(|| format!("Work unit '{id}' has a {key} value that is not a list"))?;
        let mut out = Vec::new();
        for item in items {
            match item {
                Value::String(s) => out.push((None, s.clone())),
                Value::Object(o) => {
                    if o.get("deleted").and_then(Value::as_bool) == Some(true) {
                        continue;
                    }
                    let text = o.get("text").and_then(Value::as_str).ok_or_else(|| {
                        format!("Work unit '{id}' has a {key} item without text: {item}")
                    })?;
                    out.push((o.get("id").and_then(Value::as_u64), text.to_string()));
                }
                other => {
                    return Err(format!(
                    "Work unit '{id}' has a {key} item that is neither text nor an object: {other}"
                ))
                }
            }
        }
        Ok(out)
    }

    /// Questions that are neither deleted nor answered (selected).
    pub(super) fn open_questions(&self) -> Result<Vec<(u64, String)>, String> {
        let Some(raw) = self.wu.extra.get("questions") else {
            return Ok(Vec::new());
        };
        let id = &self.wu.id;
        let items = raw
            .as_array()
            .ok_or_else(|| format!("Work unit '{id}' has questions that are not a list"))?;
        let mut out = Vec::new();
        for q in items {
            let o = q.as_object().ok_or_else(|| {
                format!("Work unit '{id}' has a question that is not an object: {q}")
            })?;
            let flag = |k: &str| o.get(k).and_then(Value::as_bool) == Some(true);
            if flag("deleted") || flag("selected") {
                continue;
            }
            match (
                o.get("id").and_then(Value::as_u64),
                o.get("text").and_then(Value::as_str),
            ) {
                (Some(qid), Some(text)) => out.push((qid, text.to_string())),
                _ => {
                    return Err(format!(
                        "Work unit '{id}' has a question without id and text: {q}"
                    ))
                }
            }
        }
        Ok(out)
    }

    pub(super) fn estimate(&self) -> Option<i64> {
        self.wu.extra.get("estimate").and_then(Value::as_i64)
    }

    pub(super) fn text_field(&self, key: &str) -> Option<&'a str> {
        self.wu
            .extra
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    }
}

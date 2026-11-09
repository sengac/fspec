//! BOARD-024 — readable projection of a complete canonical work-unit record.
//!
//! Known sections render first in a stable order (overview, description,
//! Example Mapping, Event Storm, relationships, attachments, history,
//! accounting, timestamps); every remaining stored field — including
//! unknown and legacy ones — renders generically under `Additional Fields`
//! so nothing stored on the card is hidden. Display text is sanitized; link
//! ids and attachment paths keep their original values as [`Target`]s.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

pub use super::labels::humanize;
use super::labels::scalar;
use super::sanitize::sanitize;

use super::document_storm as storm;

/// Activatable selection target with its original (unsanitized) value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Link(String),
    Attachment(String),
}

/// What a logical line represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Heading,
    Text,
    /// Selectable row for `targets[index]`.
    Item(usize),
}

/// One logical (pre-wrap) line.
#[derive(Debug, Clone)]
pub struct DocLine {
    pub indent: u16,
    pub text: String,
    pub kind: LineKind,
}

/// Projected document: lines plus the selectable targets they reference.
#[derive(Debug, Default)]
pub struct Document {
    pub lines: Vec<DocLine>,
    pub targets: Vec<Target>,
}

const LINK_FIELDS: [&str; 6] = [
    "parent",
    "children",
    "dependsOn",
    "blocks",
    "blockedBy",
    "relatesTo",
];

const SECTIONS: [(&str, &[&str]); 6] = [
    ("Overview", &["type", "status", "epic", "estimate"]),
    ("Description", &["description"]),
    (
        "Example Mapping",
        &[
            "userStory",
            "rules",
            "examples",
            "questions",
            "assumptions",
            "architectureNotes",
        ],
    ),
    ("History", &["stateHistory", "blockedReason"]),
    ("Accounting", &["iterations", "actualTokens"]),
    ("Timestamps", &["createdAt", "updatedAt"]),
];

pub(super) struct Builder {
    doc: Document,
}

impl Builder {
    pub(super) fn heading(&mut self, text: &str) {
        self.doc.lines.push(DocLine {
            indent: 0,
            text: sanitize(text).replace('\n', " "),
            kind: LineKind::Heading,
        });
    }

    pub(super) fn text(&mut self, indent: u16, text: &str) {
        for line in sanitize(text).split('\n') {
            self.doc.lines.push(DocLine {
                indent,
                text: line.to_owned(),
                kind: LineKind::Text,
            });
        }
    }

    fn target(&mut self, indent: u16, target: Target) {
        let label = match &target {
            Target::Link(id) | Target::Attachment(id) => id.clone(),
        };
        self.doc.lines.push(DocLine {
            indent,
            text: format!("▸ {}", sanitize(&label).replace('\n', " ")),
            kind: LineKind::Item(self.doc.targets.len()),
        });
        self.doc.targets.push(target);
    }

    /// Generic, lossless rendering of any JSON value under `label`.
    pub(super) fn value(&mut self, indent: u16, label: &str, value: &Value) {
        match value {
            Value::String(text) if text.contains('\n') => {
                self.text(indent, &format!("{label}:"));
                self.text(indent + 2, text);
            }
            Value::Array(items) if items.is_empty() => {
                self.text(indent, &format!("{label}: (none)"));
            }
            Value::Array(items) => {
                self.text(indent, &format!("{label}:"));
                for item in items {
                    self.entry(indent + 2, item, &[]);
                }
            }
            Value::Object(map) if map.is_empty() => self.text(indent, &format!("{label}: {{}}")),
            Value::Object(map) => {
                self.text(indent, &format!("{label}:"));
                for (key, nested) in map {
                    self.value(indent + 2, &humanize(key), nested);
                }
            }
            other => {
                let text = scalar(other).unwrap_or_default();
                self.text(indent, &format!("{label}: {text}"));
            }
        }
    }

    /// One list entry: `text` items (rules, questions, storm items) get a
    /// readable head with answers and deleted/historical markers; other
    /// fields follow as indented sub-fields (`skip` omits grouping keys).
    pub(super) fn entry(&mut self, indent: u16, item: &Value, skip: &[&str]) {
        let Value::Object(map) = item else {
            match scalar(item) {
                Some(text) => self.text(indent, &format!("• {text}")),
                None => self.value(indent, "•", item),
            }
            return;
        };
        let mut used: BTreeSet<&str> = skip.iter().copied().collect();
        let mut head = String::from("•");
        if let Some(text) = map.get("text").and_then(Value::as_str) {
            head.push(' ');
            head.push_str(text);
            used.insert("text");
            if let Some(answer) = map.get("answer").and_then(scalar) {
                head.push_str(&format!(" → Answer: {answer}"));
                used.insert("answer");
            }
        } else {
            let parts: Vec<String> = map
                .iter()
                .filter(|(key, value)| !used.contains(key.as_str()) && scalar(value).is_some())
                .map(|(key, value)| {
                    format!("{}: {}", humanize(key), scalar(value).unwrap_or_default())
                })
                .collect();
            head.push(' ');
            head.push_str(&parts.join(", "));
            used.extend(
                map.iter()
                    .filter(|(_, value)| scalar(value).is_some())
                    .map(|(key, _)| key.as_str()),
            );
        }
        if map.get("deleted").and_then(Value::as_bool) == Some(true) {
            head.push_str(" [deleted — historical]");
            used.insert("deleted");
        }
        self.text(indent, &head);
        for (key, value) in map {
            if !used.contains(key.as_str()) {
                self.value(indent + 4, &humanize(key), value);
            }
        }
    }

    fn links(&mut self, label: &str, value: &Value) {
        self.text(0, &format!("{label}:"));
        let ids: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        for id in ids {
            match id.as_str() {
                Some(id) => self.target(2, Target::Link(id.to_owned())),
                None => self.entry(2, id, &[]),
            }
        }
    }
}

/// Project a complete canonical record into a readable document.
pub fn project(record: &Map<String, Value>) -> Document {
    let mut builder = Builder {
        doc: Document::default(),
    };
    let mut used: BTreeSet<&str> = ["id", "title"].into_iter().collect();
    let mut section = |builder: &mut Builder, heading: &str, keys: &[&'static str]| {
        let present: Vec<&'static str> = keys
            .iter()
            .copied()
            .filter(|key| record.contains_key(*key))
            .collect();
        if present.is_empty() {
            return;
        }
        builder.heading(heading);
        for key in present {
            used.insert(key);
            if let Some(value) = record.get(key) {
                builder.value(0, &humanize(key), value);
            }
        }
    };
    for (heading, keys) in SECTIONS.iter().take(3) {
        section(&mut builder, heading, keys);
    }
    if let Some(value) = record.get("eventStorm") {
        storm::project(&mut builder, value);
    }
    if LINK_FIELDS.iter().any(|key| record.contains_key(*key)) {
        builder.heading("Relationships");
        for key in LINK_FIELDS {
            if let Some(value) = record.get(key) {
                builder.links(&humanize(key), value);
            }
        }
    }
    if let Some(value) = record.get("attachments") {
        builder.heading("Attachments");
        match value.as_array() {
            Some(paths) if paths.iter().all(Value::is_string) => {
                for path in paths.iter().filter_map(Value::as_str) {
                    builder.target(2, Target::Attachment(path.to_owned()));
                }
            }
            _ => builder.value(0, "Attachments", value),
        }
    }
    for (heading, keys) in SECTIONS.iter().skip(3) {
        section(&mut builder, heading, keys);
    }
    let known: BTreeSet<&str> = used
        .into_iter()
        .chain(LINK_FIELDS)
        .chain(["eventStorm", "attachments"])
        .collect();
    let extra: Vec<(&String, &Value)> = record
        .iter()
        .filter(|(key, _)| !known.contains(key.as_str()))
        .collect();
    if !extra.is_empty() {
        builder.heading("Additional Fields");
        for (key, value) in extra {
            builder.value(0, &humanize(key), value);
        }
    }
    builder.doc
}

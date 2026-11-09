//! BOARD-024 — Event Storm section: board metadata plus every item grouped
//! under its type heading with its type-specific fields.

use serde_json::Value;

use super::document::{humanize, Builder};

const GROUPS: [(&str, &str); 7] = [
    ("event", "Domain Events"),
    ("command", "Commands"),
    ("policy", "Policies"),
    ("hotspot", "Hotspots"),
    ("aggregate", "Aggregates"),
    ("bounded_context", "Bounded Contexts"),
    ("external_system", "External Systems"),
];

fn item_type(item: &Value) -> Option<&str> {
    item.get("type").and_then(Value::as_str)
}

pub(super) fn project(builder: &mut Builder, storm: &Value) {
    builder.heading("Event Storm");
    let Some(map) = storm.as_object() else {
        builder.value(0, "Event Storm", storm);
        return;
    };
    for (key, value) in map.iter().filter(|(key, _)| key.as_str() != "items") {
        builder.value(0, &humanize(key), value);
    }
    let Some(value) = map.get("items") else {
        return;
    };
    let Some(items) = value.as_array() else {
        builder.value(0, "Items", value);
        return;
    };
    if items.is_empty() {
        builder.text(0, "Items: (none)");
    }
    for (kind, heading) in GROUPS {
        let group: Vec<&Value> = items
            .iter()
            .filter(|item| item_type(item) == Some(kind))
            .collect();
        if group.is_empty() {
            continue;
        }
        builder.text(0, &format!("{heading} ({})", group.len()));
        for item in group {
            builder.entry(2, item, &["type"]);
        }
    }
    let other: Vec<&Value> = items
        .iter()
        .filter(|item| !GROUPS.iter().any(|(kind, _)| item_type(item) == Some(kind)))
        .collect();
    if !other.is_empty() {
        builder.text(0, &format!("Other Items ({})", other.len()));
        for item in other {
            builder.entry(2, item, &[]);
        }
    }
}

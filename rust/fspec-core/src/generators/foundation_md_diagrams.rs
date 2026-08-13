//! Mermaid diagram generators for `FOUNDATION.md` — Rust port of the two
//! private helpers in `src/generators/foundation-md.ts`
//! (`generateBoundedContextMermaid` + `generateContextEventFlowMermaid`).
//!
//! Both produce the exact same line-by-line output the TypeScript helpers
//! emit (joined with `\n`), so the surrounding `generate_foundation_md`
//! renderer is byte-for-byte identical to the TS reference.

use serde_json::Value;

use super::foundation_md_util::{js_str, js_strict_eq, truthy};

/// Port of `generateBoundedContextMermaid`. `bounded_contexts` is the
/// already-filtered (`type == "bounded_context" && !deleted`) slice of
/// Event Storm items, in document order.
pub(super) fn bounded_context_mermaid(bounded_contexts: &[&Value]) -> String {
    let mut lines: Vec<String> = vec!["graph TB".to_string()];

    // Map of context text → node id (`BC{i+1}`) for relationship wiring.
    let node_id = |i: usize| format!("BC{}", i + 1);

    // Generate nodes for each bounded context with a brief description.
    for (i, context) in bounded_contexts.iter().enumerate() {
        let text = context.get("text").map(js_str).unwrap_or_default();
        let description = match text.as_str() {
            "Work Management" => "Stories, Epics, Dependencies",
            "Specification" => "Features, Scenarios, Steps",
            "Discovery" => "Rules, Examples, Questions",
            "Event Storming" => "Events, Commands, Policies",
            "Foundation" => "Vision, Capabilities, Personas",
            "Testing & Validation" => "Coverage, Test Mappings",
            _ => "",
        };
        // Only add `<br/>` and description if description exists.
        let label = if description.is_empty() {
            text.clone()
        } else {
            format!("{text}<br/>{description}")
        };
        lines.push(format!("  {}[\"{label}\"]", node_id(i)));
    }

    lines.push(String::new());

    // Add relationships between contexts (only when both endpoints exist).
    let relationships = [
        ("Discovery", "Specification", "generates"),
        ("Work Management", "Specification", "links to"),
        ("Specification", "Testing & Validation", "tracked by"),
        ("Event Storming", "Foundation", "populates"),
        ("Foundation", "Discovery", "guides"),
    ];

    let find_id = |name: &str| -> Option<String> {
        bounded_contexts
            .iter()
            .position(|c| c.get("text").map(js_str).as_deref() == Some(name))
            .map(node_id)
    };

    for (from, to, label) in relationships {
        if let (Some(from_id), Some(to_id)) = (find_id(from), find_id(to)) {
            lines.push(format!("  {from_id} -->|{label}| {to_id}"));
        }
    }

    lines.join("\n")
}

/// Port of `generateContextEventFlowMermaid`. Returns an empty string when
/// the context has no commands, aggregates, or events (the TS early return).
pub(super) fn context_event_flow_mermaid(context_id: &Value, foundation: &Value) -> String {
    let commands = items_for(foundation, "command", context_id);
    let aggregates = items_for(foundation, "aggregate", context_id);
    let events = items_for(foundation, "event", context_id);

    if commands.is_empty() && aggregates.is_empty() && events.is_empty() {
        return String::new();
    }

    let mut lines: Vec<String> = vec!["flowchart TB".to_string()];

    // Commands subgraph.
    if !commands.is_empty() {
        lines.push("  subgraph Commands[\"⚡ Commands\"]".to_string());
        for cmd in &commands {
            let id = cmd.get("id").map(js_str).unwrap_or_default();
            let text = cmd.get("text").map(js_str).unwrap_or_default();
            lines.push(format!("    C{id}[{text}]"));
        }
        lines.push("  end".to_string());
        lines.push(String::new());
    }

    // Aggregates subgraph.
    if !aggregates.is_empty() {
        lines.push("  subgraph Aggregates[\"📦 Aggregates\"]".to_string());
        for agg in &aggregates {
            let id = agg.get("id").map(js_str).unwrap_or_default();
            let text = agg.get("text").map(js_str).unwrap_or_default();
            lines.push(format!("    A{id}[{text}]"));
        }
        lines.push("  end".to_string());
        lines.push(String::new());
    }

    // Events subgraph.
    if !events.is_empty() {
        lines.push("  subgraph Events[\"📢 Events\"]".to_string());
        for evt in &events {
            let id = evt.get("id").map(js_str).unwrap_or_default();
            let text = evt.get("text").map(js_str).unwrap_or_default();
            lines.push(format!("    E{id}[{text}]"));
        }
        lines.push("  end".to_string());
        lines.push(String::new());
    }

    // Flow arrows.
    if !commands.is_empty() && !aggregates.is_empty() {
        lines.push("  Commands -.-> Aggregates".to_string());
    }
    if !aggregates.is_empty() && !events.is_empty() {
        lines.push("  Aggregates -.-> Events".to_string());
    }

    lines.join("\n")
}

/// Filter `eventStorm.items` for a given `type` belonging to `context_id`.
/// Mirrors the TS predicate:
/// `item.type === <type> && !item.deleted && 'boundedContextId' in item &&
/// item.boundedContextId === contextId`.
pub(super) fn items_for<'a>(
    foundation: &'a Value,
    item_type: &str,
    context_id: &Value,
) -> Vec<&'a Value> {
    let items = foundation
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array);
    let items = match items {
        Some(a) => a,
        None => return Vec::new(),
    };
    items
        .iter()
        .filter(|item| {
            item.get("type").and_then(Value::as_str) == Some(item_type)
                && !truthy(item.get("deleted"))
                && item
                    .as_object()
                    .map(|o| o.contains_key("boundedContextId"))
                    .unwrap_or(false)
                && item
                    .get("boundedContextId")
                    .map(|bc| js_strict_eq(bc, context_id))
                    .unwrap_or(false)
        })
        .collect()
}

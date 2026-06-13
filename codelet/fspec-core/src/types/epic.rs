//! Epic type — Rust port of the TypeScript `Epic` interface at
//! `src/commands/list-epics.ts:7-12`.
//!
//! Each entry in `spec/epics.json` carries `id` plus optional `title`,
//! `description`, and any number of forward-compatible fields. We
//! deserialize them into a typed struct (rather than `serde_json::Value`)
//! so downstream commands can read them safely without hand-rolled key
//! lookups. A `serde(flatten) extra` map preserves any future-added
//! fields the TS implementation may carry without breaking Rust
//! round-tripping.

use serde::{Deserialize, Serialize};

/// A single registered epic (e.g. `authentication`, `dashboard`).
///
/// The on-disk shape is `{ id, title?, description?, ...extra }`.
/// `id` is the slug used by work units to reference the epic (e.g.
/// `workUnit.epic === epic.id`); `title` and `description` are
/// optional human-readable labels rendered by the `list-epics`
/// command.
///
/// `title` and `description` are `Option<String>` with
/// `#[serde(skip_serializing_if = "Option::is_none")]` to mirror
/// TypeScript's `JSON.stringify` semantic of OMITTING `undefined`
/// fields entirely from the serialised output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    /// Slug used by work units to reference this epic via the
    /// `workUnit.epic === epic.id` equality test. MUST be present.
    pub id: String,
    /// Optional human-readable epic name (e.g. `"Authentication"`).
    /// Rendered as the second line of each text entry when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional free-text description rendered after the title.
    /// Omitted from text output entirely when missing (matches TS
    /// `if (epic.description) { ... }` guard at
    /// `src/commands/list-epics.ts:119-121`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compat catch-all. Any future-added fields round-trip
    /// through this map without forcing a struct change.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;
    use serde_json::json;

    #[test]
    fn epic_round_trips_canonical_shape() {
        let v = json!({
            "id": "auth",
            "title": "Authentication",
            "description": "Login features",
            "createdAt": "2026-06-01T00:00:00.000Z"
        });
        let e: Epic = serde_json::from_value(v).unwrap();
        assert_eq!(e.id, "auth");
        assert_eq!(e.title.as_deref(), Some("Authentication"));
        assert_eq!(e.description.as_deref(), Some("Login features"));
        assert_eq!(
            e.extra.get("createdAt").and_then(|v| v.as_str()),
            Some("2026-06-01T00:00:00.000Z")
        );
    }

    #[test]
    fn epic_tolerates_missing_title_and_description() {
        let v = json!({ "id": "x" });
        let e: Epic = serde_json::from_value(v).unwrap();
        assert!(e.title.is_none());
        assert!(e.description.is_none());
    }

    #[test]
    fn epic_omits_none_fields_on_serialize() {
        let e = Epic {
            id: "auth".into(),
            title: Some("Authentication".into()),
            description: None,
            extra: serde_json::Map::new(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"id\":\"auth\""));
        assert!(json.contains("\"title\":\"Authentication\""));
        assert!(
            !json.contains("description"),
            "description=None must be omitted; got: {json}"
        );
    }

    #[test]
    fn epic_preserves_unknown_fields_via_extra() {
        let v = json!({
            "id": "x",
            "title": "X",
            "futureField": 42
        });
        let e: Epic = serde_json::from_value(v).unwrap();
        assert_eq!(
            e.extra
                .get("futureField")
                .and_then(serde_json::Value::as_i64),
            Some(42)
        );
    }
}

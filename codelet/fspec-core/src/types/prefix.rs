//! Prefix type — Rust port of the TypeScript `Prefix` interface at
//! `src/commands/list-prefixes.ts:7-11`.
//!
//! Each entry in `spec/prefixes.json` carries `prefix`, `description`, and
//! `createdAt` fields. We deserialize them into a typed struct (rather than
//! `serde_json::Value`) so downstream commands can read them safely without
//! hand-rolled key lookups. A `serde(flatten) extra` map preserves any
//! forward-compatible fields the TS implementation may add without breaking
//! Rust round-tripping.

use serde::{Deserialize, Serialize};

/// A single registered work-unit ID prefix (e.g. `AUTH`, `DASH`).
///
/// The on-disk shape is `{ prefix, description, createdAt, ...extra }`.
/// `prefix` is the short identifier ("AUTH") used to namespace work-unit
/// IDs ("AUTH-001"); `description` is a free-text label rendered by the
/// `list-prefixes` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefix {
    /// Short identifier (the "AUTH" in "AUTH-001"). MUST be present.
    pub prefix: String,
    /// Free-text label rendered as the second line of each text entry.
    /// MUST be present.
    pub description: String,
    /// ISO-8601 timestamp recorded by the TS `create-prefix` command. Kept
    /// optional so historical or hand-edited files without the field
    /// still parse cleanly.
    #[serde(rename = "createdAt", default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Forward-compat catch-all. Any future-added fields round-trip through
    /// this map without forcing a struct change.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    #[test]
    fn prefix_round_trips_canonical_shape() {
        let v = json!({
            "prefix": "AUTH",
            "description": "Auth features",
            "createdAt": "2026-06-01T00:00:00.000Z"
        });
        let p: Prefix = serde_json::from_value(v).unwrap();
        assert_eq!(p.prefix, "AUTH");
        assert_eq!(p.description, "Auth features");
        assert_eq!(p.created_at.as_deref(), Some("2026-06-01T00:00:00.000Z"));
        assert!(p.extra.is_empty());
    }

    #[test]
    fn prefix_tolerates_missing_created_at() {
        let v = json!({ "prefix": "X", "description": "x" });
        let p: Prefix = serde_json::from_value(v).unwrap();
        assert!(p.created_at.is_none());
    }

    #[test]
    fn prefix_preserves_unknown_fields_via_extra() {
        let v = json!({
            "prefix": "X",
            "description": "x",
            "createdAt": "t",
            "futureField": 42
        });
        let p: Prefix = serde_json::from_value(v).unwrap();
        assert_eq!(p.extra.get("futureField").and_then(serde_json::Value::as_i64), Some(42));
    }
}

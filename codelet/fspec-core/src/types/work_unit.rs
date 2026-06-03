//! WorkUnit + WorkUnitsData type definitions — Rust port of
//! `src/types/index.ts:131-195`.
//!
//! All non-essential fields are preserved via `serde_json::Value` so that
//! round-tripping `spec/work-units.json` through Rust does not lose data.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Current on-disk schema version. Matches the TS `CURRENT_VERSION` constant
/// in `src/migrations/registry.ts`.
pub const CURRENT_VERSION: &str = "0.7.1";

/// Top-level shape of `spec/work-units.json`.
///
/// Insertion order of `work_units` is preserved via [`IndexMap`] so that
/// `Object.values(...)` parity holds with the TypeScript implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkUnitsData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(rename = "workUnits", default)]
    pub work_units: IndexMap<String, WorkUnit>,
    #[serde(default = "default_states")]
    pub states: WorkUnitStates,
    /// Everything else (`migrationHistory`, `prefixCounters`, future fields)
    /// is preserved transparently.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl WorkUnitsData {
    /// Returns the default value used for first-time creation of
    /// `spec/work-units.json`. Mirrors `ensureWorkUnitsFile`'s `initialData`
    /// at `src/utils/ensure-files.ts:21-37`.
    pub fn initial(now_iso: impl Into<String>) -> Self {
        Self {
            version: Some(CURRENT_VERSION.to_string()),
            meta: Some(Meta {
                version: "1.0.0".to_string(),
                last_updated: now_iso.into(),
            }),
            work_units: IndexMap::new(),
            states: default_states(),
            extra: serde_json::Map::new(),
        }
    }
}

/// `meta` block of `work-units.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub version: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

/// The 7 Kanban state arrays. Each entry is a work-unit ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkUnitStates {
    #[serde(default)]
    pub backlog: Vec<String>,
    #[serde(default)]
    pub specifying: Vec<String>,
    #[serde(default)]
    pub testing: Vec<String>,
    #[serde(default)]
    pub implementing: Vec<String>,
    #[serde(default)]
    pub validating: Vec<String>,
    #[serde(default)]
    pub done: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
}

fn default_states() -> WorkUnitStates {
    WorkUnitStates {
        backlog: vec![],
        specifying: vec![],
        testing: vec![],
        implementing: vec![],
        validating: vec![],
        done: vec![],
        blocked: vec![],
    }
}

/// Work-unit type tag accepted at the CLI surface — `story`, `task`, or `bug`.
///
/// This enum is used ONLY where Rust must validate type values *coming in*
/// from a user (e.g. clap value-parsing for `--type` on the
/// `list-work-units` subcommand and the same field on the LLM-facing
/// dispatcher argument shape). It is NOT used to model the type field
/// stored on a [`WorkUnit`] on disk — that field is preserved as a raw
/// `Option<String>` (see [`WorkUnit.r#type`]) so any historical or
/// not-yet-modelled type value (e.g. `feature`) round-trips losslessly,
/// matching TS-runtime behaviour where `WorkUnitType` is a compile-time-
/// only union and `JSON.parse` accepts arbitrary strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkUnitType {
    Story,
    Task,
    Bug,
}

impl WorkUnitType {
    /// Canonical default applied to work units whose `type` field is
    /// missing OR holds a value outside the CLI-accepted set. Matches the
    /// TS expression `wu.type || 'story'` and the filter semantics in
    /// `src/commands/list-work-units.ts:56-61`.
    pub const DEFAULT: Self = WorkUnitType::Story;

    /// Lowercase string form used in CLI args and JSON serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkUnitType::Story => "story",
            WorkUnitType::Task => "task",
            WorkUnitType::Bug => "bug",
        }
    }
}

/// Lifecycle state of a single work unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkUnitStatus {
    Backlog,
    Specifying,
    Testing,
    Implementing,
    Validating,
    Done,
    Blocked,
}

impl WorkUnitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkUnitStatus::Backlog => "backlog",
            WorkUnitStatus::Specifying => "specifying",
            WorkUnitStatus::Testing => "testing",
            WorkUnitStatus::Implementing => "implementing",
            WorkUnitStatus::Validating => "validating",
            WorkUnitStatus::Done => "done",
            WorkUnitStatus::Blocked => "blocked",
        }
    }
}

/// A single work unit. Only the fields read by ported commands are typed;
/// everything else round-trips through `extra`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkUnit {
    pub id: String,
    pub title: String,
    /// Raw work-unit type as stored on disk. Kept as `Option<String>` (not a
    /// strict enum) so that values outside the CLI-accepted set
    /// (`story` / `task` / `bug`) — e.g. legacy `feature` entries that
    /// predate the type-narrowing — round-trip losslessly. This mirrors the
    /// TypeScript runtime, where `WorkUnitType` is a compile-time union and
    /// `JSON.parse` happily accepts any string. Use [`Self::type_str`] for
    /// the TS-equivalent `wu.type || 'story'` default.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<String>,
    pub status: WorkUnitStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// All fields the Rust port doesn't yet model — preserved verbatim so
    /// write-back doesn't lose data.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl WorkUnit {
    /// Returns the work-unit type as a string slice with the legacy
    /// "missing → story" default applied. Mirrors the TS expression
    /// `wu.type || 'story'` at `src/commands/list-work-units.ts:58` and
    /// preserves unknown variants verbatim (so `--type=story` does NOT
    /// match a `type: "feature"` unit, matching TS string-equality
    /// semantics).
    pub fn type_str(&self) -> &str {
        self.r#type
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("story")
    }

    /// Returns the work-unit type narrowed to the CLI-accepted enum when
    /// the on-disk value is one of `story` / `task` / `bug`; otherwise
    /// returns `None`. Useful when callers need a typed value but should
    /// gracefully ignore unknown variants (e.g. status-rendering code that
    /// only knows how to label the three canonical types).
    pub fn typed(&self) -> Option<WorkUnitType> {
        match self.type_str() {
            "story" => Some(WorkUnitType::Story),
            "task" => Some(WorkUnitType::Task),
            "bug" => Some(WorkUnitType::Bug),
            _ => None,
        }
    }

    /// Legacy alias for [`Self::typed`] returning the CLI-accepted enum
    /// with the "missing → story" default applied. Unknown on-disk values
    /// collapse to [`WorkUnitType::DEFAULT`] so this remains a total
    /// function — but callers comparing against a user-provided filter
    /// SHOULD prefer [`Self::type_str`] to preserve TS string-equality
    /// semantics (an unknown variant must not silently match `--type=story`).
    pub fn effective_type(&self) -> WorkUnitType {
        self.typed().unwrap_or(WorkUnitType::DEFAULT)
    }
}

/// Shape of `spec/prefixes.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixesData {
    #[serde(default)]
    pub prefixes: serde_json::Map<String, serde_json::Value>,
}

impl PrefixesData {
    /// Default value used by `ensurePrefixesFile` (TS `src/utils/ensure-files.ts:64-73`).
    pub fn initial() -> Self {
        Self {
            prefixes: serde_json::Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn work_unit_type_defaults_to_story_when_missing() {
        let wu: WorkUnit = serde_json::from_value(json!({
            "id": "AUTH-001",
            "title": "Login",
            "status": "backlog",
            "createdAt": "2026-06-01T00:00:00.000Z",
            "updatedAt": "2026-06-01T00:00:00.000Z"
        }))
        .unwrap();
        assert_eq!(wu.r#type, None);
        assert_eq!(wu.type_str(), "story");
        assert_eq!(wu.effective_type(), WorkUnitType::Story);
        assert_eq!(wu.typed(), Some(WorkUnitType::Story));
    }

    #[test]
    fn work_unit_tolerates_unknown_type_variant_for_ts_runtime_parity() {
        // RPC-253 follow-up: spec/work-units.json in the live repo contains
        // type="feature" entries that the original strict-enum deserializer
        // rejected with `unknown variant`. The TS source-of-truth at
        // src/types/index.ts:132 declares `WorkUnitType = 'story' | 'task' |
        // 'bug'`, but TS-runtime never validates this — `JSON.parse` accepts
        // any string. Rust MUST mirror that tolerance.
        let wu: WorkUnit = serde_json::from_value(json!({
            "id": "FEAT-001",
            "title": "Pre-existing feature-typed unit",
            "type": "feature",
            "status": "backlog",
            "createdAt": "2026-06-01T00:00:00.000Z",
            "updatedAt": "2026-06-01T00:00:00.000Z"
        }))
        .expect("WorkUnit deserialization must tolerate unknown type variants");
        assert_eq!(wu.r#type.as_deref(), Some("feature"));
        // type_str preserves the raw value so string-equality filters
        // (rule [16] on RPC-253) reject it from --type=story.
        assert_eq!(wu.type_str(), "feature");
        // typed() returns None for non-canonical variants.
        assert_eq!(wu.typed(), None);
        // effective_type() collapses to the DEFAULT for legacy callers that
        // need a total function.
        assert_eq!(wu.effective_type(), WorkUnitType::Story);
    }

    #[test]
    fn work_unit_empty_string_type_collapses_to_story_for_ts_or_short_circuit_parity() {
        // TS expression `wu.type || 'story'` treats `""` as falsy and falls
        // through to the default. type_str() MUST do the same.
        let wu: WorkUnit = serde_json::from_value(json!({
            "id": "WU-001",
            "title": "Empty type field",
            "type": "",
            "status": "backlog",
            "createdAt": "x",
            "updatedAt": "x"
        }))
        .unwrap();
        assert_eq!(wu.r#type.as_deref(), Some(""));
        assert_eq!(wu.type_str(), "story");
    }

    #[test]
    fn work_units_data_initial_has_all_seven_states_empty() {
        let d = WorkUnitsData::initial("2026-06-01T00:00:00.000Z");
        assert_eq!(d.version.as_deref(), Some("0.7.1"));
        assert!(d.work_units.is_empty());
        assert!(d.states.backlog.is_empty());
        assert!(d.states.specifying.is_empty());
        assert!(d.states.testing.is_empty());
        assert!(d.states.implementing.is_empty());
        assert!(d.states.validating.is_empty());
        assert!(d.states.done.is_empty());
        assert!(d.states.blocked.is_empty());
    }

    #[test]
    fn work_units_data_preserves_insertion_order() {
        // Parse from a raw JSON string (not `json!{}`) so the upstream object
        // key order is preserved on the way into our IndexMap. Going through
        // `serde_json::from_value` would route through `serde_json::Map`,
        // which is alphabetical by default.
        let raw = r#"{
            "workUnits": {
                "C-1": { "id": "C-1", "title": "third", "status": "backlog",
                         "createdAt": "x", "updatedAt": "x" },
                "A-1": { "id": "A-1", "title": "first", "status": "backlog",
                         "createdAt": "x", "updatedAt": "x" },
                "B-1": { "id": "B-1", "title": "second", "status": "backlog",
                         "createdAt": "x", "updatedAt": "x" }
            },
            "states": {
                "backlog": ["C-1", "A-1", "B-1"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }"#;
        let d: WorkUnitsData = serde_json::from_str(raw).unwrap();
        let order: Vec<&str> = d.work_units.keys().map(String::as_str).collect();
        assert_eq!(order, vec!["C-1", "A-1", "B-1"]);
    }
}

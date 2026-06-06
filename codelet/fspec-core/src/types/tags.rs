//! `tags.json` shape — Rust port of the TypeScript `Tags` interface
//! at `src/types/tags.ts` (consumed by every tag-aware fspec command).
//!
//! Only the subset of fields exercised by `list-tags` and other
//! near-term ports is modelled explicitly; everything else round-trips
//! via the `extra` `#[serde(flatten)]` catch-alls so that future-added
//! fields (and the rich `usageGuidelines` / `combinationExamples` /
//! etc. tree from `ensureTagsFile`) survive a load → save cycle losslessly.
//!
//! Insertion order on disk MUST be preserved (the TS implementation
//! iterates `tagsData.categories` directly, never re-sorting), so the
//! top-level `categories` field is a `Vec<TagCategory>` rather than a
//! map. Tags within a category are similarly a `Vec<Tag>` to mirror
//! the TS array semantics — the `list-tags` command applies its own
//! alphabetical sort on the projection step, never mutating the input.

use serde::{Deserialize, Serialize};

/// Top-level shape of `spec/tags.json`.
///
/// Mirrors the TS `Tags` interface (`src/types/tags.ts`). Auxiliary
/// fields (`combinationExamples`, `usageGuidelines`, `addingNewTags`,
/// `queries`, `statistics`, `validation`, `references`, …) round-trip
/// through `extra` without forcing a struct change every time the TS
/// surface grows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    /// The ordered list of tag categories. Insertion order on disk
    /// is the order rendered by `list-tags` — NOT alphabetical.
    #[serde(default)]
    pub categories: Vec<TagCategory>,
    /// Forward-compat catch-all preserving every other top-level
    /// field (`usageGuidelines`, `statistics`, …) verbatim.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A single tag category entry inside `tagsData.categories`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCategory {
    /// Human-readable category name (e.g. `"Phase Tags"`). Used as
    /// both the display header AND the `--category` filter key.
    pub name: String,
    /// Free-text description rendered by other tag commands. Defaults
    /// to empty string when missing from older files.
    #[serde(default)]
    pub description: String,
    /// Whether the category is required by `validate-tags`. Defaults
    /// to `false` for tolerant deserialisation of older files.
    #[serde(default)]
    pub required: bool,
    /// Tags within this category. Insertion order preserved on disk;
    /// `list-tags` applies its own alphabetical sort on projection.
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Optional `rule` field present on some categories
    /// (`Phase Tags` requires "every feature MUST have at least one").
    /// Stored as `Option<String>` so missing files don't synthesise
    /// `""` round-trips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// Forward-compat catch-all preserving every other category-level
    /// field verbatim.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A single tag entry inside a `TagCategory.tags` array.
///
/// The TS `Tag` interface carries additional fields (`usage`,
/// `scope`, `examples`, …) that other tag commands consume; the
/// `list-tags` command intentionally projects ONLY `name` +
/// `description`, so we keep those fields in the `extra` catch-all
/// to preserve them across load-modify-save cycles while still
/// ignoring them at the `list-tags` projection step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name including the leading `@` (e.g. `"@critical"`).
    pub name: String,
    /// Free-text description shown next to the tag on the `list-tags`
    /// output line.
    pub description: String,
    /// Forward-compat catch-all preserving `usage`, `scope`,
    /// `examples`, and any future tag-level fields verbatim.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl TagsData {
    /// Canonical 9-category default used when `spec/tags.json` is
    /// missing. Mirrors `ensureTagsFile`'s `initialData` at
    /// `src/utils/ensure-files.ts:98-191`. The insertion order of the
    /// returned `Vec` matches the TS array literal exactly so the
    /// `list-tags` first-render walk produces identical output.
    pub fn initial() -> Self {
        let categories = vec![
            TagCategory {
                name: "Phase Tags".to_string(),
                description: "Phase identification tags".to_string(),
                required: true,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Component Tags".to_string(),
                description: "Architectural component tags".to_string(),
                required: true,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Feature Group Tags".to_string(),
                description: "Functional area tags".to_string(),
                required: true,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Technical Tags".to_string(),
                description: "Technical concern tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Platform Tags".to_string(),
                description: "Platform-specific tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Priority Tags".to_string(),
                description: "Implementation priority tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Status Tags".to_string(),
                description: "Development status tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Testing Tags".to_string(),
                description: "Test-related tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
            TagCategory {
                name: "Automation Tags".to_string(),
                description: "Automation integration tags".to_string(),
                required: false,
                tags: vec![],
                rule: None,
                extra: serde_json::Map::new(),
            },
        ];
        Self {
            categories,
            extra: serde_json::Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    #[test]
    fn initial_yields_nine_categories_in_canonical_order() {
        let d = TagsData::initial();
        assert_eq!(d.categories.len(), 9);
        let expected = [
            "Phase Tags",
            "Component Tags",
            "Feature Group Tags",
            "Technical Tags",
            "Platform Tags",
            "Priority Tags",
            "Status Tags",
            "Testing Tags",
            "Automation Tags",
        ];
        for (i, name) in expected.iter().enumerate() {
            assert_eq!(d.categories[i].name, *name, "mismatch @ index {i}");
        }
    }

    #[test]
    fn initial_phase_component_feature_group_are_required() {
        let d = TagsData::initial();
        assert!(d.categories[0].required, "Phase Tags must be required");
        assert!(d.categories[1].required, "Component Tags must be required");
        assert!(d.categories[2].required, "Feature Group Tags must be required");
        assert!(!d.categories[3].required, "Technical Tags must NOT be required");
    }

    #[test]
    fn tag_preserves_unknown_fields_via_extra() {
        let v = json!({
            "name": "@critical",
            "description": "x",
            "usage": "rare",
            "scope": "wide",
            "examples": "auth"
        });
        let t: Tag = serde_json::from_value(v).unwrap();
        assert_eq!(t.name, "@critical");
        assert_eq!(t.description, "x");
        assert_eq!(t.extra.get("usage").and_then(|v| v.as_str()), Some("rare"));
        assert_eq!(t.extra.get("scope").and_then(|v| v.as_str()), Some("wide"));
    }

    #[test]
    fn category_round_trips_canonical_shape() {
        let v = json!({
            "name": "Phase Tags",
            "description": "desc",
            "required": true,
            "tags": [
                { "name": "@critical", "description": "c" }
            ]
        });
        let c: TagCategory = serde_json::from_value(v).unwrap();
        assert_eq!(c.name, "Phase Tags");
        assert!(c.required);
        assert_eq!(c.tags.len(), 1);
        assert_eq!(c.tags[0].name, "@critical");
    }

    #[test]
    fn tags_data_default_categories_is_empty_when_missing() {
        let v = json!({ "version": "x" });
        let d: TagsData = serde_json::from_value(v).unwrap();
        assert!(d.categories.is_empty());
        assert!(d.extra.contains_key("version"));
    }
}

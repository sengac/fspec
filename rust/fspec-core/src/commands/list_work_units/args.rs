//! The JSON arguments of `list-work-units` (CLI-017). The fspec CLI and the
//! LLM dispatcher send the same shape: camelCase keys, list filters as an
//! array of strings or one comma-separated string.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use serde::{Deserialize, Deserializer};

/// A list filter: `["a", "b"]` or `"a,b"` (OR within the filter).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct List(pub(super) Vec<String>);

impl List {
    pub(super) fn values(&self) -> &[String] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            One(String),
            Many(Vec<String>),
        }
        let items = match Raw::deserialize(d)? {
            Raw::One(s) => vec![s],
            Raw::Many(v) => v,
        };
        Ok(List(
            items
                .iter()
                .flat_map(|s| s.split(','))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        ))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Args {
    pub(super) status: Option<List>,
    pub(super) prefix: Option<List>,
    pub(super) epic: Option<List>,
    #[serde(rename = "type")]
    pub(super) r#type: Option<List>,
    pub(super) work_unit_id: Option<List>,
    pub(super) parent: Option<String>,
    pub(super) has_children: bool,
    pub(super) estimate_min: Option<i64>,
    pub(super) estimate_max: Option<i64>,
    pub(super) has_estimate: bool,
    pub(super) no_estimate: bool,

    pub(super) created_since: Option<String>,
    pub(super) created_until: Option<String>,
    pub(super) updated_since: Option<String>,
    pub(super) updated_until: Option<String>,
    pub(super) done_since: Option<String>,
    pub(super) done_until: Option<String>,
    pub(super) entered: Option<String>,
    pub(super) entered_since: Option<String>,
    pub(super) entered_until: Option<String>,

    pub(super) text: Option<String>,
    pub(super) title: Option<String>,
    pub(super) has_open_questions: bool,
    pub(super) has_assumptions: bool,
    pub(super) has_rules: bool,
    pub(super) no_rules: bool,
    pub(super) has_examples: bool,
    pub(super) has_attachments: bool,
    pub(super) has_architecture_notes: bool,
    pub(super) tag: Option<List>,

    pub(super) depends_on: Option<List>,
    pub(super) blocked_by: Option<List>,
    pub(super) blocks: Option<List>,
    pub(super) relates_to: Option<List>,
    pub(super) blocked: bool,

    pub(super) fields: Option<List>,
    pub(super) format: Option<String>,
    pub(super) group_by: Option<String>,
    pub(super) sort: Option<String>,
    pub(super) desc: bool,
    pub(super) limit: Option<usize>,
}

//! CLI-019: the blocks and pages of `fspec context` in markdown, text and
//! json.
//!
//! A page body is its blocks in order: a feature's header block before the
//! feature's first criterion on the page, then each criterion's block. The
//! page budget counts the characters of those blocks (for json, of each
//! feature and item object), not the footer.

use serde_json::{json, Value};

use super::select::{Criterion, Feature, Selection};
use super::Format;
use crate::error::FspecCoreError;

/// Every feature's header block and every criterion's block, rendered once.
pub(super) struct Blocks {
    format: Format,
    headers: Vec<String>,
    items: Vec<String>,
}

impl Blocks {
    pub(super) fn new(selection: &Selection, format: Format) -> Result<Self, FspecCoreError> {
        let headers = selection
            .features
            .iter()
            .map(|feature| header_block(feature, format))
            .collect::<Result<_, _>>()?;
        let items = selection
            .criteria
            .iter()
            .map(|criterion| item_block(&selection.features[criterion.feature], criterion, format))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            format,
            headers,
            items,
        })
    }

    pub(super) fn header_len(&self, feature: usize) -> usize {
        self.headers[feature].chars().count()
    }

    pub(super) fn item_len(&self, criterion: usize) -> usize {
        self.items[criterion].chars().count()
    }
}

fn tag_list(tags: &[String]) -> String {
    if tags.is_empty() {
        "(none)".to_string()
    } else {
        tags.join(" ")
    }
}

fn serialize(value: &Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(value)
        .map_err(|e| FspecCoreError::Message(format!("context: failed to serialize: {e}")))
}

fn feature_json(feature: &Feature) -> Value {
    json!({
        "file": feature.file,
        "name": feature.name,
        "tags": feature.tags,
        "gherkin": feature.header.join("\n"),
    })
}

fn item_json(feature: &Feature, criterion: &Criterion) -> Value {
    json!({
        "file": feature.file,
        "line": criterion.line,
        "feature": feature.name,
        "rule": criterion.rule,
        "name": criterion.name,
        "tags": criterion.tags,
        "inheritedTags": criterion.inherited,
        "gherkin": criterion.lines.join("\n"),
    })
}

fn header_block(feature: &Feature, format: Format) -> Result<String, FspecCoreError> {
    let gherkin = feature.header.join("\n");
    Ok(match format {
        Format::Markdown => format!("## {}\n\n```gherkin\n{gherkin}\n```\n\n", feature.file),
        Format::Text => format!("Feature file: {}\n{gherkin}\n\n", feature.file),
        Format::Json => serialize(&feature_json(feature))?,
    })
}

fn item_block(
    feature: &Feature,
    criterion: &Criterion,
    format: Format,
) -> Result<String, FspecCoreError> {
    let location = format!(
        "Location: {}:{}\nInherited tags: {}",
        feature.file,
        criterion.line,
        tag_list(&criterion.inherited)
    );
    let gherkin = criterion.lines.join("\n");
    Ok(match format {
        Format::Markdown => format!(
            "### {}\n\n{location}\n\n```gherkin\n{gherkin}\n```\n\n",
            criterion.name
        ),
        Format::Text => format!("{location}\n{gherkin}\n\n"),
        Format::Json => serialize(&item_json(feature, criterion))?,
    })
}

/// Render page `number` of `count` holding the criteria `items`.
pub(super) fn page(
    selection: &Selection,
    blocks: &Blocks,
    items: &[usize],
    number: usize,
    count: usize,
    next: Option<&str>,
) -> Result<String, FspecCoreError> {
    if blocks.format == Format::Json {
        let mut features: Vec<usize> = Vec::new();
        for &i in items {
            let feature = selection.criteria[i].feature;
            if !features.contains(&feature) {
                features.push(feature);
            }
        }
        let page = json!({
            "page": number,
            "pages": count,
            "nextCommand": next,
            "features": features
                .iter()
                .map(|&f| feature_json(&selection.features[f]))
                .collect::<Vec<_>>(),
            "items": items
                .iter()
                .map(|&i| {
                    let criterion = &selection.criteria[i];
                    item_json(&selection.features[criterion.feature], criterion)
                })
                .collect::<Vec<_>>(),
        });
        return Ok(format!("{}\n", serialize(&page)?));
    }
    let mut out = String::new();
    let mut shown: Vec<usize> = Vec::new();
    for &i in items {
        let feature = selection.criteria[i].feature;
        if !shown.contains(&feature) {
            shown.push(feature);
            out.push_str(&blocks.headers[feature]);
        }
        out.push_str(&blocks.items[i]);
    }
    if items.is_empty() {
        out.push_str("No scenarios match.\n\n");
    }
    if blocks.format == Format::Markdown {
        out.push_str("---\n\n");
    }
    out.push_str(&format!("page {number} of {count}\n"));
    if let Some(next) = next {
        out.push_str(&format!("Next page: {next}\n"));
    }
    Ok(out)
}

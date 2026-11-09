//! BUG-439: how thinking Off is expressed per Claude model.
//!
//! Feature: spec/features/claude-off-on-always-on-thinking-models.feature

use serde_json::{json, Value};

/// `(family, major, minor)` of a `claude-{family}-{major}[-{minor}][-{date}]`
/// id; `None` for other ids (including the `claude-3-*` naming).
fn claude_version(model: &str) -> Option<(&str, u32, Option<u32>)> {
    let mut parts = model.strip_prefix("claude-")?.split('-');
    let family = parts
        .next()
        .filter(|family| family.chars().all(|c| c.is_ascii_alphabetic()))?;
    let major = parts.next()?.parse().ok()?;
    let minor = parts
        .next()
        .filter(|part| part.len() <= 2)
        .and_then(|part| part.parse().ok());
    Some((family, major, minor))
}

/// BUG-439: how Off is expressed for a Claude model. Claude 4.x thinks only
/// when asked, so Off sends nothing; Claude 5+ thinks by default and each
/// model has its own Off form, or none (Anthropic thinking-troubleshooting
/// table, MODEL-009 research §4).
pub fn claude_off_config(model: &str) -> Result<Option<Value>, String> {
    let Some((family, major, minor)) = claude_version(model) else {
        return Ok(None);
    };
    if major < 5 {
        return Ok(None);
    }
    match (family, major, minor) {
        ("sonnet", 5, Some(5)) => Ok(Some(json!({"thinking": {"type": "between_tools"}}))),
        ("opus", 5, None) | ("haiku", 5, Some(5)) => {
            Ok(Some(json!({"thinking": {"type": "disabled"}})))
        }
        ("opus", 5, Some(5)) | ("fable", _, _) | ("mythos", _, _) => Err(format!(
            "thinking cannot be turned off on {model}; choose Low, Medium or High"
        )),
        _ => Err(format!(
            "thinking cannot be turned off on {model}: no Off form is known for this model; \
             choose Low, Medium or High"
        )),
    }
}

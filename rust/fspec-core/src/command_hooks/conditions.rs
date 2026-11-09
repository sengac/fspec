//! Hook condition evaluation (HOOK-005).

use std::path::Path;

use serde_json::Value;

use super::config::CommandHook;
use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitsData;

/// The work-unit facts a hook condition reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HookWorkUnit {
    pub id: String,
    pub tags: Vec<String>,
    pub epic: Option<String>,
    pub estimate: Option<f64>,
}

impl HookWorkUnit {
    /// Read work unit `id` from `spec/work-units.json`. `None` when the board
    /// or the work unit does not exist; an unreadable board or a malformed
    /// `tags`/`estimate` is an error.
    pub fn load(project_root: &Path, id: &str) -> Result<Option<Self>, FspecCoreError> {
        let path = crate::io::spec_paths::work_units(project_root)?;
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(board_error(&e.to_string())),
        };
        let data: WorkUnitsData =
            serde_json::from_str(&raw).map_err(|e| board_error(&e.to_string()))?;
        let Some(wu) = data.work_units.get(id) else {
            return Ok(None);
        };
        let tags = match wu.extra.get("tags") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .and_then(|tags| {
                    tags.iter()
                        .map(|t| t.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| board_error(&format!("{id}.tags must be an array of strings")))?,
        };
        let estimate = match wu.extra.get("estimate") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_f64()
                    .ok_or_else(|| board_error(&format!("{id}.estimate must be a number")))?,
            ),
        };
        Ok(Some(Self {
            id: wu.id.clone(),
            tags,
            epic: wu.epic.clone(),
            estimate,
        }))
    }
}

fn board_error(detail: &str) -> FspecCoreError {
    FspecCoreError::Message(format!(
        "Cannot evaluate hook conditions: spec/work-units.json: {detail}"
    ))
}

/// Whether `hook` runs for `work_unit` (`None`: the command has no work unit
/// id, or its work unit does not exist).
///
/// A hook without a condition always matches. Condition fields are ANDed;
/// `tags` and `prefix` match when ANY listed value matches; `epic` must equal
/// the work unit's epic; `estimateMin`/`estimateMax` bound its estimate and
/// an unestimated work unit does not match. A conditional hook never matches
/// without a work unit.
pub fn hook_matches(hook: &CommandHook, work_unit: Option<&HookWorkUnit>) -> bool {
    let Some(condition) = &hook.condition else {
        return true;
    };
    let Some(wu) = work_unit else {
        return false;
    };
    if !condition.tags.is_empty() && !condition.tags.iter().any(|t| wu.tags.contains(t)) {
        return false;
    }
    if !condition.prefix.is_empty()
        && !condition
            .prefix
            .iter()
            .any(|p| wu.id.starts_with(p.as_str()))
    {
        return false;
    }
    if let Some(epic) = &condition.epic {
        if wu.epic.as_ref() != Some(epic) {
            return false;
        }
    }
    if condition.estimate_min.is_some() || condition.estimate_max.is_some() {
        let Some(estimate) = wu.estimate else {
            return false;
        };
        if condition.estimate_min.is_some_and(|min| estimate < min)
            || condition.estimate_max.is_some_and(|max| estimate > max)
        {
            return false;
        }
    }
    true
}

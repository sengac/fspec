//! RPC-364 — auto/manual checkpoint label formatting for the Checkpoints
//! list pane.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! Automatic checkpoints carry a `-auto-<state>` suffix in their `name`
//! (e.g. `AUTH-001-auto-testing`) and render as `"{work_unit_id}: {Phase}"`
//! with the phase capitalized. Manual checkpoints render their raw `name`.

use codelet_rpc_types::CheckpointInfo;

/// Build the human-facing label for one checkpoint row.
///
/// Automatic checkpoints become `"{work_unit_id}: {Phase}"` where `Phase`
/// is the `-auto-<state>` suffix capitalized; manual checkpoints return
/// their raw `name` unchanged.
pub fn checkpoint_label(cp: &CheckpointInfo) -> String {
    if cp.is_automatic {
        if let Some(phase) = auto_phase(&cp.name) {
            return format!("{}: {}", cp.work_unit_id, phase);
        }
    }
    cp.name.clone()
}

/// Extract + capitalize the `<state>` from a `...-auto-<state>` name.
/// Returns `None` when the `-auto-` marker is absent or the suffix empty.
fn auto_phase(name: &str) -> Option<String> {
    let idx = name.find("-auto-")?;
    let state = &name[idx + "-auto-".len()..];
    if state.is_empty() {
        return None;
    }
    Some(capitalize(state))
}

/// Capitalize the first character, lowercasing the rest.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => {
            let rest: String = chars.as_str().to_lowercase();
            format!("{}{}", first.to_uppercase(), rest)
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn cp(work_unit_id: &str, name: &str, is_automatic: bool) -> CheckpointInfo {
        CheckpointInfo {
            work_unit_id: work_unit_id.to_string(),
            name: name.to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            is_automatic,
        }
    }

    #[test]
    fn auto_checkpoint_renders_id_and_capitalized_phase() {
        let c = cp("AUTH-001", "AUTH-001-auto-testing", true);
        assert_eq!(checkpoint_label(&c), "AUTH-001: Testing");
    }

    #[test]
    fn manual_checkpoint_renders_raw_name() {
        let c = cp("AUTH-001", "baseline", false);
        assert_eq!(checkpoint_label(&c), "baseline");
    }
}

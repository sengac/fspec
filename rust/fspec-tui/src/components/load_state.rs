//! TUI-106 — staged cascade load tracker shared by the lazy mode-views.
//!
//! Feature: shared-animated-loadingdialog-base… (TUI-106)
//!
//! One per lazy mode-view cascade (Checkpoints: list → files → diff;
//! Changed Files: list → diff). Each *stage* is an independent in-flight
//! RPC spawned by the App dispatchers (the action bus remains the single
//! coordination channel); the tracker records WHICH stage is in flight
//! plus its human-readable label so the loading dialog can name what is
//! being loaded (the "progress" for many-checkpoint repos — TUI-109
//! later feeds the per-item `(idx/total)` counter).
//!
//! Stale-drop invariance: `complete_stage(key)` is a no-op (returns
//! `false`) when `key` does not match the currently-in-flight stage, so
//! a late result for a de-selected item can never clear the current
//! stage's loading. This mirrors the two views' existing matching-key
//! stale-drop in `set_files` / `set_diff` (RPC-364 / RPC-356), which stay
//! untouched.

/// One in-flight cascade stage (an identity key for stale-drop + the
/// spinner line label displayed by the loading dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stage {
    key: String,
    label: String,
}

/// Staged in-flight marker for a lazy mode-view cascade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadTracker {
    /// Label of the initial list/scan stage ("Loading checkpoint list…"
    /// / "Loading changed files…"). Owned by the view's cascade.
    list_label: String,
    /// Set to true once the list stage flushed (the load may be empty —
    /// a failed load degrades to the real empty state, current behavior).
    list_loaded: bool,
    /// The in-flight cascade stage after the list, if any.
    stage: Option<Stage>,
}

impl LoadTracker {
    /// Construct a tracker with the list/scan stage in flight.
    pub fn new(list_label: impl Into<String>) -> Self {
        Self {
            list_label: list_label.into(),
            list_loaded: false,
            stage: None,
        }
    }

    /// True while a lazy load is in flight (list not yet flushed OR a
    /// cascade stage is current). Drives `view::is_loading()` and the
    /// run-loop redraw gate (TUI-106).
    pub fn is_loading(&self) -> bool {
        !self.list_loaded || self.stage.is_some()
    }

    /// The list/scan stage has flushed (possibly with an empty result).
    /// Returns true when a load was in flight, i.e. the view may dismiss
    /// its loading dialog now.
    pub fn mark_list_flushed(&mut self) -> bool {
        let was_loading = self.is_loading();
        self.list_loaded = true;
        was_loading
    }

    /// True once the list/scan stage has flushed. The state
    /// discriminator behind "loading ≠ empty": a view that is loaded
    /// AND empty paints the real empty message, never the loading dialog.
    pub fn is_loaded(&self) -> bool {
        self.list_loaded
    }

    /// A cascade stage load was requested. `key` is the stale-drop
    /// identity (see `files_stage_key` / `diff_stage_key`); `label` is
    /// the view-owned spinner-line text (views sanitize before passing).
    pub fn begin_stage(&mut self, key: &str, label: impl Into<String>) {
        self.stage = Some(Stage {
            key: key.to_string(),
            label: label.into(),
        });
    }

    /// A cascade stage result matched `key`. No-op (returns `false`)
    /// when `key` does not match the current stage — a stale result for
    /// a de-selected item must NOT clear the current stage's loading.
    pub fn complete_stage(&mut self, key: &str) -> bool {
        match self.stage.as_ref() {
            Some(stage) if stage.key == key => {
                self.stage = None;
                true
            }
            _ => false,
        }
    }

    /// The spinner-line label for the active stage, or the list label
    /// while the list is loading. `None` when the cascade is settled.
    pub fn active_label(&self) -> Option<String> {
        match self.stage.as_ref() {
            Some(stage) => Some(stage.label.clone()),
            None if !self.list_loaded => Some(self.list_label.clone()),
            None => None,
        }
    }

    /// Identity key of the in-flight cascade stage (for stale-drop).
    pub fn active_stage_key(&self) -> Option<&str> {
        self.stage.as_ref().map(|stage| stage.key.as_str())
    }

    /// Stage key for the Checkpoints "files" stage — mirrors the
    /// `(work_unit_id, name)` identity the view already uses for
    /// stale-drop (`views/checkpoints/mod.rs` `files_key`).
    pub fn files_stage_key(work_unit_id: &str, name: &str) -> String {
        format!("files:{work_unit_id}:{name}")
    }

    /// Stage key for the Checkpoints "diff" stage — mirrors
    /// `(work_unit_id, name, path)` (`diff_key`).
    pub fn diff_stage_key(work_unit_id: &str, name: &str, path: &str) -> String {
        format!("diff:{work_unit_id}:{name}:{path}")
    }

    /// Stage key for the Changed Files "diff" stage — mirrors
    /// `diff_path: Option<String>`.
    pub fn diff_stage_key_path(path: &str) -> String {
        format!("diff:{path}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn fresh_tracker_is_loading_with_list_label() {
        let t = LoadTracker::new("Loading checkpoint list…");
        assert!(t.is_loading());
        assert!(!t.is_loaded());
        assert_eq!(
            t.active_label().as_deref(),
            Some("Loading checkpoint list…")
        );
    }

    #[test]
    fn complete_stage_is_noop_for_stale_key() {
        let mut t = LoadTracker::new("Loading checkpoint list…");
        t.mark_list_flushed();
        t.begin_stage(&LoadTracker::files_stage_key("A", "B"), "files…");
        assert!(!t.complete_stage(&LoadTracker::files_stage_key("A", "OLD")));
        assert!(t.is_loading(), "stale result must not clear the stage");
        assert!(t.complete_stage(&LoadTracker::files_stage_key("A", "B")));
        assert!(!t.is_loading());
        assert_eq!(t.active_label(), None);
    }

    #[test]
    fn stage_keys_follow_stale_drop_shape() {
        assert_eq!(
            LoadTracker::files_stage_key("AUTH-001", "pre-refactor"),
            "files:AUTH-001:pre-refactor"
        );
        assert_eq!(
            LoadTracker::diff_stage_key("AUTH-001", "pre-refactor", "src/main.rs"),
            "diff:AUTH-001:pre-refactor:src/main.rs"
        );
        assert_eq!(
            LoadTracker::diff_stage_key_path("src/app.rs"),
            "diff:src/app.rs"
        );
    }
}

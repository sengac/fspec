//! MUX-001 / BUG-167 — mux config persistence state (shared
//! `fspec-config.json`, `tui.mux`).
//!
//! Feature: spec/features/rust-mux-mode.feature +
//! spec/features/mux-config-persistence-wiring.feature
//!
//! The mux grid config (orientation, splits, pane list, focused pane,
//! enabled flag) is persisted in the shared CONFIG-008
//! `fspec-config.json` under `tui.mux` — the same pattern as
//! `tui.defaultThinkingLevel`.
//!
//! BUG-167: the dirs were once stored here (`set_persist_dir`) and had to be
//! wired manually at every construction site — in production that wiring was
//! never done, so every save returned `Err("mux: persist dirs not set")` and
//! was silently swallowed. This state is now CONFIG-ONLY: it holds the live
//! config and delegates load/save to the `codelet_sessions::
//! mux_config_persistence` GLOBALS, which resolve the CONFIG-008 two-scope
//! dirs themselves (process-global data dir + current dir — the same
//! resolution every other `fspec-config.json` persistence uses). The
//! serde round-trip lives here so `codelet-sessions` stays free of the TUI's
//! typed config.
//!
//! Invariants:
//!   * Missing / malformed key → the default preset (R6; load infallible).
//!   * Persistence is best-effort: `save` surfaces an `Err` the caller logs
//!     (non-fatal — `/mux save` pushes a one-line scrollback notice).

use crate::views::multiplex::MuxConfig;

/// The mux persistence state (config-only — no dirs).
#[derive(Debug, Default)]
pub struct MuxState {
    config: MuxConfig,
}

impl MuxState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn config(&self) -> &MuxConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut MuxConfig {
        &mut self.config
    }

    /// Load the persisted config from the shared `fspec-config.json`
    /// (`tui.mux`; R6: missing key → default preset). Reads the
    /// deep-merged (project-over-user) view via the shared-config globals.
    pub fn load(&mut self) {
        let Some(value) = codelet_sessions::mux_config_persistence::load_mux_config() else {
            return;
        };
        match serde_json::from_value::<MuxConfig>(value) {
            Ok(cfg) => self.config = cfg,
            Err(err) => {
                tracing::debug!("mux: invalid tui.mux value, using default: {err}");
            }
        }
    }

    /// Persist the current config under `tui.mux` (R6). Best-effort:
    /// failures are surfaced as `Err` (the `/mux save` caller pushes a
    /// one-line notice; the mux-exit auto-save logs and continues).
    pub fn save(&self) -> Result<(), String> {
        let value = serde_json::to_value(&self.config)
            .map_err(|err| format!("mux: cannot serialize config: {err}"))?;
        codelet_sessions::mux_config_persistence::save_mux_config(&value)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::{MuxOrientation, MuxPaneKind};
    use tempfile::TempDir;

    /// BUG-167: the config-only state round-trips through the path-
    /// injectable persistence core with explicit dirs (no global data dir
    /// needed).
    #[test]
    fn round_trip_persists_config() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        let mut state = MuxState::new();
        state.config_mut().orientation = MuxOrientation::Vertical;
        state.config_mut().splits = vec![40];
        state.config_mut().panes = vec![MuxPaneKind::Board, MuxPaneKind::Agent];

        let value = serde_json::to_value(state.config()).expect("serialize");
        codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(
            data.path(),
            cwd.path(),
            &value,
        )
        .expect("save");
        let raw = std::fs::read_to_string(data.path().join("fspec-config.json"))
            .expect("read fspec-config.json");
        assert!(
            raw.contains("\"mux\""),
            "the tui.mux key must round-trip: {raw}"
        );

        let reloaded_value =
            codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
                data.path(),
                cwd.path(),
            )
            .expect("load");
        let mut reloaded = MuxState::new();
        reloaded.config = serde_json::from_value(reloaded_value).expect("round-trip");
        assert_eq!(reloaded.config().orientation, MuxOrientation::Vertical);
        assert_eq!(reloaded.config().splits, vec![40]);
        assert_eq!(
            reloaded.config().panes,
            vec![MuxPaneKind::Board, MuxPaneKind::Agent]
        );
    }

    #[test]
    fn missing_key_falls_back_to_default() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        let value =
            codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
                data.path(),
                cwd.path(),
            );
        assert!(value.is_none(), "missing tui.mux must load as None");
    }

    #[test]
    fn save_preserves_sibling_keys() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        // Seed a sibling key (tui.lastUsedModel) the save must preserve.
        let cfg = serde_json::json!({
            "tui": { "lastUsedModel": "anthropic/claude" }
        });
        std::fs::create_dir_all(data.path()).expect("mkdir");
        std::fs::write(data.path().join("fspec-config.json"), cfg.to_string())
            .expect("seed config");

        let state = MuxState::new();
        let value = serde_json::to_value(state.config()).expect("serialize");
        codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(
            data.path(),
            cwd.path(),
            &value,
        )
        .expect("save");

        let raw = std::fs::read_to_string(data.path().join("fspec-config.json"))
            .expect("read fspec-config.json");
        assert!(
            raw.contains("lastUsedModel"),
            "sibling keys must survive: {raw}"
        );
        assert!(
            raw.contains("\"mux\""),
            "the tui.mux key must be written: {raw}"
        );
    }
}

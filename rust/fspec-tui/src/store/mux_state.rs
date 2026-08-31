//! MUX-001 — mux config persistence state (shared `fspec-config.json`).
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! The mux grid config (orientation, splits, pane list, focused pane,
//! enabled flag) is persisted in the shared CONFIG-008
//! `fspec-config.json` under `tui.mux` — the same pattern as
//! `tui.defaultThinkingLevel`. Missing / malformed key → default
//! preset (R6). Loaded at bootstrap; saved on `/mux save` and on mux
//! exit.
//!
//! The `serde_json::Value` ↔ `MuxConfig` round-trip happens here so the
//! `codelet-sessions` core stays free of the TUI's typed config.

use std::path::PathBuf;

use crate::views::multiplex::MuxConfig;

/// The mux persistence state.
#[derive(Debug, Default)]
pub struct MuxState {
    config: MuxConfig,
    /// `(data_dir, cwd)` selecting the shared config scopes. `None`
    /// until set (tests + bootstrap set it via
    /// `App::set_mux_persist_dir`).
    dirs: Option<(PathBuf, PathBuf)>,
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

    /// Set the shared-config scope dirs (data dir + cwd).
    pub fn set_persist_dir(&mut self, data_dir: PathBuf, cwd: PathBuf) {
        self.dirs = Some((data_dir, cwd));
    }

    /// Load the persisted config (R6: missing key → default preset).
    /// Corrupt values also fall back to the default preset (traced).
    pub fn load(&mut self) {
        let Some((data_dir, cwd)) = self.dirs.as_ref() else {
            return;
        };
        let Some(value) =
            codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(data_dir, cwd)
        else {
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
    /// failures are traced and swallowed (non-fatal).
    pub fn save(&self) -> Result<(), String> {
        let Some((data_dir, cwd)) = self.dirs.as_ref() else {
            return Err("mux: persist dirs not set".to_string());
        };
        let value = serde_json::to_value(&self.config)
            .map_err(|err| format!("mux: cannot serialize config: {err}"))?;
        codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(data_dir, cwd, &value)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::{MuxOrientation, MuxPaneKind};
    use tempfile::TempDir;

    #[test]
    fn round_trip_persists_config() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        let mut state = MuxState::new();
        state.set_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
        state.config_mut().orientation = MuxOrientation::Vertical;
        state.config_mut().splits = vec![40];
        state.config_mut().panes = vec![MuxPaneKind::Board, MuxPaneKind::Agent];

        state.save().expect("save");
        let raw = std::fs::read_to_string(data.path().join("fspec-config.json"))
            .expect("read fspec-config.json");
        assert!(
            raw.contains("\"mux\""),
            "the tui.mux key must round-trip: {raw}"
        );

        let mut reloaded = MuxState::new();
        reloaded.set_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
        reloaded.load();
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
        let mut state = MuxState::new();
        state.set_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
        state.load();
        assert_eq!(state.config().splits, vec![50]);
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

        let mut state = MuxState::new();
        state.set_persist_dir(data.path().to_path_buf(), cwd.path().to_path_buf());
        state.save().expect("save");

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

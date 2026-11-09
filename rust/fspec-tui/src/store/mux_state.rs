//! MUX-001 / BUG-167 — mux config persistence state (shared
//! `fspec-config.json`, `tui.mux`).
//!
//! Feature: spec/features/rust-mux-mode.feature +
//! spec/features/mux-config-persistence-wiring.feature
//!
//! The mux grid config (MUX-010 layout string, focused pane, enabled
//! flag) is persisted in the shared CONFIG-008
//! `fspec-config.json` under `tui.mux` — the same pattern as
//! `tui.thinkingLevels`.
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
//!   * Missing key → the default preset (R6). MUX-011 / BUG-708: a value,
//!     or an fspec-config.json, that cannot be read keeps the default preset for this session and `load` returns
//!     the exact error (the key, the 1-based column and the problem) for the
//!     App to show; nothing is written back, so the bad text stays on disk.
//!   * CLEAN-014: there is no migration of the removed `{orientation,
//!     splits, panes}` shape; such a value is unreadable (MUX-011 error).
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
    /// MUX-011: a saved value that cannot be read leaves the default
    /// preset in place and returns the exact error. BUG-708: so does an
    /// fspec-config.json that cannot be read or parsed (the error names it).
    pub fn load(&mut self) -> Result<(), String> {
        let loaded = codelet_sessions::mux_config_persistence::load_mux_config()
            .map_err(|err| format!("{err}; {DEFAULT_NOTE}"))?;
        let Some(value) = loaded else {
            return Ok(());
        };
        self.config = parse_saved(value)?;
        Ok(())
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

/// MUX-011 / BUG-708: what every load error ends with.
const DEFAULT_NOTE: &str = "using the default layout this session";

/// MUX-011: the saved `tui.mux` value as a config, or the exact error. The
/// layout string is parsed on its own so its error names `tui.mux.layout`
/// and the parser's 1-based column.
fn parse_saved(value: serde_json::Value) -> Result<MuxConfig, String> {
    if let Some(text) = value.get("layout").and_then(serde_json::Value::as_str) {
        if let Err(err) = crate::views::multiplex::MuxLayout::parse(text) {
            return Err(format!(
                "tui.mux.layout in fspec-config.json: {err}; {DEFAULT_NOTE}"
            ));
        }
    }
    serde_json::from_value::<MuxConfig>(value)
        .map_err(|err| format!("tui.mux in fspec-config.json: {err}; {DEFAULT_NOTE}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::MuxLayout;
    use tempfile::TempDir;

    /// BUG-167: the config-only state round-trips through the path-
    /// injectable persistence core with explicit dirs (no global data dir
    /// needed). MUX-010: as the layout string.
    #[test]
    fn round_trip_persists_config() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        let mut state = MuxState::new();
        state.config_mut().layout = MuxLayout::parse("[board:2, agent:3]").unwrap();

        let value = serde_json::to_value(state.config()).expect("serialize");
        assert_eq!(value["layout"], serde_json::json!("[board:2, agent:3]"));
        codelet_sessions::mux_config_persistence::save_mux_config_with_dirs(
            data.path(),
            cwd.path(),
            &value,
        )
        .expect("save");
        let reloaded_value = codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
            data.path(),
            cwd.path(),
        )
        .expect("load")
        .expect("saved");
        let mut reloaded = MuxState::new();
        reloaded.config = serde_json::from_value(reloaded_value).expect("round-trip");
        assert_eq!(reloaded.config().layout.to_string(), "[board:2, agent:3]");
    }

    #[test]
    fn missing_key_falls_back_to_default() {
        let data = TempDir::new().expect("data dir");
        let cwd = TempDir::new().expect("cwd");
        let value = codelet_sessions::mux_config_persistence::load_mux_config_with_dirs(
            data.path(),
            cwd.path(),
        )
        .expect("load");
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

    #[test]
    fn the_removed_shape_is_not_read_as_a_config() {
        let old = serde_json::json!({"orientation": "Horizontal", "splits": [50],
            "panes": ["Board", "Agent"], "focused_pane": 1, "enabled": true});
        assert!(serde_json::from_value::<MuxConfig>(old).is_err());
    }
}

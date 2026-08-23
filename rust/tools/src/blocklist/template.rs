//! BLOCK-012 — Bundled default system blocklist template.
//!
//! Feature: spec/features/auto-install-default-system-blocklist-template-when-fspec-blocklist-json-is-missing.feature
//!
//! The default blocklist (version 1.0.0, 68 rules covering Windows and
//! Linux dangerous commands, sensitive file paths, and agent-loop
//! protection) is embedded at compile time via `include_str!`, mirroring
//! the `codex_allowlist.rs` precedent (bundled JSON + user override at
//! `~/.fspec`). [`install_default_system_blocklist`] writes the template
//! to the system path only when the file is missing; failures are logged
//! and swallowed so a failed install never breaks command checking.

use std::path::Path;

use tracing::{info, warn};

use super::config::BlocklistConfig;

/// Bundled default blocklist template — embedded at compile time.
pub const DEFAULT_BLOCKLIST_TEMPLATE: &str = include_str!("../../data/default-blocklist.json");

/// Parse the embedded template into a [`BlocklistConfig`].
pub fn default_blocklist_config() -> Result<BlocklistConfig, serde_json::Error> {
    serde_json::from_str(DEFAULT_BLOCKLIST_TEMPLATE)
}

/// Write the embedded default template to `path` if it is missing.
///
/// Creates the parent directory when needed. All failures (no home dir,
/// read-only filesystem, unparseable template) are logged via
/// `tracing::warn!` and swallowed — a failed install must never break
/// command checking.
pub fn install_default_system_blocklist(path: &Path) {
    let config = match default_blocklist_config() {
        Ok(config) => config,
        Err(e) => {
            warn!("Embedded default blocklist template failed to parse: {e}");
            return;
        }
    };
    match config.save_to_file(path) {
        Ok(()) => info!("Installed default blocklist template at {path:?}"),
        Err(e) => warn!("Failed to install default blocklist at {path:?}: {e}"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn template_parses_with_expected_version_and_rule_count() {
        let config = default_blocklist_config().expect("embedded template must parse");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.rules.len(), 68);
    }
}

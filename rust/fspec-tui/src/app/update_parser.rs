//! `/update` subcommand parser + message formatting — UPD-002.
//!
//! Feature: spec/features/in-place-self-update-tui-command.feature
//!
//! Mirrors the grammar of `continue_parser.rs` (`ContinueSubcommand`),
//! routed from `slash_parser.rs::parse_slash_command` like the `/continue`
//! family:
//! * `/update` (bare) → [`UpdateSubcommand::CheckAndUpdate`]
//! * `/update check` → [`UpdateSubcommand::CheckOnly`]
//! * `/update <other>` → [`UpdateSubcommand::Invalid(arg)`]
//!
//! `format_update_message` renders the human-readable scrollback line for a
//! given [`UpdateOutcome`] so the dispatcher and tests share one source of
//! truth for the UX copy (up-to-date / installed / error).

use codelet_fspec_core::update::UpdateOutcome;

/// Outcome of parsing a `/update …` slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateSubcommand {
    /// `/update` — check for the latest release and install it if newer.
    CheckAndUpdate,
    /// `/update check` — report the latest version without downloading.
    CheckOnly,
    /// `/update <other>` — error notice, no action.
    Invalid(String),
}

/// Parse a `/update …` slash-command input into an [`UpdateSubcommand`].
/// `input` may begin with `/update` (which is stripped) or already be the
/// body.
pub fn parse_update_command(input: &str) -> UpdateSubcommand {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("/update").unwrap_or(trimmed).trim();
    if body.is_empty() {
        return UpdateSubcommand::CheckAndUpdate;
    }
    if body.eq_ignore_ascii_case("check") {
        return UpdateSubcommand::CheckOnly;
    }
    UpdateSubcommand::Invalid(body.to_string())
}

/// Render the scrollback line for an update result.
///
/// * `UpToDate` → `✓ fspec is up to date (v{version})`
/// * `Updated` → `✓ fspec v{version} installed. Restart fspec to activate.`
/// * `Failed` → `[error] update failed: {message}`
pub fn format_update_message(current: &str, outcome: &UpdateOutcome) -> String {
    match outcome {
        UpdateOutcome::UpToDate { version } => {
            format!("✓ fspec is up to date (v{version}) — current v{current}")
        }
        UpdateOutcome::Updated {
            version,
            restart_required,
        } => {
            if *restart_required {
                format!("✓ fspec v{version} installed. Restart fspec to activate.")
            } else {
                format!("✓ fspec v{version} installed.")
            }
        }
        UpdateOutcome::Failed { message } => {
            format!("error: update failed: {message}")
        }
    }
}

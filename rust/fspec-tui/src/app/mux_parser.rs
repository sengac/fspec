//! MUX-001 — `/mux` slash-command grammar parser.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Grammar (space-separated):
//!
//! ```text
//! mux := "mux" ( "on" | "off"
//!              | orientation
//!              | pane-count
//!              | pane-list (split-percent)?
//!              | "save" | "default" | "help" )
//! orientation  := "h" | "v" | "horizontal" | "vertical"
//! pane-count   := integer 2..=4
//! pane-list    := pane-kind { pane-kind }        # 2..=4 items
//! pane-kind    := "board" | "agent" | "files" | "checkpoints"
//! split-percent := integer 10..=90               # exactly one
//! ```
//!
//! Parse errors (R7) surface as a one-line message and leave the
//! current mux config untouched.

use thiserror::Error;

use crate::views::multiplex::{MuxOrientation, MuxPaneKind};

/// A parsed `/mux` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuxSubcommand {
    /// `/mux` (bare) — open the MuxConfigDialog (MUX-004: the on/off
    /// toggle moved into the dialog's Enabled row; the MUX-001 Toggle
    /// behaviour is superseded).
    Config,
    /// `/mux on` — enable with saved/default config.
    On,
    /// `/mux off` — disable, return to the pre-mux view.
    Off,
    /// `/mux h|v|horizontal|vertical`.
    Orientation(MuxOrientation),
    /// `/mux 2..=4` — set the pane count with default kinds.
    PaneCount(usize),
    /// `/mux <kinds...> [pct]` — explicit pane list + optional first
    /// split percent.
    PaneList {
        panes: Vec<MuxPaneKind>,
        split_percent: Option<u16>,
    },
    /// `/mux save` — persist the current config to the shared
    /// `fspec-config.json` under `tui.mux`.
    Save,
    /// `/mux default` — reset to the default preset.
    Default,
    /// `/mux help` — show the available subcommands.
    Help,
}

/// `/mux` parse error (R7: one-line, config untouched).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MuxError {
    #[error("/mux: unknown pane kind: {0}")]
    UnknownPaneKind(String),
    #[error("/mux: split percent must be 10..=90, got {0}")]
    SplitPercentOutOfRange(u16),
    #[error("/mux: pane count must be 2..=4, got {0}")]
    PaneCountOutOfRange(usize),
    #[error("/mux: at least 2 panes required, got {0}")]
    TooFewPanes(usize),
    #[error("/mux: at most 4 panes supported, got {0}")]
    TooManyPanes(usize),
    #[error("/mux: unrecognized subcommand: {0}")]
    UnknownSubcommand(String),
}

/// Parse a submitted `/mux …` line. Bare `/mux` (no args) resolves to
/// [`MuxSubcommand::Config`] (opens the MuxConfigDialog — MUX-004).
pub fn parse_mux_command(line: &str) -> Result<MuxSubcommand, MuxError> {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("/mux") else {
        return Err(MuxError::UnknownSubcommand(trimmed.to_string()));
    };
    let args: Vec<&str> = rest.split_whitespace().collect();
    if args.is_empty() {
        return Ok(MuxSubcommand::Config);
    }

    match args[0] {
        "on" => Ok(MuxSubcommand::On),
        "off" => Ok(MuxSubcommand::Off),
        "h" | "horizontal" => Ok(MuxSubcommand::Orientation(MuxOrientation::Horizontal)),
        "v" | "vertical" => Ok(MuxSubcommand::Orientation(MuxOrientation::Vertical)),
        "save" => Ok(MuxSubcommand::Save),
        "default" => Ok(MuxSubcommand::Default),
        "help" => Ok(MuxSubcommand::Help),
        first => {
            // Pane count: a bare integer 2..=4.
            if let Ok(n) = first.parse::<usize>() {
                if (2..=4).contains(&n) {
                    return Ok(MuxSubcommand::PaneCount(n));
                }
                return Err(MuxError::PaneCountOutOfRange(n));
            }
            // Pane list: 2..=4 kinds, optional trailing split percent.
            parse_pane_list(&args)
        }
    }
}

fn parse_pane_list(args: &[&str]) -> Result<MuxSubcommand, MuxError> {
    // Count the non-numeric tokens (pane kinds) FIRST so an over-long
    // list errors as TooManyPanes even when a trailing token is not a
    // valid kind.
    let kind_count = args.iter().filter(|a| a.parse::<u16>().is_err()).count();
    if kind_count > 4 {
        return Err(MuxError::TooManyPanes(kind_count));
    }
    if kind_count < 2 {
        return Err(MuxError::TooFewPanes(kind_count));
    }
    let mut panes: Vec<MuxPaneKind> = Vec::new();
    let mut split_percent: Option<u16> = None;
    for (i, arg) in args.iter().enumerate() {
        if let Ok(pct) = arg.parse::<u16>() {
            // A numeric token is the split percent — only allowed as
            // the FINAL token, and only once.
            if i != args.len() - 1 || split_percent.is_some() {
                return Err(MuxError::SplitPercentOutOfRange(pct));
            }
            if !(10..=90).contains(&pct) {
                return Err(MuxError::SplitPercentOutOfRange(pct));
            }
            split_percent = Some(pct);
            continue;
        }
        match parse_pane_kind(arg) {
            Some(kind) => panes.push(kind),
            None => return Err(MuxError::UnknownPaneKind(arg.to_string())),
        }
    }
    Ok(MuxSubcommand::PaneList {
        panes,
        split_percent,
    })
}

fn parse_pane_kind(arg: &str) -> Option<MuxPaneKind> {
    match arg {
        "board" => Some(MuxPaneKind::Board),
        "agent" => Some(MuxPaneKind::Agent),
        "files" => Some(MuxPaneKind::ChangedFiles),
        "checkpoints" => Some(MuxPaneKind::Checkpoints),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn bare_and_lifecycle_subcommands() {
        // MUX-004: bare /mux resolves to Config (opens the MuxConfigDialog),
        // no longer Toggle.
        assert_eq!(parse_mux_command("/mux"), Ok(MuxSubcommand::Config));
        assert_eq!(parse_mux_command("/mux on"), Ok(MuxSubcommand::On));
        assert_eq!(parse_mux_command("/mux off"), Ok(MuxSubcommand::Off));
        assert_eq!(parse_mux_command("/mux save"), Ok(MuxSubcommand::Save));
        assert_eq!(
            parse_mux_command("/mux default"),
            Ok(MuxSubcommand::Default)
        );
        assert_eq!(parse_mux_command("/mux help"), Ok(MuxSubcommand::Help));
    }

    #[test]
    fn orientation_and_count() {
        assert_eq!(
            parse_mux_command("/mux v"),
            Ok(MuxSubcommand::Orientation(MuxOrientation::Vertical))
        );
        assert_eq!(parse_mux_command("/mux 3"), Ok(MuxSubcommand::PaneCount(3)));
        assert!(matches!(
            parse_mux_command("/mux 5"),
            Err(MuxError::PaneCountOutOfRange(5))
        ));
    }

    #[test]
    fn pane_list_with_split() {
        assert_eq!(
            parse_mux_command("/mux board agent 40"),
            Ok(MuxSubcommand::PaneList {
                panes: vec![MuxPaneKind::Board, MuxPaneKind::Agent],
                split_percent: Some(40),
            })
        );
        assert!(matches!(
            parse_mux_command("/mux board zzz"),
            Err(MuxError::UnknownPaneKind(ref k)) if k == "zzz"
        ));
        assert!(matches!(
            parse_mux_command("/mux board agent 5"),
            Err(MuxError::SplitPercentOutOfRange(5))
        ));
    }
}

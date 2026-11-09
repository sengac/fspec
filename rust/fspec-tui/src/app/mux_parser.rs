//! MUX-001/MUX-010 — `/mux` slash-command grammar parser.
//!
//! Feature: spec/features/rust-mux-mode.feature +
//! spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! Grammar:
//!
//! ```text
//! mux := "mux" [ "on" | "off" | "save" | "default" | "help" | layout ]
//! layout := see views::multiplex::layout_tree::parse
//!           e.g. agent, [board, board], board, files
//! ```
//!
//! Bare `/mux` opens the MuxConfigDialog. The removed MUX-001 forms
//! (`h|v|horizontal|vertical`, a pane count, a space-separated pane list
//! with an optional split percent) fail with [`MuxError::RemovedSyntax`],
//! whose text shows the layout syntax. Errors are one line and leave the
//! current mux config untouched.

use thiserror::Error;

use crate::views::multiplex::{MuxLayout, MuxLayoutError};

/// The layout example every syntax hint shows.
pub const MUX_LAYOUT_EXAMPLE: &str = "/mux agent, [board, board], board, files";

/// A parsed `/mux` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuxSubcommand {
    /// `/mux` (bare) — open the MuxConfigDialog (MUX-004).
    Config,
    /// `/mux on` — enable with saved/default config.
    On,
    /// `/mux off` — disable, return to the pre-mux view.
    Off,
    /// MUX-010: `/mux <layout>` — apply the layout and enable mux.
    Layout(MuxLayout),
    /// `/mux save` — persist the current config to the shared
    /// `fspec-config.json` under `tui.mux`.
    Save,
    /// `/mux default` — reset to the default preset.
    Default,
    /// `/mux help` — show the available subcommands.
    Help,
}

/// `/mux` parse error (one line, config untouched).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MuxError {
    #[error("/mux: {0}")]
    Layout(#[from] MuxLayoutError),
    #[error(
        "/mux: '{0}' is the removed mux syntax; write a layout string, e.g. {MUX_LAYOUT_EXAMPLE}"
    )]
    RemovedSyntax(String),
    #[error("/mux: unrecognized subcommand: {0}")]
    UnknownSubcommand(String),
}

/// True for the removed MUX-001 argument forms: an orientation word, a
/// bare integer, or two or more space-separated words with none of the
/// layout punctuation.
fn is_removed_syntax(args: &str) -> bool {
    let words: Vec<&str> = args.split_whitespace().collect();
    let orientation = matches!(words.as_slice(), ["h" | "v" | "horizontal" | "vertical"]);
    let count = words.len() == 1 && words[0].chars().all(|c| c.is_ascii_digit());
    let spaced = words.len() > 1 && !args.contains([',', '[', ']', ':']);
    orientation || count || spaced
}

/// Parse a submitted `/mux …` line. Bare `/mux` (no args) resolves to
/// [`MuxSubcommand::Config`] (opens the MuxConfigDialog — MUX-004).
pub fn parse_mux_command(line: &str) -> Result<MuxSubcommand, MuxError> {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("/mux") else {
        return Err(MuxError::UnknownSubcommand(trimmed.to_string()));
    };
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Err(MuxError::UnknownSubcommand(trimmed.to_string()));
    }
    let args = rest.trim();
    match args {
        "" => Ok(MuxSubcommand::Config),
        "on" => Ok(MuxSubcommand::On),
        "off" => Ok(MuxSubcommand::Off),
        "save" => Ok(MuxSubcommand::Save),
        "default" => Ok(MuxSubcommand::Default),
        "help" => Ok(MuxSubcommand::Help),
        _ if is_removed_syntax(args) => Err(MuxError::RemovedSyntax(args.to_string())),
        _ => Ok(MuxSubcommand::Layout(MuxLayout::parse(args)?)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn bare_and_lifecycle_subcommands() {
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
    fn a_layout_string_parses() {
        let Ok(MuxSubcommand::Layout(layout)) =
            parse_mux_command("/mux agent, [board, board], board, files")
        else {
            panic!("a layout");
        };
        assert_eq!(layout.to_string(), "agent, [board, board], board, files");
        // A single pane is a layout, not the removed syntax.
        assert!(matches!(
            parse_mux_command("/mux board"),
            Ok(MuxSubcommand::Layout(_))
        ));
    }

    #[test]
    fn the_removed_syntax_names_the_new_one() {
        for args in [
            "v",
            "horizontal",
            "3",
            "board agent 40",
            "files checkpoints",
        ] {
            assert_eq!(
                parse_mux_command(&format!("/mux {args}"))
                    .unwrap_err()
                    .to_string(),
                format!(
                    "/mux: '{args}' is the removed mux syntax; write a layout string, \
                     e.g. /mux agent, [board, board], board, files"
                )
            );
        }
    }

    #[test]
    fn layout_errors_carry_the_column() {
        assert_eq!(
            parse_mux_command("/mux board, zzz")
                .unwrap_err()
                .to_string(),
            "/mux: unknown pane 'zzz' at column 8; panes are board, agent, files, checkpoints"
        );
        assert_eq!(
            parse_mux_command("/muxx").unwrap_err(),
            MuxError::UnknownSubcommand("/muxx".to_string())
        );
    }
}

//! Help-request routing for the native Rust fspec dispatcher (RPC-414).
//!
//! [`dispatch_command`](crate::dispatch::dispatch_command) calls
//! [`try_dispatch_help`] once, BEFORE the canonical command lookup. When the
//! incoming tool call is a help request this module renders the answer from
//! the existing help registry ([`crate::help::format_command_help`] +
//! [`crate::help::configs`]); otherwise it returns `None` so normal dispatch
//! proceeds byte-identically to before.
//!
//! Recognised help shapes:
//!   1. `command == "help"` with no `args.command`     → general tool help.
//!   2. `command == "help"` with `args.command == "x"` → per-command help for `x`.
//!   3. `command` ending in ` --help` or ` -h`         → per-command help for
//!      the stripped command name (whitespace-tolerant).
//!
//! Feature: spec/features/fspec-tool-help-dispatch.feature

use serde_json::Value;

use crate::dispatch::DispatchResult;
use crate::error::FspecCoreError;
use crate::help::format_command_help;
use crate::help_dispatch_table::config_for;

/// General Fspec tool help. Deliberately concise and factual: it explains the
/// two ways to obtain per-command usage docs. Test S4 asserts this text
/// mentions `--help`.
const GENERAL_HELP: &str = "\
FSPEC TOOL HELP

The Fspec tool manages Gherkin feature specifications and project work units.

GETTING PER-COMMAND HELP
  • Append --help (or -h) to a command to see its usage doc.
      e.g. command: \"create-prefix --help\"
  • Or call command: \"help\" with args {\"command\": \"<name>\"}.
      e.g. command: \"help\", args: {\"command\": \"create-prefix\"}

Each per-command doc lists the command's arguments, options, and examples.";

/// Inspect a dispatch request and, if it is a help request, produce the
/// rendered [`DispatchResult`]. Returns `None` for non-help inputs so the
/// caller's normal canonical-lookup path runs unchanged.
pub(crate) fn try_dispatch_help(command: &str, args_json: &str) -> Option<DispatchResult> {
    // Precedence: the literal `help` command (Shape 1/2) is matched BEFORE the
    // trailing-flag form (Shape 3), so an input like `"help --help"` resolves
    // to general help rather than being treated as help-for-command-"help".
    // Shape 1 & 2: the literal `help` command.
    if command == "help" {
        return Some(match parse_args_command(args_json) {
            Some(name) => render_command_help(&name),
            None => success(GENERAL_HELP.to_string()),
        });
    }

    // Shape 3: a trailing `--help` / `-h` flag on an otherwise normal command.
    strip_trailing_help_flag(command).map(|name| render_command_help(&name))
}

/// Render per-command help for a kebab-case `name`.
fn render_command_help(name: &str) -> DispatchResult {
    if let Some(config) = config_for(name) {
        return success(format_command_help(config));
    }

    // A real canonical command that simply ships no CONFIG must degrade
    // gracefully — NOT surface as an UnknownCommand error.
    if crate::canonical::lookup(name).is_some() {
        return success(format!(
            "No detailed help is available for `{name}`.\n  Usage: fspec {name}"
        ));
    }

    // Not a canonical command at all → UnknownCommand naming the stripped name.
    failure(FspecCoreError::UnknownCommand {
        command: name.to_string(),
    })
}

/// Parse `args_json` defensively and extract a non-blank `command` field.
/// Missing / blank / invalid JSON / non-string all map to `None` (treated as
/// "no args.command").
fn parse_args_command(args_json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(args_json).ok()?;
    let raw = value.get("command")?.as_str()?.trim();
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

/// If `command` ends with a ` --help` or ` -h` help flag, return the stripped,
/// trimmed command name. Whitespace-tolerant: any run of whitespace before the
/// flag is collapsed away. Returns `None` when no trailing help flag is present
/// or when stripping would leave an empty name.
fn strip_trailing_help_flag(command: &str) -> Option<String> {
    let trimmed = command.trim_end();
    for flag in ["--help", "-h"] {
        if let Some(prefix) = trimmed.strip_suffix(flag) {
            // The flag must be preceded by whitespace so we don't mangle a
            // command whose own name happens to end in the flag text.
            if prefix.ends_with(char::is_whitespace) {
                let name = prefix.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Build a successful help [`DispatchResult`].
fn success(data: String) -> DispatchResult {
    DispatchResult {
        success: true,
        data,
        error: None,
        system_reminder: None,
    }
}

/// Build a failing help [`DispatchResult`] from an error.
fn failure(err: FspecCoreError) -> DispatchResult {
    DispatchResult {
        success: false,
        data: String::new(),
        error: Some(err.to_string()),
        system_reminder: None,
    }
}

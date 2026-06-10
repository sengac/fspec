//! Help-text formatter — Rust port of `src/utils/help-formatter.ts`.
//!
//! Mirrors the TypeScript `formatCommandHelp(config)` function used by every
//! TS Commander.js subcommand's `--help` action. The output is byte-for-byte
//! identical to the TS reference when piped to a non-TTY (no ANSI colour),
//! so that the standalone fspec Rust binary can serve the same `--help`
//! contract to shell users.
//!
//! Feature: spec/features/list-hooks-cli-subcommand.feature (RPC-247)
//! Originating TS source: src/utils/help-formatter.ts lines 44-187.
//!
//! NOTE on colour: the TS formatter wraps section headers in `chalk.bold`,
//! commands in `chalk.cyan`, and similar. When stdout is not a TTY (or
//! NO_COLOR is set) those wrappers reduce to identity. Rust mirrors the
//! non-colour path only, because the byte-parity contract is defined against
//! piped/captured TS output.

pub mod configs;

/// A documented command-line option (flag).
pub struct CommandOption {
    pub flag: &'static str,
    pub description: &'static str,
    pub default_value: Option<&'static str>,
}

/// A documented positional or named argument.
pub struct CommandArgument {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

/// A documented example invocation.
pub struct CommandExample {
    pub command: &'static str,
    pub description: Option<&'static str>,
    pub output: Option<&'static str>,
}

/// A documented common pattern.
pub struct CommonPattern {
    pub pattern: &'static str,
    pub example: &'static str,
    pub description: &'static str,
}

/// Either a free-form string or a structured pattern. Mirrors the TS
/// `string[] | CommonPattern[]` union type.
pub enum CommonPatternEntry {
    Bullet(&'static str),
    Structured(CommonPattern),
}

/// A documented common error and its remediation.
pub struct CommonError {
    pub error: &'static str,
    pub fix: &'static str,
}

/// The complete help-page configuration for a single `fspec <cmd>` subcommand.
/// Mirrors the TS `CommandHelpConfig` interface at
/// `src/utils/help-formatter.ts:27-42`.
pub struct CommandHelpConfig {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: Option<&'static str>,
    pub arguments: &'static [CommandArgument],
    pub options: &'static [CommandOption],
    pub examples: &'static [CommandExample],
    pub related_commands: &'static [&'static str],
    pub when_to_use: Option<&'static str>,
    pub when_not_to_use: Option<&'static str>,
    pub prerequisites: &'static [&'static str],
    pub common_patterns: &'static [CommonPatternEntry],
    pub typical_workflow: Option<&'static str>,
    pub common_errors: &'static [CommonError],
    pub notes: &'static [&'static str],
}

/// Render the help text for `config`, byte-for-byte matching the TS
/// `formatCommandHelp` output when piped to non-TTY.
///
/// The TS implementation builds an array of lines and joins with `\n`. We
/// preserve that contract exactly: the returned `String` does NOT have a
/// trailing newline (the empty trailing line from `lines.push('')` is the
/// final element joined into the output).
pub fn format_command_help(config: &CommandHelpConfig) -> String {
    // Header
    let mut lines: Vec<String> = vec![
        String::new(),
        config.name.to_uppercase(),
        config.description.to_string(),
        String::new(),
    ];

    // When to use
    if let Some(s) = config.when_to_use {
        lines.push("WHEN TO USE".to_string());
        lines.push(format!("  {s}"));
        lines.push(String::new());
    }

    // When NOT to use
    if let Some(s) = config.when_not_to_use {
        lines.push("WHEN NOT TO USE".to_string());
        lines.push(format!("  {s}"));
        lines.push(String::new());
    }

    // Prerequisites
    if !config.prerequisites.is_empty() {
        lines.push("PREREQUISITES".to_string());
        for p in config.prerequisites {
            lines.push(format!("  • {p}"));
        }
        lines.push(String::new());
    }

    // Usage (always emitted; TS defaults to `fspec <name>` when missing)
    lines.push("USAGE".to_string());
    let usage_default = format!("fspec {}", config.name);
    let usage_line = config.usage.unwrap_or(&usage_default);
    lines.push(format!("  {usage_line}"));
    lines.push(String::new());

    // Arguments
    if !config.arguments.is_empty() {
        lines.push("ARGUMENTS".to_string());
        for arg in config.arguments {
            let arg_name = if arg.required {
                format!("<{}>", arg.name)
            } else {
                format!("[{}]", arg.name)
            };
            let required_marker = if arg.required {
                "(required)"
            } else {
                "(optional)"
            };
            lines.push(format!("  {arg_name} {required_marker}"));
            lines.push(format!("    {}", arg.description));
        }
        lines.push(String::new());
    }

    // Options
    if !config.options.is_empty() {
        lines.push("OPTIONS".to_string());
        for opt in config.options {
            lines.push(format!("  {}", opt.flag));
            lines.push(format!("    {}", opt.description));
            if let Some(default) = opt.default_value {
                lines.push(format!("    Default: {default}"));
            }
        }
        lines.push(String::new());
    } else {
        lines.push("OPTIONS".to_string());
        lines.push("  No options available".to_string());
        lines.push(String::new());
    }

    // Common Patterns
    if !config.common_patterns.is_empty() {
        lines.push("COMMON PATTERNS".to_string());
        for pattern in config.common_patterns {
            match pattern {
                CommonPatternEntry::Bullet(s) => {
                    lines.push(format!("  • {s}"));
                }
                CommonPatternEntry::Structured(p) => {
                    lines.push(format!("  • {}", p.pattern));
                    lines.push(format!("    Example: {}", p.example));
                    lines.push(format!("    {}", p.description));
                    lines.push(String::new());
                }
            }
        }
        lines.push(String::new());
    }

    // Typical Workflow
    if let Some(s) = config.typical_workflow {
        lines.push("TYPICAL WORKFLOW".to_string());
        lines.push(format!("  {s}"));
        lines.push(String::new());
    }

    // Examples
    if !config.examples.is_empty() {
        lines.push("EXAMPLES".to_string());
        let last_idx = config.examples.len() - 1;
        for (i, ex) in config.examples.iter().enumerate() {
            if let Some(d) = ex.description {
                lines.push(format!("  {}. {}", i + 1, d));
            }
            lines.push(format!("  $ {}", ex.command));
            if let Some(o) = ex.output {
                lines.push(format!("  {o}"));
            }
            if i < last_idx {
                lines.push(String::new());
            }
        }
        lines.push(String::new());
    }

    // Common Errors
    if !config.common_errors.is_empty() {
        lines.push("COMMON ERRORS".to_string());
        for err in config.common_errors {
            lines.push(format!("  ✗ {}", err.error));
            lines.push(format!("    Fix: {}", err.fix));
            lines.push(String::new());
        }
    }

    // Related Commands
    if !config.related_commands.is_empty() {
        lines.push("RELATED COMMANDS".to_string());
        for cmd in config.related_commands {
            lines.push(format!("  fspec {cmd}"));
        }
        lines.push(String::new());
    }

    // Notes
    if !config.notes.is_empty() {
        lines.push("NOTES".to_string());
        for note in config.notes {
            lines.push(format!("  • {note}"));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

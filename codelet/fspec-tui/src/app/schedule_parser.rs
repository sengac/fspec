//! `/schedule` subcommand parser — RPC-058.
//!
//! Mirrors `src/tui/utils/scheduleCommandParser.ts::parseScheduleCommand`
//! from the TS Ink frontend. Tokenises quoted strings so that a cron
//! expression with spaces (e.g. `"0 9 * * *"`) or a shell command with
//! spaces (e.g. `"tar -czf ..."`) survives a single flag argument.
//!
//! Supported subcommands:
//! * `add <name> --cron <expr> --tz <zone> [--role <r> --prompt <p>]
//!   [--command <cmd>] [--overlap skip|queue]`
//! * `list`
//! * `pause <name>`
//! * `resume <name>`
//! * `remove <name>`
//! * `help` (bare or unknown subcommand)
//!
//! `job_type` is inferred from the presence of `--command`: shell when
//! present, agent otherwise.

/// Outcome of parsing a `/schedule …` slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleSubcommand {
    /// `/schedule add <name> --cron <expr> --tz <zone> …`
    Add {
        name: String,
        cron: String,
        timezone: String,
        /// "agent" | "shell" (inferred from --command presence)
        job_type: String,
        role: Option<String>,
        prompt: Option<String>,
        command: Option<String>,
        /// "skip" | "queue" — `None` means the engine treats as "skip".
        overlap_policy: Option<String>,
    },
    /// `/schedule list`
    List,
    /// `/schedule pause <name>`
    Pause { name: String },
    /// `/schedule resume <name>`
    Resume { name: String },
    /// `/schedule remove <name>`
    Remove { name: String },
    /// `/schedule` (bare) or an unknown subcommand — fall through to the
    /// help notice.
    Help,
}

/// Tokenise `input`, respecting double-quoted strings. Strips the leading
/// `/schedule` segment so callers can pass the raw input.
fn tokenise(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    // Strip leading "/schedule" (the rest of the string is the subcommand).
    let body = trimmed
        .strip_prefix("/schedule")
        .map(str::trim_start)
        .unwrap_or(trimmed);

    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaping = false;
    for ch in body.chars() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }
        if ch == '\\' {
            escaping = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Parse a `/schedule …` slash command into a [`ScheduleSubcommand`].
pub fn parse_schedule_command(input: &str) -> ScheduleSubcommand {
    let tokens = tokenise(input);
    if tokens.is_empty() {
        return ScheduleSubcommand::Help;
    }
    let head = tokens[0].as_str();
    match head {
        "list" => ScheduleSubcommand::List,
        "pause" => match tokens.get(1) {
            Some(name) => ScheduleSubcommand::Pause {
                name: name.clone(),
            },
            None => ScheduleSubcommand::Help,
        },
        "resume" => match tokens.get(1) {
            Some(name) => ScheduleSubcommand::Resume {
                name: name.clone(),
            },
            None => ScheduleSubcommand::Help,
        },
        "remove" => match tokens.get(1) {
            Some(name) => ScheduleSubcommand::Remove {
                name: name.clone(),
            },
            None => ScheduleSubcommand::Help,
        },
        "add" => parse_add(&tokens),
        _ => ScheduleSubcommand::Help,
    }
}

fn parse_add(tokens: &[String]) -> ScheduleSubcommand {
    // tokens[0] == "add", tokens[1] == <name>, rest is flag pairs.
    let Some(name) = tokens.get(1) else {
        return ScheduleSubcommand::Help;
    };
    let mut cron: Option<String> = None;
    let mut timezone: Option<String> = None;
    let mut role: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut command: Option<String> = None;
    let mut overlap_policy: Option<String> = None;

    let mut i = 2;
    while i < tokens.len() {
        let flag = tokens[i].as_str();
        let value = tokens.get(i + 1).cloned();
        match flag {
            "--cron" => {
                cron = value;
                i += 2;
            }
            "--tz" | "--timezone" => {
                timezone = value;
                i += 2;
            }
            "--role" => {
                role = value;
                i += 2;
            }
            "--prompt" => {
                prompt = value;
                i += 2;
            }
            "--command" => {
                command = value;
                i += 2;
            }
            "--overlap" | "--overlap-policy" => {
                overlap_policy = value;
                i += 2;
            }
            _ => {
                // Unknown flag — skip the flag and its (possible) value
                // so we don't get stuck. The TS parser does the same.
                i += if value.is_some() { 2 } else { 1 };
            }
        }
    }

    let job_type = if command.is_some() {
        "shell".to_string()
    } else {
        "agent".to_string()
    };

    ScheduleSubcommand::Add {
        name: name.clone(),
        cron: cron.unwrap_or_default(),
        timezone: timezone.unwrap_or_default(),
        job_type,
        role,
        prompt,
        command,
        overlap_policy,
    }
}

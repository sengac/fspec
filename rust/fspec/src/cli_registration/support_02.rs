/// Whether the option token (`--epic` / `-e`) for the given subcommand
/// consumes a following value.
fn option_takes_value(sub: Option<&clap::Command>, tok: &str) -> bool {
    let Some(sub) = sub else { return false };
    let flag = tok.trim_start_matches('-');
    for arg in sub.get_arguments() {
        let matches_long = arg.get_long().map(|l| l == flag).unwrap_or(false);
        let matches_short = arg
            .get_short()
            .map(|s| s.to_string() == flag)
            .unwrap_or(false);
        if matches_long || matches_short {
            // Prefer the explicit num_args when clap has finalised it; otherwise
            // fall back to the ArgAction, which the derive layer always sets
            // (Set/Append consume a value; SetTrue/SetFalse/Count/Help/Version
            // are valueless flags). Relying on num_args alone is unsafe because
            // `Command::command()` does not finalise num_args for derive-defined
            // options, leaving it `None` and mis-counting option values as
            // excess positionals (TS parity: Commander counts only true
            // positionals).
            if let Some(n) = arg.get_num_args() {
                return n.takes_values();
            }
            return matches!(
                arg.get_action(),
                clap::ArgAction::Set | clap::ArgAction::Append
            );
        }
    }
    false
}

/// The subcommand name from argv (first non-flag token after the program),
/// or `None` if absent.
fn subcommand_token() -> Option<String> {
    std::env::args().nth(1).filter(|a| !a.starts_with('-'))
}

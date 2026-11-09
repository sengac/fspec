/// Render a clap parse error in the byte-exact Commander.js format used by
/// the TypeScript `fspec` reference, then exit with the matching code.
///
/// The TS CLI uses Commander.js, whose argument/usage errors differ from
/// clap's default output in BOTH wording and exit code:
///
/// | situation                | Commander.js (TS)                                        | exit |
/// |--------------------------|----------------------------------------------------------|------|
/// | help / version requested | clap's own help/version text                             | 0    |
/// | missing required arg     | `error: missing required argument '<name>'`              | 1    |
/// | extra positional         | `error: too many arguments for '<cmd>'. Expected N ...`  | 1    |
/// | unknown option           | `error: unknown option '<flag>'`                         | 1    |
/// | option needs a value     | `error: option '<spec>' argument missing`                | 1    |
/// | unknown subcommand       | `error: unknown command '<name>'`                        | 1    |
///
/// clap reports usage errors on stderr with exit code 2 and the multi-line
/// "the following required arguments were not provided" block. This helper
/// re-renders the FIRST offending item in Commander's single-line style.
///
/// Help (`--help`/`-h`) and version (`--version`/`-V`) are NOT errors in the
/// Commander sense — clap surfaces them as `DisplayHelp` / `DisplayVersion`
/// "errors" carrying the rendered text. We print that text verbatim on stdout
/// and exit 0, matching the TS behaviour.
fn render_clap_error(err: clap::Error) -> std::process::ExitCode {
    match err.kind() {
        // clap routes help/version text through Error::print(); reuse it so
        // the rendered block matches clap's own formatting and exits 0.
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::ExitCode::from(0)
        }
        ErrorKind::MissingRequiredArgument => {
            // clap stores the missing items as usage tokens. Commander
            // reports only the FIRST, and distinguishes positional arguments
            // from required options:
            //   - positional `<TITLE>`        → `missing required argument 'title'`
            //   - required option `--role ...` → `required option '--role <role>' not specified`
            match first_invalid_arg(&err) {
                Some(tok) if tok.starts_with('-') => {
                    eprintln!(
                        "error: required option '{}' not specified",
                        commander_option_spec(&tok)
                    );
                }
                Some(tok) => {
                    eprintln!(
                        "error: missing required argument '{}'",
                        commander_arg_name(&tok)
                    );
                }
                None => {
                    eprintln!("error: missing required argument ''");
                }
            }
            std::process::ExitCode::from(1)
        }
        ErrorKind::UnknownArgument => {
            // clap stores the typed token (e.g. "--zzz" or "-z"). For an
            // extra positional value clap ALSO raises UnknownArgument, but
            // Commander reports that as a "too many arguments" error — we
            // distinguish by whether the token starts with '-'.
            match first_invalid_arg(&err) {
                Some(tok) if tok.starts_with('-') => {
                    eprintln!("error: unknown option '{tok}'");
                }
                _ => {
                    eprintln!("{}", too_many_arguments_message());
                }
            }
            std::process::ExitCode::from(1)
        }
        ErrorKind::TooManyValues => {
            eprintln!("{}", too_many_arguments_message());
            std::process::ExitCode::from(1)
        }
        ErrorKind::InvalidValue if option_needs_value(&err) => {
            // clap: "a value is required for '...' but none was supplied".
            // Commander: "option '<spec>' argument missing".
            eprintln!(
                "error: option '{}' argument missing",
                invalid_arg_spec(&err).unwrap_or_default()
            );
            std::process::ExitCode::from(1)
        }
        ErrorKind::NoEquals => {
            eprintln!(
                "error: option '{}' argument missing",
                invalid_arg_spec(&err).unwrap_or_default()
            );
            std::process::ExitCode::from(1)
        }
        ErrorKind::InvalidSubcommand => {
            // clap stores the rejected subcommand under InvalidSubcommand;
            // fall back to argv[1] if absent.
            let name = match err.get(ContextKind::InvalidSubcommand) {
                Some(ContextValue::String(s)) => s.clone(),
                _ => subcommand_token().unwrap_or_default(),
            };
            eprintln!("error: unknown command '{name}'");
            std::process::ExitCode::from(1)
        }
        // Any other clap error: fall back to clap's own rendering + exit 1
        // (Commander always exits 1 on argument errors).
        _ => {
            let _ = err.print();
            std::process::ExitCode::from(1)
        }
    }
}

/// Extract the first `InvalidArg` context string from a clap error.
fn first_invalid_arg(err: &clap::Error) -> Option<String> {
    match err.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(s)) => Some(s.clone()),
        Some(ContextValue::Strings(v)) => v.first().cloned(),
        _ => None,
    }
}

/// Render the full `-d, --description <description>` style flag spec for an
/// option error (Commander includes BOTH short and long forms).
fn invalid_arg_spec(err: &clap::Error) -> Option<String> {
    match err.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(s)) => Some(commander_option_spec(s)),
        Some(ContextValue::Strings(v)) => v.first().map(|s| commander_option_spec(s)),
        _ => None,
    }
}

/// Whether an `InvalidValue` error is the "option requires a value" variant.
/// clap's `empty_value` constructor stores an EMPTY `InvalidValue` string —
/// distinguishing the "needs a value" case from a genuine bad-value rejection
/// (which carries the offending value).
fn option_needs_value(err: &clap::Error) -> bool {
    matches!(
        err.get(ContextKind::InvalidValue),
        Some(ContextValue::String(s)) if s.is_empty()
    )
}

/// Convert a clap usage token / value_name to the Commander argument name.
///
/// clap surfaces required-argument tokens from the declared `value_name`,
/// which in this codebase is either `UPPER_SNAKE` (e.g. `WORK_UNIT_ID`, auto
/// or explicit) or already the verbatim Commander name in camelCase
/// (e.g. `workUnitId`). Commander uses the `.argument('<name>')` name verbatim:
/// a single lowercase word (`title`) or camelCase for multi-word names
/// (`workUnitId`). We strip the angle brackets and:
///   - `UPPER_SNAKE` / `snake_case` (contains `_`) → `camelCase`;
///   - already mixed-case (e.g. `workUnitId`) → returned verbatim;
///   - a single all-upper or all-lower word (e.g. `TITLE`, `file`) → lowercased.
fn commander_arg_name(token: &str) -> String {
    let trimmed = token.trim_start_matches('<').trim_end_matches('>');
    if trimmed.contains('_') {
        // UPPER_SNAKE / snake_case → camelCase.
        let words: Vec<&str> = trimmed.split('_').filter(|w| !w.is_empty()).collect();
        let mut out = String::new();
        for (i, w) in words.iter().enumerate() {
            let lower = w.to_lowercase();
            if i == 0 {
                out.push_str(&lower);
            } else {
                let mut chars = lower.chars();
                if let Some(first) = chars.next() {
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                }
            }
        }
        out
    } else if trimmed.chars().any(|c| c.is_ascii_uppercase())
        && trimmed.chars().any(|c| c.is_ascii_lowercase())
    {
        // Already camelCase / mixed case (e.g. `workUnitId`) — verbatim.
        trimmed.to_string()
    } else {
        // Single all-upper or all-lower word (e.g. `TITLE`, `file`).
        trimmed.to_lowercase()
    }
}

/// Render the `-s, --long <value>` Commander option spec from clap's
/// `InvalidArg` token (which is the typed flag, e.g. `--description` or `-d`).
/// We look up the matching clap argument by long/short to reconstruct both
/// forms and the value placeholder.
fn commander_option_spec(typed: &str) -> String {
    let cmd = Cli::command();
    let sub_name = subcommand_token();
    let sub = sub_name
        .as_deref()
        .and_then(|n| cmd.get_subcommands().find(|c| c.get_name() == n));
    let search = sub.unwrap_or(&cmd);
    // clap's InvalidArg for an option error is `arg.to_string()`, e.g.
    // "--description <DESCRIPTION>" or "-d, --description <DESCRIPTION>".
    // Reduce it to the bare flag token (first whitespace/comma-delimited
    // piece) before matching against the declared arguments.
    let flag_token = typed.split([' ', ',']).next().unwrap_or(typed).trim();
    let flag = flag_token.trim_start_matches('-');
    for arg in search.get_arguments() {
        let matches_long = arg.get_long().map(|l| l == flag).unwrap_or(false);
        let matches_short = arg
            .get_short()
            .map(|s| s.to_string() == flag)
            .unwrap_or(false);
        if matches_long || matches_short {
            let short = arg
                .get_short()
                .map(|s| format!("-{s}, "))
                .unwrap_or_default();
            let long = arg.get_long().map(|l| format!("--{l}")).unwrap_or_default();
            let value = arg
                .get_value_names()
                .and_then(|v| v.first())
                .map(|v| format!(" <{}>", v.as_str().to_lowercase()))
                .unwrap_or_default();
            return format!("{short}{long}{value}");
        }
    }
    typed.to_string()
}

/// Build the Commander "too many arguments" message for the current
/// subcommand: `too many arguments for '<cmd>'. Expected N arguments but
/// got M.`. N = declared positional count, M = supplied positional count.
fn too_many_arguments_message() -> String {
    let cmd = Cli::command();
    let argv: Vec<String> = std::env::args().collect();
    let sub_name = subcommand_token().unwrap_or_default();
    let sub = cmd.get_subcommands().find(|c| c.get_name() == sub_name);
    let expected = sub.map(|c| c.get_positionals().count()).unwrap_or(0);
    // Supplied positionals: argv after the subcommand, excluding flags and
    // their values. This is a best-effort count that matches Commander's
    // "got M" for the common all-positional commands.
    let got = count_supplied_positionals(&argv, sub);
    // Commander pluralises the EXPECTED noun: `argument` for 1, else
    // `arguments` (`expected === 1 ? '' : 's'`). The `got` count is never
    // pluralised in Commander's template.
    let noun = if expected == 1 {
        "argument"
    } else {
        "arguments"
    };
    format!("error: too many arguments for '{sub_name}'. Expected {expected} {noun} but got {got}.")
}

/// Count positional tokens supplied after the subcommand, skipping options
/// and any value they consume.
fn count_supplied_positionals(argv: &[String], sub: Option<&clap::Command>) -> usize {
    let mut count = 0usize;
    let mut iter = argv.iter().skip(2); // program + subcommand
    while let Some(tok) = iter.next() {
        if tok == "--" {
            count += iter.count();
            break;
        }
        if tok.starts_with('-') && tok.len() > 1 {
            // Option: if it takes a value and has no '=', skip the next token.
            if !tok.contains('=') && option_takes_value(sub, tok) {
                let _ = iter.next();
            }
            continue;
        }
        count += 1;
    }
    count
}

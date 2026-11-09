#[tokio::main]
async fn main() -> std::process::ExitCode {
    // BUG-619: every rustls client in this process uses the ring provider.
    codelet_rpc_server::install_crypto_provider();
    // PROV-097: load <cwd>/.env into the process environment before any
    // clap parsing / mode dispatch, so combined::run / daemon::run reach
    // ProviderCredentials::detect() with .env-sourced keys visible.
    startup_env::load_startup_env();

    // RPC-247 / strict byte-parity: intercept `fspec <list-*> --help` before
    // clap parses, so we can emit the TS-formatted help block instead of
    // clap's auto-generated one. Returns Some(exit_code) when handled.
    if let Some(code) = intercept_ts_help() {
        return std::process::ExitCode::from(code);
    }

    let matches = match Cli::command().try_get_matches() {
        Ok(matches) => matches,
        Err(err) => return render_clap_error(err),
    };
    let cli = match <Cli as clap::FromArgMatches>::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(err) => return render_clap_error(err.format(&mut Cli::command())),
    };

    // BUG-285: `fspec --sync-version <version>` — first step of /fspec.
    if let Some(embedded) = cli.sync_version.as_deref() {
        return sync_version::exit_code(embedded).await;
    }

    // HOOK-018: every fspec command runs between its pre-<command> and
    // post-<command> hooks from spec/fspec-hooks.json.
    let Some(target) = command_hook_target(&matches) else {
        return run_cli(cli).await;
    };
    let root = match target.project_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("{err:#}");
            return std::process::ExitCode::from(1);
        }
    };
    let pre = target.run(&root, HookPhase::Pre);
    let Some(pre) = pre else {
        return std::process::ExitCode::from(1);
    };
    if pre.blocked {
        return std::process::ExitCode::from(1);
    }
    let code = run_cli(cli).await;
    if code != std::process::ExitCode::SUCCESS {
        return code;
    }
    match target.run(&root, HookPhase::Post) {
        Some(post) if !post.blocked => code,
        _ => std::process::ExitCode::from(1),
    }
}

use codelet_fspec_core::command_hooks::{
    format_hook_output, is_hooked_command, run_command_hooks, HookPhase, PhaseOutcome,
};

/// The fspec command the CLI is about to run, for its command hooks.
struct CommandHookTarget {
    command: String,
    work_unit_id: Option<String>,
}

/// The parsed subcommand when it is an fspec command (not the TUI, daemon,
/// client or other modes), with its `work_unit_id` argument when it has one.
fn command_hook_target(matches: &clap::ArgMatches) -> Option<CommandHookTarget> {
    let (command, args) = matches.subcommand()?;
    if !is_hooked_command(command) {
        return None;
    }
    let work_unit_id = args
        .try_get_one::<String>("work_unit_id")
        .ok()
        .flatten()
        .cloned();
    Some(CommandHookTarget {
        command: command.to_string(),
        work_unit_id,
    })
}

impl CommandHookTarget {
    /// The project root the command's bridge uses (init: the directory it is
    /// run in).
    fn project_root(&self) -> anyhow::Result<PathBuf> {
        if self.command == "init" {
            Ok(std::env::current_dir()?)
        } else {
            common::spec_project_root()
        }
    }

    /// Run one hook phase and print each hook's output: a failing blocking
    /// hook's to stderr, the rest to stdout. `None` (after printing the
    /// error) when the hook configuration cannot be used.
    fn run(&self, root: &std::path::Path, phase: HookPhase) -> Option<PhaseOutcome> {
        match run_command_hooks(root, &self.command, self.work_unit_id.as_deref(), phase) {
            Ok(outcome) => {
                for result in &outcome.results {
                    let formatted = format_hook_output(result);
                    if formatted.is_empty() {
                        continue;
                    }
                    if result.blocking && !result.success() {
                        eprintln!("{formatted}");
                    } else {
                        println!("{formatted}");
                    }
                }
                Some(outcome)
            }
            Err(err) => {
                eprintln!("✗ {err}");
                None
            }
        }
    }
}

/// Route the parsed command to its mode or bridge and return its exit code.
async fn run_cli(cli: Cli) -> std::process::ExitCode {
    // All list-* bridge arms share the same exit-code contract: delegate
    // to the bridge module's `run`, propagate the returned code verbatim,
    // on `Err` print the anyhow chain to stderr and exit 1.
    macro_rules! forward {
        ($bridge:path, $args:expr) => {
            match $bridge($args).await {
                Ok(code) => return std::process::ExitCode::from(code),
                Err(err) => {
                    eprintln!("{err:#}");
                    return std::process::ExitCode::from(1);
                }
            }
        };
    }

    let res = route_dispatch_01!(cli.cmd, cli.workspace, forward;);
    match res {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            std::process::ExitCode::from(1)
        }
    }
}

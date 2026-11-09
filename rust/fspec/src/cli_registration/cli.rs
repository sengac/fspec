/// `fspec` — combined frontend+server, daemon, or client.
#[derive(Parser, Debug)]
#[command(
    name = "fspec",
    version,
    about = "fspec — combined TUI + WS server (default), `daemon` (headless server), or `client` (frontend-only)",
    long_about = "The fspec binary runs in one of several modes selected by the subcommand:\n\n\
                  - (no subcommand)     combined mode: ratatui TUI + always-on WS server in one process\n\
                  - `daemon`            headless WS server only (suitable for systemd / launchd)\n\
                  - `client`            frontend-only; connects to a running daemon via WebSocket\n\
                  - `status`            one-shot health probe against the running daemon\n\n\
                  All remaining subcommands are the main fspec CLI commands. For details on any\n\
                  individual command, run `fspec <command> --help`."
)]
struct Cli {
    /// Workspace root to observe via WorkUnitsWatcher.
    /// Defaults to CWD in combined and daemon modes; ignored by client mode.
    /// NOT clap `global = true` — list-* subcommands resolve project_root
    /// from CWD and must not advertise `--workspace`.
    #[arg(long, value_name = "PATH")]
    workspace: Option<PathBuf>,

    /// Internal (BUG-285): compare the version embedded in the /fspec slash
    /// command with this binary and refresh the agent files on a mismatch.
    #[arg(long = "sync-version", value_name = "VERSION", hide = true)]
    sync_version: Option<String>,

    #[command(subcommand)]
    cmd: Option<Mode>,
}

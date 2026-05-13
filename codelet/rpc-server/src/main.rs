//! `codelet-rpc-server` binary.
//!
//! Minimal test-spawnable WebSocket daemon (RPC-005 architecture rule 12):
//!   - binds 127.0.0.1:0 (ephemeral port reported on stdout)
//!   - tracing logs to stderr
//!   - ctrl_c shutdown
//!   - hosts a single shared [`SharedFspecService`] reading from a real
//!     `WorkUnitsWatcher` over `--workspace <path>` (RPC-006).
//!
//! Hardening (configurable bind, SIGTERM, daemon mode, auth) is explicitly
//! deferred to a follow-up "productionize rpc-server" card.

use clap::Parser;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "codelet-rpc-server",
    about = "Minimal WebSocket daemon for the fspec dual-transport tarpc service",
    long_about = None,
)]
struct Cli {
    /// Workspace root to observe via the WorkUnitsWatcher (RPC-006).
    /// Defaults to the current working directory so a developer can run
    /// `codelet-rpc-server` from inside a project root with no flags.
    #[arg(long, value_name = "PATH")]
    workspace: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tracing → stderr (RPC-005 rule 12).
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let workspace = match cli.workspace {
        Some(p) => p,
        None => std::env::current_dir()?,
    };

    let watcher = Arc::new(WorkUnitsWatcher::new(&workspace)?);
    let service = Arc::new(SharedFspecService::new(watcher));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service).await?;

    // Ephemeral port to stdout — first line, plain integer, flushed.
    // Test harness reads exactly one line then keeps the child alive
    // until it kills the process.
    println!("{}", addr.port());
    use std::io::Write;
    std::io::stdout().flush()?;
    tracing::info!(workspace = %workspace.display(), addr = %addr, "rpc-server listening");

    // ctrl_c shutdown only — see deferral note above.
    tokio::signal::ctrl_c().await?;
    tracing::info!("ctrl_c received; shutting down");
    Ok(())
}

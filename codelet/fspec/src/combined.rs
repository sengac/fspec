//! Combined mode (RPC-010): always-on WS server + ratatui TUI in one
//! process.
//!
//! Sequence (per architecture note [1]):
//!   1. resolve workspace
//!   2. build the shared service → one Arc<SharedFspecService>
//!   3. start the WS server via `bind_and_serve` on loopback:0
//!   4. write daemon.json at the autodiscovery path
//!   5. print `PORT=<n>` to STDERR (stdout is the alt-screen TUI canvas)
//!   6. construct the embedded backend with Handle::current
//!   7. construct + bootstrap + run the App, racing against
//!      SIGINT/SIGTERM (rule [9] extended to combined mode)
//!   8. on exit: abort join handle → remove daemon.json
//!
//! Rule [23] teardown order: abort the WS server JoinHandle BEFORE
//! removing daemon.json so an external client observes a connection-
//! closed error rather than a hang.

use std::path::PathBuf;
use std::pin::Pin;
use std::process;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Result;
use codelet_fspec_tui::{App, EmbeddedFspecBackend};
use codelet_rpc_server::bind_and_serve;

use crate::common::{self, ShutdownReason};

pub async fn run(workspace: Option<PathBuf>) -> Result<()> {
    common::install_panic_hook();

    let workspace = common::resolve_workspace(workspace)?;
    let service = common::build_service(&workspace)?;
    common::init_tracing_combined(&service)?;

    let shutdown: Pin<Box<dyn std::future::Future<Output = Result<ShutdownReason>> + Send>> =
        Box::pin(common::build_shutdown_future());

    let (addr, _stats, join) =
        bind_and_serve("127.0.0.1:0", Arc::clone(&service)).await?;

    let djson = common::daemon_json_path()?;
    let started_at = SystemTime::now();
    if let Err(e) = common::write_daemon_json(
        &djson,
        addr.port(),
        process::id(),
        &workspace,
        started_at,
    ) {
        tracing::warn!(error = %e, "failed to write daemon.json (continuing)");
    }

    eprintln!("PORT={}", addr.port());
    use std::io::Write;
    let _ = std::io::stderr().flush();

    tracing::info!(workspace = %workspace.display(), port = addr.port(), "fspec combined mode bootstrapping");

    let backend = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service.clone());
    let mut app = App::new(Arc::new(backend));
    let run_result = match app.bootstrap().await {
        Ok(()) => drive_run(app, shutdown).await,
        Err(e) => Err(e),
    };

    join.abort();
    common::remove_daemon_json(&djson);

    run_result
}

async fn drive_run(
    app: App,
    mut shutdown: Pin<Box<dyn std::future::Future<Output = Result<ShutdownReason>> + Send>>,
) -> Result<()> {
    tokio::select! {
        r = app.run() => match r {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!(error = %e, "App::run failed; blocking on shutdown signal");
                shutdown.await.map(|_| ())
            }
        },
        sig = &mut shutdown => sig.map(|_| ()),
    }
}

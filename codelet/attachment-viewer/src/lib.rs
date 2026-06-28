//! Axum attachment viewer HTTP server (RPC-372).
//!
//! A local HTTP server that renders markdown attachments (incl. mermaid) to HTML
//! and serves other files (images/pdf) raw, scoped to a project directory. It
//! mirrors the fspec.pro axum architecture: a `build_router(_with_config)`
//! factory, a `Clone` state newtype over `Arc`, the `State` extractor, and
//! `CorsLayer::permissive()` + `TraceLayer::new_for_http()` layers.
//!
//! Browser launching and TUI key wiring are out of scope (RPC-373/RPC-374).

mod config;
mod handlers;
pub mod markdown;
mod state;

pub use config::ViewerConfig;
pub use handlers::validate_path;
pub use state::ViewerState;

use std::path::PathBuf;

use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Build the viewer router from an already-constructed [`ViewerState`].
pub fn build_router(state: ViewerState) -> Router {
    Router::new()
        .route("/view/{*path}", get(handlers::view))
        .route("/health", get(handlers::health))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Build the viewer router from a [`ViewerConfig`] (test-injectable factory).
pub fn build_router_with_config(config: ViewerConfig) -> Router {
    build_router(ViewerState::new(config))
}

/// A running viewer server: the bound port plus the shutdown channel and the
/// serving task.
pub struct ViewerHandle {
    /// The local TCP port the server is listening on.
    pub port: u16,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl ViewerHandle {
    /// Signal graceful shutdown and wait for the serving task to finish.
    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }
}

/// Start the viewer server bound to `127.0.0.1` on a random free port,
/// confined to `cwd`. Returns a [`ViewerHandle`] exposing the chosen port.
pub async fn start_viewer(cwd: PathBuf) -> anyhow::Result<ViewerHandle> {
    let app = build_router_with_config(ViewerConfig { cwd });
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let (tx, rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = rx.await;
        });
        if let Err(err) = server.await {
            tracing::error!("attachment viewer server error: {err}");
        }
    });

    Ok(ViewerHandle {
        port,
        shutdown: tx,
        task,
    })
}

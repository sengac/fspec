//! WebSocket [`FspecBackend`] implementation.
//!
//! Feature: spec/features/auto-reconnect-supervisor.feature
//! Architecture rule [21]: a SECOND constructor
//! `connect_with_supervisor(url, action_tx)` spawns a transport-layer
//! reconnect supervisor task that publishes `Action::Disconnected /
//! Reconnecting(n) / Reconnected` onto the App's action bus. The
//! existing `connect(url)` constructor stays as a no-supervisor
//! convenience for tests (RPC-005..RPC-010 signatures unchanged per
//! RPC-011 rule [16]).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use codelet_rpc_server::{ws_client_connect, FspecWsClient};
use codelet_rpc_types::{
    HealthInfo, LogRecord, SessionId, SessionInfo, StreamChunk, WorkUnitInfo,
};
use tarpc::context;
use tokio::sync::{broadcast, mpsc::UnboundedSender, Notify, RwLock};

use super::{BackendError, FspecBackend};
use crate::components::Action;

/// RPC-011 backoff schedule per rule [3] / scenario "Auto-reconnect backoff schedule":
/// 250ms → 500ms → 1s → 2s → 5s cap.
const BACKOFF_SCHEDULE_MS: &[u64] = &[250, 500, 1_000, 2_000, 5_000];

/// WebSocket-backed [`FspecBackend`].
///
/// RPC-011: the inner `FspecWsClient` is now held behind an
/// `Arc<RwLock<Option<FspecWsClient>>>` so the transport-layer
/// supervisor task can atomically swap it out on disconnect /
/// reconnect. When the slot is `None`, RPC trait methods return
/// `Err(BackendError::Disconnected)` so the UI renders the dialog
/// rather than panicking or hanging.
pub struct WebSocketFspecBackend {
    client: Arc<RwLock<Option<FspecWsClient>>>,
    /// RPC-011: manual-reconnect signal. When the user presses 'r'
    /// from the DisconnectDialog the App calls
    /// `FspecBackend::request_manual_reconnect()` which routes here
    /// and notifies the supervisor task to cancel its current backoff
    /// sleep and retry immediately (resetting the schedule).
    manual_reconnect: Arc<Notify>,
}

impl WebSocketFspecBackend {
    /// RPC-005..RPC-010 form: open a WebSocket connection to the
    /// fspec rpc-server at `url` and wrap it in a
    /// [`WebSocketFspecBackend`]. RPC-011 rule [16] preserves this
    /// signature unchanged.
    pub async fn connect(url: url::Url) -> Result<Self> {
        let (ws, _response) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .with_context(|| format!("connect_async failed for {url}"))?;
        let client = ws_client_connect(ws)
            .await
            .context("ws_client_connect failed to build FspecWsClient")?;
        Ok(Self {
            client: Arc::new(RwLock::new(Some(client))),
            manual_reconnect: Arc::new(Notify::new()),
        })
    }

    /// RPC-011 rule [21]: SECOND constructor — opens an initial
    /// WebSocket connection AND spawns a supervisor task that watches
    /// the connection for drops, attempts reconnect with exponential
    /// backoff (250ms → 500 → 1000 → 2000 → 5000 cap), and publishes
    /// `Action::Disconnected / Reconnecting(n) / Reconnected` onto the
    /// supplied `action_tx`.
    ///
    /// If the initial connect_async fails this method STILL returns
    /// Ok(Self) with a `None` client slot; the supervisor task drives
    /// the retry loop in the background so the App can render the
    /// DisconnectDialog from the very first attempt. Callers that
    /// require a successfully-established initial connection should
    /// use the legacy `connect()` constructor instead.
    pub async fn connect_with_supervisor(
        url: url::Url,
        action_tx: UnboundedSender<Action>,
    ) -> Result<Self> {
        // Best-effort initial connect — failure is fine; the supervisor
        // will retry. If it succeeds we seed the client slot so the
        // App's bootstrap RPCs can go through immediately.
        let initial_client = match tokio_tungstenite::connect_async(url.as_str()).await {
            Ok((ws, _response)) => match ws_client_connect(ws).await {
                Ok(c) => Some(c),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let manual_reconnect = Arc::new(Notify::new());
        let initial_present = initial_client.is_some();
        let client_slot: Arc<RwLock<Option<FspecWsClient>>> =
            Arc::new(RwLock::new(initial_client));

        // If the initial connect failed, surface Action::Disconnected
        // immediately so the App renders the dialog from t=0.
        if !initial_present {
            let _ = action_tx.send(Action::Disconnected);
        }

        let supervisor_url = url.clone();
        let supervisor_client = Arc::clone(&client_slot);
        let supervisor_action_tx = action_tx.clone();
        let supervisor_manual = Arc::clone(&manual_reconnect);
        tokio::spawn(async move {
            run_supervisor(
                supervisor_url,
                supervisor_client,
                supervisor_action_tx,
                supervisor_manual,
                initial_present,
            )
            .await;
        });

        Ok(Self {
            client: client_slot,
            manual_reconnect,
        })
    }

    /// RPC-011 rule [21] / [22]: alternative supervisor wiring that
    /// constructs the backend via plain `connect()` FIRST so the App
    /// can be built around it via `App::new(Arc::new(backend))` per
    /// the RPC-010 source-shape contract, THEN attaches the reconnect
    /// supervisor using an `action_tx` cloned from the App
    /// (`app.action_tx_clone()`).
    ///
    /// Returns a [`SupervisorHandle`] that holds Arc clones of the
    /// backend's internal client slot and manual-reconnect Notify.
    /// Call [`SupervisorHandle::start`] AFTER the App exists, passing
    /// `app.action_tx_clone()`, to spawn the supervisor task.
    pub fn supervisor_handle(&self) -> SupervisorHandle {
        SupervisorHandle {
            client: Arc::clone(&self.client),
            manual_reconnect: Arc::clone(&self.manual_reconnect),
        }
    }
}

/// RPC-011 rule [21] / [22] handle to a not-yet-started reconnect
/// supervisor. Returned by [`WebSocketFspecBackend::supervisor_handle`]
/// before the App exists; the App-bound `start(url, action_tx)` call
/// spawns the supervisor task on the SAME `Arc<RwLock<Option<…>>>`
/// the backend itself holds, so reconnect-driven client-slot swaps
/// are visible to RPC method bodies through the existing backend
/// reference.
pub struct SupervisorHandle {
    client: Arc<RwLock<Option<FspecWsClient>>>,
    manual_reconnect: Arc<Notify>,
}

impl SupervisorHandle {
    /// Spawn the supervisor task. Publishes `Action::Disconnected /
    /// Reconnecting(n) / Reconnected / ManualReconnect-acknowledged`
    /// emissions onto `action_tx`. The supervisor holds Arc clones
    /// of the backend's client slot — so when the supervisor swaps
    /// in a fresh `FspecWsClient` after a successful reconnect, the
    /// backend's existing RPC method bodies (which take a `read()`
    /// guard on the SAME Arc) see the new client immediately.
    pub fn start(self, url: url::Url, action_tx: UnboundedSender<Action>) {
        let SupervisorHandle {
            client,
            manual_reconnect,
        } = self;
        tokio::spawn(async move {
            run_supervisor(url, client, action_tx, manual_reconnect, true).await;
        });
    }
}

#[async_trait]
impl FspecBackend for WebSocketFspecBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_sessions(context::current()).await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .create_session(context::current(), role)
            .await?)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_input(context::current(), id, text)
            .await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        client.client().interrupt(context::current(), id).await?;
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        // Best-effort: read the client slot synchronously via try_read.
        // If the supervisor is currently rebuilding the client this
        // returns a closed receiver — the App's subscriber task observes
        // RecvError::Closed and the supervisor's Action::Reconnected
        // triggers a fresh subscribe.
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.work_units_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.chunks_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.logs_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    async fn health(&self) -> Result<HealthInfo> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or(BackendError::Disconnected)?;
        Ok(client.client().health(context::current()).await?)
    }

    /// RPC-011 rule [4]: notify the supervisor task to cancel its
    /// current backoff sleep, attempt connect immediately, and reset
    /// the backoff schedule on its next failure. Idempotent; safe to
    /// call even when the supervisor is not currently sleeping.
    fn request_manual_reconnect(&self) {
        self.manual_reconnect.notify_one();
    }
}

/// Tiny helper that returns a broadcast receiver from a freshly-created
/// channel whose sender has already been dropped, so the returned
/// receiver yields `RecvError::Closed` on first poll. Used by the
/// `*_rx` accessors when the inner client slot is `None`.
fn empty_broadcast_rx<T: Clone + Send + 'static>() -> broadcast::Receiver<T> {
    let (tx, rx) = broadcast::channel::<T>(1);
    drop(tx);
    rx
}

/// RPC-011 supervisor task per architecture note [0]: drives the
/// auto-reconnect lifecycle for a single `WebSocketFspecBackend`.
///
/// `had_initial_connection` controls the first iteration: when the
/// initial connect succeeded the supervisor waits for the connection
/// to drop BEFORE emitting Action::Disconnected; when it failed the
/// connect_with_supervisor caller already emitted Disconnected and we
/// jump straight into the backoff loop.
async fn run_supervisor(
    url: url::Url,
    client_slot: Arc<RwLock<Option<FspecWsClient>>>,
    action_tx: UnboundedSender<Action>,
    manual_reconnect: Arc<Notify>,
    had_initial_connection: bool,
) {
    if had_initial_connection {
        // Wait until the initial client's chunks_rx broadcast closes.
        let initial_chunks_rx = {
            let guard = client_slot.read().await;
            guard.as_ref().map(|c| c.chunks_rx())
        };
        if let Some(mut rx) = initial_chunks_rx {
            loop {
                match rx.recv().await {
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
        // Connection lost. Clear the slot and publish Disconnected.
        {
            let mut guard = client_slot.write().await;
            *guard = None;
        }
        if action_tx.send(Action::Disconnected).is_err() {
            return;
        }
    }

    // Reconnect loop with exponential backoff.
    let mut attempt: u32 = 0;
    loop {
        attempt = attempt.saturating_add(1);
        // Use index = min(attempt-1, last) so attempt 5..N stays at the
        // 5000ms cap per the data-table scenario.
        let idx = (attempt as usize).saturating_sub(1).min(BACKOFF_SCHEDULE_MS.len() - 1);
        let delay = Duration::from_millis(BACKOFF_SCHEDULE_MS[idx]);

        if action_tx.send(Action::Reconnecting(attempt)).is_err() {
            return;
        }

        // Sleep, but allow manual-reconnect to short-circuit.
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = manual_reconnect.notified() => {
                // Manual reconnect resets the schedule per rule [4] /
                // scenario "Pressing r ... resets backoff".
                attempt = 0;
            }
        }

        // Attempt connect_async.
        let connect_result = tokio_tungstenite::connect_async(url.as_str()).await;
        let Ok((ws, _)) = connect_result else {
            // Failed — loop and try again with longer backoff.
            continue;
        };
        let new_client = match ws_client_connect(ws).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Subscribe to chunks before swapping in the new client so we
        // don't miss the connection-alive signal.
        let mut rx = new_client.chunks_rx();
        {
            let mut guard = client_slot.write().await;
            *guard = Some(new_client);
        }

        // RPC-011 rule [5]: re-issue bootstrap on the new client. We
        // delegate this to the App by emitting Action::Reconnected —
        // the App.dispatch() handler pops the dialog and triggers a
        // fresh bootstrap via the existing RPC-009 mechanism.
        if action_tx.send(Action::Reconnected).is_err() {
            return;
        }

        // Re-arm the connection-alive watcher for the next drop.
        loop {
            match rx.recv().await {
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        // Connection dropped again — clear the slot, publish
        // Disconnected, restart the backoff loop.
        {
            let mut guard = client_slot.write().await;
            *guard = None;
        }
        if action_tx.send(Action::Disconnected).is_err() {
            return;
        }
        attempt = 0;
    }
}

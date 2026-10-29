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

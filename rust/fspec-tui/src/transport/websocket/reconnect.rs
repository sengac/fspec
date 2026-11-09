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
    had_initial_connection: bool,
    bearer: Option<Arc<str>>,
    ca: Option<Arc<codelet_rpc_server::DaemonCa>>,
) {
    if had_initial_connection {
        // Wait until the initial client's chunks_rx broadcast closes.
        let initial_chunks_rx = {
            let guard = client_slot.read().await;
            guard
                .as_ref()
                .map(codelet_rpc_server::FspecWsClient::chunks_rx)
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
        // RPC-448: the backend was dropped (the App switched daemons), so
        // this supervisor is the slot's last holder — detach, don't reconnect.
        if Arc::strong_count(&client_slot) == 1 {
            return;
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
        if Arc::strong_count(&client_slot) == 1 {
            return; // RPC-448: backend dropped — detached.
        }
        attempt = attempt.saturating_add(1);
        // Use index = min(attempt-1, last) so attempt 5..N stays at the
        // 5000ms cap per the data-table scenario.
        let idx = (attempt as usize)
            .saturating_sub(1)
            .min(BACKOFF_SCHEDULE_MS.len() - 1);
        let delay = Duration::from_millis(BACKOFF_SCHEDULE_MS[idx]);

        if action_tx.send(Action::Reconnecting(attempt)).is_err() {
            return;
        }

        tokio::time::sleep(delay).await;

        // Attempt connect_async.
        // RPC-447: re-present the bearer token on every reconnect.
        // RPC-458: and trust the same saved-daemon CA.
        let connect_result =
            codelet_rpc_server::ws_connect_trusting(url.as_str(), bearer.as_deref(), ca.as_deref())
                .await;
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

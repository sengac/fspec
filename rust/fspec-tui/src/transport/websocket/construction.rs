impl WebSocketFspecBackend {
    /// Negotiate bounded read RPCs in the WebSocket handshake, never probe
    /// an unknown tarpc variant on a legacy daemon. Legacy fallback is explicit.
    pub async fn connect_mcp(url: url::Url) -> Result<(Self, bool)> {
        let (backend, bounded, _) = Self::connect_mcp_full(url).await?;
        Ok((backend, bounded))
    }
    pub async fn connect_mcp_full(mut url: url::Url) -> Result<(Self, bool, bool)> {
        url.set_path("/fspec-read-v2");
        let (ws, response) = tokio_tungstenite::connect_async(url.as_str()).await?;
        let bounded = response
            .headers()
            .get("x-fspec-read-version")
            .is_some_and(|v| v == "2");
        let full = response
            .headers()
            .get("x-fspec-rpc-schema")
            .is_some_and(|v| v == codelet_rpc::mcp_dispatch::INTERFACE);
        let client = ws_client_connect(ws).await?;
        Ok((
            Self {
                client: Arc::new(RwLock::new(Some(client))),
                manual_reconnect: Arc::new(Notify::new()),
            },
            bounded,
            full,
        ))
    }

    /// Typed RPC dispatch without reconnect/retry; caller checks negotiated schema.
    pub async fn invoke_mcp_rpc(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> std::result::Result<serde_json::Value, codelet_rpc::mcp_dispatch::InvokeError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or_else(|| {
            codelet_rpc::mcp_dispatch::InvokeError::Transport("Disconnected".into())
        })?;
        codelet_rpc::mcp_dispatch::invoke(client.client(), name, args).await
    }

    /// Bounded response; callers must first negotiate read-v2 support.
    pub async fn read_work_units_bounded(
        &self,
        detail: bool,
        arguments: String,
    ) -> Result<std::result::Result<String, String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .read_work_units_bounded(context::current(), detail, arguments)
            .await?)
    }

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
            Ok((ws, _response)) => ws_client_connect(ws).await.ok(),
            Err(_) => None,
        };

        let manual_reconnect = Arc::new(Notify::new());
        let initial_present = initial_client.is_some();
        let client_slot: Arc<RwLock<Option<FspecWsClient>>> = Arc::new(RwLock::new(initial_client));

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

    /// RPC-434: borrow the current client's `session_events_rx()`
    /// template receiver — `None` when the client slot is
    /// disconnected (or the lock is contended) so the splitter
    /// callers degrade to closed receivers (R2).
    fn client_session_events_rx(
        &self,
    ) -> Option<broadcast::Receiver<codelet_rpc_types::SessionEvent>> {
        match self.client.try_read() {
            Ok(guard) => guard.as_ref().map(FspecWsClient::session_events_rx),
            Err(_) => None,
        }
    }
}

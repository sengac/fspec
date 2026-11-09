macro_rules! compose_websocket_oauth {
    ($($collected:tt)*) => {
        compose_websocket_finish! {
            $($collected)*

            async fn oauth_clear_tokens(&self, provider_id: String) -> Result<()> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_clear_tokens(context::current(), provider_id)
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_get_tokens(&self, provider_id: String) -> Result<bool> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_get_tokens(context::current(), provider_id)
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_headless_start(&self, provider_id: String) -> Result<OAuthHeadlessStart> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_headless_start(context::current(), provider_id)
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_headless_complete(
                &self,
                provider_id: String,
                code_with_state: String,
                pkce_verifier: String,
            ) -> Result<()> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_headless_complete(
                        context::current(), provider_id, code_with_state, pkce_verifier,
                    )
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_device_start(&self, provider_id: String) -> Result<OAuthDeviceStart> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_device_start(super::oauth_context::interactive(), provider_id)
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_device_poll(
                &self,
                provider_id: String,
                device_auth_id: String,
                interval: u64,
            ) -> Result<()> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_device_poll(
                        super::oauth_context::interactive(), provider_id, device_auth_id, interval,
                    )
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn oauth_copilot_device_start(
                &self,
                enterprise_host: Option<String>,
            ) -> Result<OAuthDeviceStart> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client.client()
                    .oauth_copilot_device_start(context::current(), enterprise_host)
                    .await?
                    .map_err(anyhow::Error::msg)
            }

            async fn session_card_changes(
                &self,
                session_id: SessionId,
            ) -> Result<Vec<codelet_rpc_types::SessionCardChange>> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client
                    .client()
                    .session_card_changes(context::current(), session_id)
                    .await?
                    .map_err(|e| anyhow::anyhow!("{e}"))
            }

            async fn move_session_cards_back(
                &self,
                session_id: SessionId,
            ) -> Result<Vec<codelet_rpc_types::CardMoveBack>> {
                let guard = self.client.read().await;
                let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
                client
                    .client()
                    .move_session_cards_back(context::current(), session_id)
                    .await?
                    .map_err(|e| anyhow::anyhow!("{e}"))
            }
        }
    };
}

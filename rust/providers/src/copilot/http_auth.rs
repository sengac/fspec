use super::CopilotHttpClient;
use crate::copilot::{token_refresh, CopilotAuthJson};
use http::Request;
use std::sync::Arc;
use tokio::sync::RwLock;

impl CopilotHttpClient {
    pub(crate) fn with_auth(
        mut self,
        auth: Arc<RwLock<CopilotAuthJson>>,
        initial_base: String,
    ) -> Self {
        self.auth = Some(auth);
        self.initial_base = Some(initial_base);
        self
    }

    pub(crate) async fn live_binding(
        &self,
        model: &str,
        deployment: &str,
    ) -> Result<String, rig::completion::CompletionError> {
        if let Some(auth) = &self.auth {
            token_refresh::ensure_fresh_copilot_token(auth)
                .await
                .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
            let state = auth.read().await;
            return Ok(self.binding_for(Some(&state), model, deployment)?);
        }
        Ok(self.binding_for(None, model, deployment)?)
    }

    fn binding_for(
        &self,
        state: Option<&CopilotAuthJson>,
        model: &str,
        deployment: &str,
    ) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let identity = if let Some(state) = state {
            serde_json::to_vec(&(
                &state.github_oauth_token,
                &state.enterprise_url,
                deployment,
                state
                    .endpoints_api
                    .as_deref()
                    .filter(|origin| !origin.is_empty())
                    .or(self.initial_base.as_deref()),
                model,
            ))?
        } else {
            serde_json::to_vec(&(
                self.access_token.as_ref(),
                deployment,
                &self.initial_base,
                model,
            ))?
        };
        Ok(format!("{:x}", Sha256::digest(identity)))
    }

    fn capture_binding(&self, state: Option<&CopilotAuthJson>) -> rig::http_client::Result<()> {
        if let Some(capture) = &self.capture {
            let (model, deployment) = capture.identity();
            let binding = self
                .binding_for(state, model, deployment)
                .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
            capture.bind_request(binding)?;
        }
        Ok(())
    }

    pub(super) async fn fresh_request<T>(
        &self,
        mut request: Request<T>,
    ) -> rig::http_client::Result<(Request<T>, String)> {
        let Some(auth) = &self.auth else {
            self.capture_binding(None)?;
            return Ok((request, self.access_token.to_string()));
        };
        token_refresh::ensure_fresh_copilot_token(auth)
            .await
            .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
        let state = auth.read().await;
        self.capture_binding(Some(&state))?;
        if let (Some(initial), Some(current)) = (&self.initial_base, &state.endpoints_api) {
            if !current.is_empty() {
                if let Some(suffix) = request
                    .uri()
                    .to_string()
                    .strip_prefix(initial.trim_end_matches('/'))
                {
                    *request.uri_mut() = format!("{}{suffix}", current.trim_end_matches('/'))
                        .parse()
                        .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
                }
            }
        }
        Ok((request, state.copilot_token.clone().unwrap_or_default()))
    }
}

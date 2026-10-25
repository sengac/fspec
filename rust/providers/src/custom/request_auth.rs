use super::RhaiCustomProvider;
use crate::{
    custom::AuthConfig,
    oauth::script_provider::{ScriptProviderConfig, ScriptedOAuthProvider},
    ProviderError,
};

impl RhaiCustomProvider {
    pub(crate) async fn ensure_request_auth(&self) -> Result<(), ProviderError> {
        let (token_url, client_id, scopes) = match &self.config.auth {
            AuthConfig::OauthPkce {
                token_url,
                client_id,
                scopes,
                ..
            }
            | AuthConfig::OauthDeviceCode {
                token_url,
                client_id,
                scopes,
                ..
            } => (token_url, client_id, scopes),
            _ => return Ok(()),
        };
        let has = |names: &[&str]| {
            self.ast
                .iter_functions()
                .any(|function| names.contains(&function.name))
        };
        if !has(&["auth_needs_refresh", "needs_refresh"])
            || !has(&["auth_refresh", "refresh_token"])
        {
            return Ok(());
        }
        let config = ScriptProviderConfig {
            name: self.config.name.clone(),
            display_name: self.config.display_name.clone(),
            script: self.config.script.clone(),
            auth_url: Some(self.config.base_url.clone()),
            token_url: Some(token_url.clone()),
            client_id: Some(client_id.clone()),
            scopes: scopes.clone(),
            redirect_uri: None,
            flow: None,
            credential_file: None,
        };
        let provider = ScriptedOAuthProvider::from_compiled(
            self.engine.clone(),
            self.ast.as_ref().clone(),
            config,
        );
        crate::oauth::scripted_request_refresh::ensure(provider)
            .await
            .map_err(|_| {
                ProviderError::api(
                    self.config_name(),
                    "scripted OAuth refresh failed before request",
                )
            })
    }
}

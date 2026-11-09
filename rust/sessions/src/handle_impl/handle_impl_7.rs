macro_rules! compose_handle_impl_7 {
    ($($collected:tt)*) => {
        compose_handle_impl_8! {
            $($collected)*
    // CONT-003: goal chrome state — delegates to the BackgroundSession's
    // goal Mutex; the agent loop syncs it into the inner session before
    // each dispatched user message.
    fn set_goal_state(
        &self,
        session_id: &SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                session.set_goal_state(goal);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    fn get_goal_state(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<(String, Option<String>)>, String> {
        Ok(self.get_session(&session_id.value)?.get_goal_state())
    }
    fn get_role(&self, session_id: &SessionId) -> Result<Option<String>, String> {
        Ok(self.get_session(&session_id.value)?.get_role())
    }
    fn set_role(&self, session_id: &SessionId, role: Option<String>) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                match role {
                    Some(r) => session.set_role(r),
                    None => session.clear_role(),
                }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    // ========================================================================
    // RPC-054: Provider credentials surface — delegate to `codelet-providers`.
    // The provider helpers there are already NAPI-free so we can call them
    // directly without dragging any NAPI dependency into codelet-sessions.
    //
    // Scope per the RPC-054 attachment:
    //   * `list_provider_credentials` / `get_provider_credential` reflect
    //     `ProviderCredentials::detect()` + `custom::list_providers_info()`
    //     into the wire-portable `ProviderCredentialInfo` shape.
    //   * `set_provider_credentials` (api_key kind) PERSISTS to
    //     `<data_dir>/credentials/credentials.json` via
    //     `credentials::save_credential` and refreshes the in-memory store
    //     (RPC-054 reopened — mirrors TS saveCredential). oauth / custom
    //     kinds are validated but remain non-persistent (follow-up).
    //   * `delete_provider_credentials` removes the provider entry via
    //     `credentials::delete_credential` (mirrors TS deleteCredential);
    //     absent provider / missing file is a no-op success.
    //   * `test_provider_connection` delegates to
    //     `codelet_providers::custom::test_provider_connection` for custom
    //     providers and falls back to `ProviderManager::with_provider`
    //     (CONFIG-004 pattern) for built-ins.
    //   * `refresh_models_cache` delegates to the same custom-provider
    //     helper because the built-ins have static model lists.
    // ========================================================================

    fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>, String> {
        // BUG-465: a provider listing failure is returned, not an empty list.
        let list = codelet_providers::custom::list_providers_info()
            .map_err(|error| format!("the provider credentials could not be read: {error}"))?;
        Ok(list
                .into_iter()
                .map(|p| ProviderCredentialInfo {
                    provider_id: p.name.clone(),
                    display_name: p.display_name.unwrap_or_else(|| p.name.clone()),
                    configured: p.available,
                    credential_type: if p.is_custom {
                        "custom".to_string()
                    } else {
                        // PROV-053 / RPC-107: codex, github-copilot, and
                        // anthropic use OAuth flows; everything else uses
                        // an env-var API key. The "anthropic" slug
                        // replaces the legacy Rust-internal "claude" slug
                        // as part of the RPC-107 canonical-catalog port.
                        match p.name.as_str() {
                            "anthropic" | "codex" | "github-copilot" => "oauth".to_string(),
                            _ => "api_key".to_string(),
                        }
                    },
                    model_count: p.models.len() as u32,
                    // RPC-108: propagate masked credential + source
                    // verbatim from codelet-providers. Masking has
                    // already happened server-side via
                    // `credentials::mask_api_key` before this mapper
                    // runs, so raw key bytes never traverse the wire.
                    masked_key: p.masked_key,
                    source: p.source,
                })
                .collect())
    }
    fn get_provider_credential(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderCredentialInfo>, String> {
        // BUG-466: a failed provider listing is the error, not "unknown".
        Ok(self
            .list_provider_credentials()?
            .into_iter()
            .find(|p| p.provider_id == provider_id))
    }
    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        // RPC-054 (reopened): the api_key write path is no longer a stub —
        // it persists to <data_dir>/credentials/credentials.json and refreshes
        // the in-memory store, mirroring TS saveCredential. oauth / custom
        // kinds remain non-persistent here (OAuth write path is a follow-up).
        match creds.kind.as_str() {
            "api_key" => {
                let api_key = creds.api_key.as_deref().unwrap_or("");
                if api_key.is_empty() {
                    return Err("api_key input requires a non-empty api_key".to_string());
                }
                crate::credentials::save_credential(provider_id, api_key)?;
                tracing::info!(
                    provider = provider_id,
                    kind = "api_key",
                    "set_provider_credentials: api key persisted to credentials.json"
                );
                Ok(())
            }
            "oauth" => {
                if creds.oauth_token.as_deref().unwrap_or("").is_empty() {
                    return Err("oauth input requires a non-empty oauth_token".to_string());
                }
                tracing::info!(
                    provider = provider_id,
                    kind = "oauth",
                    "set_provider_credentials: oauth persistence is a follow-up; input accepted"
                );
                Ok(())
            }
            "custom" => {
                if creds.custom_endpoint.as_deref().unwrap_or("").is_empty() {
                    return Err("custom input requires a non-empty custom_endpoint".to_string());
                }
                tracing::info!(
                    provider = provider_id,
                    kind = "custom",
                    "set_provider_credentials: custom persistence is a follow-up; input accepted"
                );
                Ok(())
            }
            other => Err(format!("unknown credential kind: {other}")),
        }
    }
    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        // PROV-133: pressing 'd' in provider settings must actually remove the
        // credential across EVERY source the availability projection reads.
        // `ProviderCredentials::detect()` derives `configured` from ENV VARS +
        // OAuth AUTH FILES (never credentials.json), so an authoritative delete
        // (Option A) must clear all three sources for the target provider.
        //
        // 1. credentials.json entry (absent provider / missing file = no-op).
        crate::credentials::delete_credential(provider_id)?;

        // 2. Process env vars for ONLY this provider (env source).
        crate::credentials::remove_provider_env_vars(provider_id);

        // 3. OAuth auth file for the three OAuth providers (auth-file source).
        //    Sync file removals so this sync bridge never needs a tokio runtime;
        //    a missing file stays a no-op success. Copilot uses the sync twin
        //    `delete_copilot_auth_sync` for the same reason.
        match provider_id {
            "anthropic" => {
                codelet_providers::claude_auth::delete_claude_auth()
                    .map_err(|e| format!("failed to delete claude auth file: {e}"))?;
            }
            "codex" => {
                codelet_providers::codex::codex_auth::clear_codex_oauth()
                    .map_err(|e| format!("failed to delete codex auth file: {e}"))?;
            }
            "github-copilot" => {
                codelet_providers::copilot::delete_copilot_auth_sync()
                    .map_err(|e| format!("failed to delete copilot auth file: {e}"))?;
            }
            _ => {}
        }

        tracing::info!(
            provider = provider_id,
            "delete_provider_credentials: credential removed from credentials.json, env vars, and auth file"
        );
        Ok(())
    }
    fn test_provider_connection(&self, provider_id: &str) -> Result<TestConnectionResult, String> {
        let start = std::time::Instant::now();
        // Try the custom-provider HTTP probe first — it's the only path
        // that returns rich `ProviderTestResult` metadata. If the
        // provider id is unknown to the custom-provider helper, fall
        // back to `ProviderManager::with_provider` which validates
        // credentials for the built-ins.
        //
        // RPC-070: pre-fix this method built its own `tokio::runtime`
        // via `Handle::try_current()` + `runtime.block_on(...)`, which
        // panics under a nested runtime context (the live tarpc
        // dispatcher). Replace with the canonical
        // `block_in_place(|| Handle::current().block_on(...))` pattern
        // so this bridge is safe from inside any multi-thread tokio
        // worker.
        let provider_id_owned = provider_id.to_string();
        let custom_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                codelet_providers::custom::test_provider_connection(&provider_id_owned).await
            })
        });
        match custom_result {
            Ok(probe) => Ok(TestConnectionResult {
                success: probe.reachable,
                error: if probe.reachable {
                    None
                } else {
                    Some(format!("unreachable (status {:?})", probe.status_code))
                },
                latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
            }),
            Err(_) => {
                // Fall back to credential validation for built-in providers.
                match codelet_providers::ProviderManager::with_provider(provider_id) {
                    Ok(_) => Ok(TestConnectionResult {
                        success: true,
                        error: None,
                        latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                    }),
                    Err(e) => Ok(TestConnectionResult {
                        success: false,
                        error: Some(e.to_string()),
                        latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                    }),
                }
            }
        }
    }
        }
    };
}

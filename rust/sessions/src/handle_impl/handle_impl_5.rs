macro_rules! compose_handle_impl_5 {
    ($($collected:tt)*) => {
        compose_handle_impl_6! {
            $($collected)*
    fn list_providers(&self) -> Result<Vec<codelet_rpc_types::ProviderInfo>, String> {
        // RPC-073: wire the providers registry into the /model picker.
        // Mirrors the TS Ink path (NAPI `list_providers` →
        // `codelet_providers::custom::list_providers_info`) so the
        // Rust ratatui ModelSelectorDialog sees the same provider tree
        // as the TS Ink ModelSelectorView.
        //
        // The tarpc wire format `codelet_rpc_types::ProviderInfo` is
        // lossy vs. the 9-field `codelet_providers::custom::ProviderInfo`
        // (no facade / base_url / api_key_env_var / api_style /
        // is_custom / available — the latter is exposed via the
        // sibling `list_provider_credentials` for the /provider view).
        // We map field-by-field here:
        //   * `name` → `key` (canonical provider slug used by
        //     `set_session_model`).
        //   * `display_name.unwrap_or(name)` → `display_name` (matches
        //     the existing precedent at handle_impl.rs:803 for
        //     `list_provider_credentials`).
        //   * Each `ProviderModelInfo` → `ModelEntry` with
        //     `supports_thinking` → `supports_reasoning`, `is_custom`
        //     propagated from parent, `usize` → `u32` saturating.
        // Built-in providers carry empty `models`; custom providers
        // surface their declared models.
        //
        // RPC-451: a failed listing (e.g. a corrupt
        // `~/.fspec/providers/*.json`) is returned to the caller.
        match codelet_providers::custom::list_providers_info() {
            Ok(list) => {
                // RPC-073 (reopened): source the models.dev catalog ONCE so
                // built-in/cloud providers expose their tool-capable models
                // (credential-gated) instead of empty rows. Mirrors TS
                // `loadCloudModels` + `buildCloudSections`. Graceful: any load
                // failure (cold cache offline, corrupt file) → None → the
                // built-ins keep their empty model lists (prior behaviour).
                let cloud_registry = build_cloud_registry();
                // PROV-130: partition the mapped sections into the cloud
                // (built-in / canonical) bucket and the custom-provider bucket
                // by the SOURCE `p.is_custom` flag. The wire `ProviderInfo`
                // carries no section-level `is_custom`, so the split must happen
                // here at the map, before it is erased. Mirrors the TS
                // `cloudSections` vs `customSections` distinction consumed by
                // `modelInitializationService.ts`.
                let mut cloud_sections: Vec<codelet_rpc_types::ProviderInfo> = Vec::new();
                let mut custom_sections: Vec<codelet_rpc_types::ProviderInfo> = Vec::new();
                for p in list {
                    let is_custom = p.is_custom;
                    let display_name = p.display_name.unwrap_or_else(|| p.name.clone());
                    let mut models: Vec<codelet_rpc_types::ModelEntry> = p
                        .models
                        .into_iter()
                        .map(|m| codelet_rpc_types::ModelEntry {
                            id: m.id.clone(),
                            display_name: m.id,
                            // PROV-170: an undeclared window stays unavailable;
                            // PROV-172: list_providers_info already rejected a value the u32
                            // wire cannot carry.
                            context_window: m.context_window,
                            supports_reasoning: m.supports_thinking,
                            supports_vision: m.supports_vision,
                            is_custom,
                        })
                        .collect();
                    // RPC-073: built-in (non-custom) providers carry no
                    // declared models — fill them from the models.dev
                    // registry, gated on configured credentials.
                    //
                    // PROV-146: the `openai` provider is EXCLUDED from cloud
                    // catalog population. It is exclusively for local
                    // OpenAI-protocol-compatible servers (vLLM, Ollama,
                    // sglang, RunPod, Fireworks) configured via profiles.
                    // PROV-176: cloud OpenAI models are offered only under the
                    // `Codex (ChatGPT)` provider, as Codex /models lists them
                    // (`synthesize_codex_section`); the models.dev OpenAI
                    // catalog is not listed (product owner, 2026-11-06). Without this exclusion, a profile's apiKey
                    // bridged into `OPENAI_API_KEY` (PROV-121) makes
                    // `provider_has_credentials("openai")` true and the
                    // full cloud catalog leaks into the OpenAI API section.
                    if !is_custom && models.is_empty() && p.name != "openai" {
                        if let Some(registry) = cloud_registry.as_ref() {
                            let has_creds = crate::cloud_models::provider_has_credentials(&p.name);
                            models = crate::cloud_models::cloud_model_entries(
                                registry, &p.name, has_creds,
                            );
                        }
                    }
                    // RPC-338: cloud / custom providers are never profile
                    // sections and are always treated as reachable.
                    let info = crate::profile_sections::cloud_provider_info(
                        &p.name,
                        &display_name,
                        models,
                    );
                    if is_custom {
                        custom_sections.push(info);
                    } else {
                        cloud_sections.push(info);
                    }
                }
                // PROV-176: the "Codex (ChatGPT)" section lists the models
                // Codex /models returns, fetched every time the list is built
                // (models.dev only supplies a listed model's context window).
                // It LEADS the cloud group (PROV-130) and is built BEFORE the
                // PROV-127 drop-empty filter, which keeps it when it carries
                // the reason it lists nothing.
                let cloud_sections = crate::cloud_models::synthesize_codex_section(
                    cloud_sections,
                    cloud_registry.as_ref(),
                );
                // PROV-127: drop cloud/custom sections whose model list is empty
                // so uncredentialed/known-absent providers do not render as dead
                // "Provider (0 models)" headers. Mirrors TS
                // `cloudSectionBuilder.ts` (`filter(s => s.hasCredentials)`) +
                // `modelInitializationService.ts` (`filter(s => s.models.length
                // > 0)`). Local profile sections (appended below) may legitimately
                // have zero models when unreachable (RPC-338/MODEL-004) and are
                // never subject to this filter.
                let cloud_sections =
                    crate::profile_sections::retain_populated_cloud_sections(cloud_sections);
                let custom_sections =
                    crate::profile_sections::retain_populated_cloud_sections(custom_sections);
                // PROV-130: assemble the final DISPLAY order to match TS
                // `modelInitializationService.ts:196-200`
                // `[...profileSections, ...customSections, ...cloudSections]` —
                // local-server profiles FIRST, then custom providers, then the
                // cloud group. Because the first section-with-models is the
                // auto-default, this makes the Rust default match TS.
                //
                // RPC-338: local-server profile sections probe each profile's
                // `/v1/models` endpoint to compute reachability (MODEL-004: a
                // profile with custom models is never marked unreachable).
                // Mirrors TS `loadProfileSections`.
                // PROV-175: an invalid profile config fails the listing.
                let mut providers = crate::profile_sections::build_local_profile_sections()
                    .map_err(|error| format!("the provider list could not be read: {error}"))?;
                providers.extend(custom_sections);
                providers.extend(cloud_sections);
                providers.extend(crate::profile_sections::build_inherited_profile_sections(
                    &providers,
                ));
                Ok(providers)
            }
            Err(e) => Err(format!("the provider list could not be read: {e}")),
        }
    }
    /// PROV-118: delegate to the inherent `SessionManager::set_default_model`
    /// (BUG-411: an empty model or a failed save is returned).
    fn set_default_model(&self, model: &str) -> Result<(), String> {
        SessionManager::set_default_model(self, model)
    }
    /// RPC-440: delegate to the inherent `SessionManager::get_default_model`.
    fn get_default_model(&self) -> Option<String> {
        SessionManager::get_default_model(self)
    }
    /// RIG-016: steer, queue or interrupt through the live session.
    fn submit_input(
        &self,
        session_id: &SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<(), String> {
        self.live_session(session_id)?.submit_input(text, None, mode)
    }
    /// MCP-003: connect a daemon client's MCP servers to an idle live session.
    fn configure_session_mcp_servers(
        &self,
        session_id: &SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<codelet_rpc_types::SessionMcpServerTools>, String>,
                > + Send,
        >,
    > {
        let session = self.live_session(session_id);
        Box::pin(async move { crate::session_mcp::configure(session?, servers).await })
    }
    /// TOOL-027: interrupt or end the input of a live exec session's
    /// command. Bridges to the async unified_exec store via
    /// `loop_block_on` (multi-thread runtime, like `write_exec_stdin`). A
    /// success answers the prompt, so the stored exec-stdin request is
    /// cleared (BUG-171 parity with `write_exec_stdin`).
    fn control_exec_stdin(
        &self,
        session_id: &SessionId,
        exec_session_id: &str,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<(), String> {
        let session = self
            .get_session(&session_id.value)?;
        let control = match control {
            codelet_rpc_types::ExecStdinControl::Interrupt => {
                codelet_tools::unified_exec::StdinControl::Interrupt
            }
            codelet_rpc_types::ExecStdinControl::Eof => codelet_tools::unified_exec::StdinControl::Eof,
        };
        let exec_session_id = exec_session_id.to_string();
        loop_block_on(async move {
            codelet_tools::unified_exec::global_store()
                .control_stdin(&exec_session_id, control)
                .await
        })?;
        session.set_exec_stdin_request(None)
    }
        }
    };
}

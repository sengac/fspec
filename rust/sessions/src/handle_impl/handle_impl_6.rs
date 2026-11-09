macro_rules! compose_handle_impl_6 {
    ($($collected:tt)*) => {
        compose_handle_impl_7! {
            $($collected)*
    fn set_model(
        &self,
        session_id: &SessionId,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let session = match self.get_session(&session_id.value) {
            Ok(session) => session,
            Err(error) => return Err(error),
        };
        let uuid = parse_session_uuid(session_id)?;

        // RPC-343: re-resolve the new model server-side instead of only
        // swapping the cosmetic label strings. Mirror creation-time credential
        // resolution for the (possibly new) provider, apply the selection to
        // the inner request-issuing provider manager via the shared resolver,
        // then recompute and cache the model limits. On any failure we return
        // Err BEFORE mutating session state, so a bad model leaves the previous
        // model and its cached limits intact.
        let model = format!("{provider_id}/{model_id}");
        let project_path = std::path::PathBuf::from(&session.project);

        // session.inner is a tokio::sync::Mutex. This trait method is sync and
        // is invoked from the (idle) UI dispatch path, so take the lock without
        // blocking: try_lock succeeds for an idle session and avoids both a
        // sync→async bridge and the risk of deadlocking a streaming turn. If
        // the session is mid-stream the switch is declined (state unchanged).
        let resolved = {
            let mut inner = session
                .inner
                .try_lock()
                .map_err(|_| "Session is busy; cannot switch model right now".to_string())?;
            crate::model_switch::validate_selection(uuid, &model)?;
            let environment = crate::model_switch::SelectionEnvironment::capture(provider_id);
            // BUG-411: switching to a provider whose credentials cannot be
            // resolved fails before the session state changes.
            crate::credentials::resolve_and_set_env_var(provider_id, Some(project_path.as_path()))
                .map_err(|error| {
                    format!("cannot resolve credentials for provider {provider_id}: {error}")
                })?;
            let (next_manager, resolved) = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(crate::model_resolution::resolve_provider_manager_and_limits(&model))
            })?;
            crate::model_switch::persist_selection(&mut inner, uuid, &model)?;
            environment.commit();
            *inner.provider_manager_mut() = next_manager;
            inner.apply_provider_runtime_settings();
            session.inherit_continue_state(inner.continue_enabled, inner.continue_budget);
            (inner.continue_enabled, inner.continue_budget) = session.get_continue_state();
            // MODEL-011: the new model's thinking target, for checking levels.
            session.set_thinking_target(
                codelet_providers::thinking_wire::ThinkingTarget::for_provider_manager(
                    inner.provider_manager(),
                ),
            );
            let pm = inner.provider_manager_mut();
            // BUG-168: update the tool-layer capability registry on every
            // mid-session model switch so the Read tool's PDF default mode
            // follows the new model.
            codelet_tools::model_capabilities::set_session_model_vision(
                uuid,
                crate::model_resolution::resolve_model_vision(pm)?,
            );
            // PROV-144: update the per-profile image budget alongside the
            // vision entry (absent => None => default 4), sourced from the
            // shared resolver so a mid-session switch cannot drift.
            codelet_tools::model_capabilities::set_session_model_max_images(
                uuid,
                crate::model_resolution::resolve_profile_max_images(pm),
            );
            resolved
        };

        session.set_model(Some(provider_id.to_string()), Some(model_id.to_string()));
        session.set_model_limits(resolved);
        // MODEL-012: the new model's saved level (product owner, 2026-11-05):
        // applied when offered, otherwise not set (with a notice when one is
        // saved but not offered or unreadable).
        session.apply_saved_thinking_level();

        // PROV-123: keep the global default in sync so a NEW session created in
        // this same process inherits the just-selected model (TS-parity with
        // modelSelectionService "keeps store in sync for new sessions").
        // `set_default_model` updates the in-memory `default_model` RwLock that
        // `create_session` / `create_isolated_session` read AND persists
        // `default-model.json` + `tui.lastUsedModel` (PROV-119/122), so it
        // SUPERSEDES the standalone PROV-122 `save_persisted_model_string` call.
        // `model` is the composite `provider_id/model_id` — for profile
        // selections `provider_id` is the profile-qualified key (e.g.
        // `openai:qwen`), so the stored default round-trips to a form a new
        // session resolves identically. BUG-411: a failed save is returned.
        SessionManager::set_default_model(self, &model)
    }
    // RPC-347: custom-model write surface. These delegate to the RPC-346
    // persistence functions in `profile_sections`, converting the
    // transport-portable `CustomModelDefinition` into the on-disk
    // `CustomModelDef` via `conversions::custom_model_def_from_wire`. The
    // openai-only guard and every refusal (missing config / profile / model,
    // BUG-325) live in `profile_sections`, so these are thin pass-throughs.
    fn add_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &codelet_rpc_types::CustomModelDefinition,
    ) -> Result<(), String> {
        let def = crate::conversions::custom_model_def_from_wire(definition);
        crate::profile_sections::save_custom_model(provider_id, profile_name, &def, None)
            .map_err(|e| e.to_string())
    }
    fn update_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        original_model_id: &str,
        definition: &codelet_rpc_types::CustomModelDefinition,
    ) -> Result<(), String> {
        let def = crate::conversions::custom_model_def_from_wire(definition);
        crate::profile_sections::save_custom_model(
            provider_id,
            profile_name,
            &def,
            Some(original_model_id),
        )
        .map_err(|e| e.to_string())
    }
    fn delete_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        model_id: &str,
    ) -> Result<(), String> {
        crate::profile_sections::delete_custom_model(provider_id, profile_name, model_id)
            .map_err(|e| e.to_string())?;
        // BUG-613: a deleted default model is no default.
        let deleted = format!("{provider_id}:{profile_name}/{model_id}");
        self.clear_default_model_if(|model| model == deleted)?;
        Ok(())
    }
    // PROV-108: profile write surface. Delegates to the `profile_persistence`
    // read-modify-write (preserving customModels + sibling keys), converting
    // the transport-portable `ProfileDefinition` via
    // `conversions::profile_def_from_wire`.
    fn list_provider_settings(
        &self,
    ) -> Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, String> {
        crate::provider_settings::list_provider_settings().map_err(|error| error.to_string())
    }
    fn save_provider_settings(
        &self,
        provider_id: &str,
        definition: &codelet_rpc_types::ProfileDefinition,
    ) -> Result<(), String> {
        crate::provider_settings::save_provider_settings(provider_id, definition)
            .map_err(|error| error.to_string())
    }
    fn save_profile(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &codelet_rpc_types::ProfileDefinition,
    ) -> Result<(), String> {
        if !crate::profile_persistence::profiles_supported(provider_id) {
            return Err(crate::profile_persistence::profiles_unsupported_error(
                provider_id,
            ));
        }
        crate::provider_settings::validate_definition(definition)
            .map_err(|error| error.to_string())?;
        crate::provider_settings::validate_definition(definition)
            .map_err(|error| error.to_string())?;
        let def = crate::conversions::profile_def_from_wire(definition);
        crate::profile_persistence::save_profile(provider_id, profile_name, &def)
            .map_err(|e| e.to_string())
    }
    fn delete_profile(&self, provider_id: &str, profile_name: &str) -> Result<(), String> {
        crate::profile_persistence::delete_profile(provider_id, profile_name)
            .map_err(|e| e.to_string())?;
        // BUG-612: the environment this profile wrote goes with it.
        crate::profile_environment::ledger::forget_owner(&crate::profile_environment::ledger::owner(
            provider_id,
            profile_name,
        ));
        // BUG-613: a default model of the deleted profile is no default.
        let prefix = format!("{provider_id}:{profile_name}/");
        self.clear_default_model_if(|model| model.starts_with(&prefix))?;
        Ok(())
    }
    fn rename_profile(
        &self,
        provider_id: &str,
        old_name: &str,
        new_name: &str,
        definition: &codelet_rpc_types::ProfileDefinition,
    ) -> Result<(), String> {
        if !crate::profile_persistence::profiles_supported(provider_id) {
            return Err(crate::profile_persistence::profiles_unsupported_error(
                provider_id,
            ));
        }
        let def = crate::conversions::profile_def_from_wire(definition);
        crate::profile_persistence::rename_profile(provider_id, old_name, new_name, &def)
            .map_err(|e| e.to_string())
    }
    fn set_thinking_level(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        // MODEL-011: only a level the session's model offers is stored.
        let session = self.get_session(&session_id.value)?;
        crate::thinking_options::accepted(&session.thinking_target()?, &level)?;
        session.set_base_thinking_level(level);
        Ok(())
    }
    // CONT-002: auto-continue state — delegates to the BackgroundSession's
    // atomic chrome fields; the agent loop syncs them into the inner
    // session before each dispatched user message.
    fn set_continue_state(
        &self,
        session_id: &SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                session.set_continue_state(enabled, budget);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    fn get_continue_state(&self, session_id: &SessionId) -> Result<(bool, u32), String> {
        Ok(self.get_session(&session_id.value)?.get_continue_state())
    }
        }
    };
}

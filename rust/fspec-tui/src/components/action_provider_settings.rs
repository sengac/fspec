macro_rules! compose_action_provider_settings {
    ($($collected:tt)*) => {
        compose_action_slash_operations! {
            $($collected)*
            // ========================================================================
            // RPC-054: Provider settings view actions. The `/provider` slash
            // command dispatches `OpenProviderSettingsView`; the view itself
            // emits `SaveProviderCredentials` / `TestProviderConnection` /
            // `RefreshProviderModels` / `DeleteProviderCredentials`; the App
            // folds backend responses into the view via the *Loaded /
            // *Complete / *Refreshed variants.
            // ========================================================================
            /// RPC-054: emitted by the slash command palette (and `/provider`
            /// submit-line parser) to open the ProviderSettingsView. App::dispatch
            /// flips the Navigator's ViewMode to ProviderSettings AND spawns
            /// `backend.list_provider_credentials()`.
            OpenProviderSettingsView,
            /// RPC-054: emitted by the view on Esc (list mode). App::dispatch
            /// returns the Navigator to the Agent view.
            CloseProviderSettingsView,
            /// RPC-337: open the full-screen ModelSelector mode-view. Emitted
            /// by the `/model` slash command AND by ProviderSettings Tab
            /// (SwitchToModels). App::dispatch flips the Navigator's ViewMode to
            /// ModelSelector AND spawns `backend.list_providers()` (folded back
            /// via `Action::ListProvidersLoaded`). This is the only model-picker
            /// entry point (no Compositor modal exists).
            OpenModelSelectorView,
            /// RPC-337: emitted by the ModelSelector mode-view on Esc.
            /// App::dispatch returns the Navigator to the Agent view. A
            /// committed `Action::ModelSelected` ALSO returns to Agent (the
            /// selection closes the selector — handled in Navigator::apply_action).
            CloseModelSelectorView,
            /// RPC-337: emitted by the ModelSelector mode-view on `r`. Re-spawns
            /// `backend.list_providers()` so the provider/model tree refreshes;
            /// the view shows `(refreshing...)` until `ListProvidersLoaded`
            /// arrives.
            RefreshModelSelector,
            /// RPC-347: emitted by the ModelSelector mode-view's `a` keybind (wired in
            /// RPC-344) to create a NEW custom model on the focused profile.
            /// App::dispatch spawns `backend.add_custom_model(provider_id,
            /// profile_name, definition)` followed by a `list_providers` refresh.
            /// Added here as inert wire surface so the backend path exists before the
            /// keybinds/form land.
            AddCustomModel {
                provider_id: String,
                profile_name: String,
                definition: codelet_rpc_types::CustomModelDefinition,
            },
            /// RPC-347: emitted by the ModelSelector `e` keybind (RPC-344) to edit an
            /// existing custom model in place. `original_model_id` names the entry
            /// being replaced. App::dispatch spawns `backend.update_custom_model(..)`.
            EditCustomModel {
                provider_id: String,
                profile_name: String,
                original_model_id: String,
                definition: codelet_rpc_types::CustomModelDefinition,
            },
            /// RPC-347: emitted by the ModelSelector `d` keybind (RPC-344) after the
            /// delete-confirm overlay. App::dispatch spawns
            /// `backend.delete_custom_model(..)`.
            DeleteCustomModel {
                provider_id: String,
                profile_name: String,
                model_id: String,
            },
            /// RPC-054: a spawned `backend.list_provider_credentials()` task
            /// resolved. App::dispatch folds the result into the open
            /// ProviderSettingsView.
            ProviderCredentialsLoaded(Vec<codelet_rpc_types::ProviderCredentialInfo>),
            ProviderSettingsConfigsLoaded(Vec<codelet_rpc_types::ProviderSettingsConfig>),
            SaveProviderSettings {
                provider_id: String,
                definition: codelet_rpc_types::ProfileDefinition,
            },
            /// RPC-054: emitted by the view on Enter inside the edit-API-key
            /// form. App::dispatch spawns `backend.set_provider_credentials`
            /// followed by a fresh `list_provider_credentials` refresh.
            SaveProviderCredentials {
                provider_id: String,
                api_key: String,
            },
            /// RPC-054: emitted by the view on the `t` key. App::dispatch
            /// spawns `backend.test_provider_connection` and folds the result
            /// back via `Action::ProviderTestComplete`.
            TestProviderConnection(String),
            /// RPC-054: a spawned `backend.test_provider_connection` task
            /// resolved. App::dispatch folds the result into the view's status
            /// area.
            ProviderTestComplete {
                provider_id: String,
                result: codelet_rpc_types::TestConnectionResult,
            },
            /// RPC-054: emitted by the view on the `r` key. App::dispatch
            /// spawns `backend.refresh_models_cache` and follows up with a
            /// `list_provider_credentials` refresh.
            RefreshProviderModels(String),
            /// RPC-054: a spawned `backend.refresh_models_cache` task resolved.
            /// App::dispatch folds the new model count into the view via a
            /// follow-up list refresh and updates the status area.
            ProviderModelsRefreshed {
                provider_id: String,
                model_count: u32,
            },
            /// RPC-054: emitted by the view on the `d` key. App::dispatch
            /// spawns `backend.delete_provider_credentials` followed by a
            /// fresh `list_provider_credentials` refresh.
            DeleteProviderCredentials(String),
            /// RPC-054: emitted by the view AFTER the user accepts the
            /// ConfirmDialog primary button — replaces the previous
            /// direct-from-`d` flow so the destructive backend call never
            /// fires without explicit confirmation. App::dispatch routes this
            /// arm to the same `delete_provider_credentials` handler as the
            /// raw `DeleteProviderCredentials` variant.
            ConfirmDeleteProviderCredentials(String),
            /// RPC-054: emitted by the App for inline status updates that
            /// don't fit one of the typed *Loaded / *Complete variants
            /// (e.g. error messages from save / delete spawns).
            ProviderSettingsStatus(String),

            /// PROV-109: emitted by the profile create/edit form on submit.
            /// App::dispatch spawns `backend.save_profile(provider_id,
            /// profile_name, definition)` followed by a `list_provider_credentials`
            /// refresh so the openai profile slice repaints with the new state.
            /// PROV-136: `old_profile_name` is `Some(original)` for an edit-mode
            /// rename (dispatch routes to `rename_profile`), else `None`.
            SaveProfile {
                provider_id: String,
                profile_name: String,
                old_profile_name: Option<String>,
                definition: codelet_rpc_types::ProfileDefinition,
            },
            /// PROV-109: raw delete request for a local-server profile. App::dispatch
            /// spawns `backend.delete_profile(..)` + a list refresh. The view-layer is
            /// responsible for opening the confirm dialog before this fires.
            DeleteProfile {
                provider_id: String,
                profile_name: String,
            },
            /// PROV-109: emitted AFTER the user accepts the delete ConfirmDialog
            /// primary button. Routes through the SAME delete handler as
            /// `DeleteProfile` so the destructive backend call never fires without
            /// explicit confirmation (mirrors `ConfirmDeleteProviderCredentials`).
            ConfirmDeleteProfile {
                provider_id: String,
                profile_name: String,
            },
            /// PROV-116: emitted from the SUCCESS branch of a profile delete (just
            /// before the reload) to record the parent provider the reload should
            /// re-focus. A failed delete never emits it, so the cursor never jumps.
            ProfileDeleteNavigate {
                provider_id: String,
            },

            /// PROV-112: emitted by the ProviderSettingsView's DisconnectOAuth
            /// confirm dialog when the user presses `y`/`Y`. App::dispatch spawns
            /// `backend.oauth_clear_tokens(provider_id)` (per-provider routing lives
            /// in the backend) followed by a `list_provider_credentials` refresh so
            /// the disconnected provider's `oauth-status` (Logout) row disappears.
            /// Backend clear errors are swallowed (status without leaking the RPC
            /// name); the clear is idempotent.
            OAuthDisconnect {
                provider_id: String,
            },

            // ========================================================================
            // PROV-113: OAuth login actions (browser / headless / device). The view
            // emits `OAuthLoginStart` on Enter over a login row; App::dispatch routes
            // by (provider, method) to the backend browser/headless/device calls. The
            // intermediate `OAuthHeadlessReady` / `OAuthDeviceReady` results set the
            // code-entry / device-waiting modes; `OAuthLoginSucceeded` /
            // `OAuthLoginFailed` fold the terminal outcome (dropped when their
            // generation no longer matches the view's, i.e. the flow was cancelled).
            // ========================================================================
            /// PROV-113: start an OAuth login for `provider_id` using `method`. Carries
            /// the view's current generation so a stale result can be dropped.
            OAuthLoginStart {
                provider_id: String,
                method: crate::views::provider_settings::nav_item::OAuthMethod,
                generation: u64,
            },
            /// PROV-113: anthropic headless start resolved — set the code-entry mode.
            OAuthHeadlessReady {
                provider_id: String,
                authorize_url: String,
                pkce_verifier: String,
                generation: u64,
            },
            /// PROV-113: codex device start resolved — set device-waiting + poll.
            OAuthDeviceReady {
                provider_id: String,
                user_code: String,
                verification_url: String,
                device_auth_id: String,
                interval: u64,
                generation: u64,
            },
            /// PROV-113: submit the pasted headless code (anthropic).
            OAuthLoginHeadlessSubmit {
                provider_id: String,
                code: String,
                pkce_verifier: String,
                generation: u64,
            },
            /// PROV-113: a login completed successfully.
            OAuthLoginSucceeded {
                provider_id: String,
                generation: u64,
            },
            /// PROV-113: a login failed (message is UI-safe — no RPC/method name).
            OAuthLoginFailed {
                provider_id: String,
                error: String,
                generation: u64,
            },
            /// PROV-113: open the authorize URL in the user's browser (best-effort).
            OAuthOpenUrl {
                url: String,
            },
            /// PROV-113: copy the authorize URL to the clipboard (best-effort).
            OAuthCopyUrl {
                url: String,
            },
            /// PROV-114: begin the github-copilot device flow. `enterprise_host` is
            /// `None` for GitHub.com or `Some(normalized_host)` for GitHub Enterprise.
            /// Carries the view's current generation so a stale result can be dropped.
            /// App::dispatch spawns `backend.oauth_copilot_device_start(enterprise_host)`
            /// then reuses the PROV-113 `OAuthDeviceReady` → `oauth_device_poll` path.
            OAuthCopilotDeviceStart {
                enterprise_host: Option<String>,
                generation: u64,
            },

        }
    };
}

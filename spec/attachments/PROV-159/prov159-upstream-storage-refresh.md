# PROV-159 — Original Codex storage and refresh samples

Repository: https://github.com/openai/codex
Commit: `bb64b894fd67158be0f874872e6d7bfe717e7368` (reviewed 2026-10-16).
The snippets are verbatim excerpts, not proposed fspec implementations.
Each source link is pinned; omitted surrounding code is not implied to be copied.
Cross-reference finding IDs in `prov159-oauth-research.md`.

## S1 — One backend owns keychain load/save/delete (F1)

Source: `codex-rs/login/src/auth/storage.rs:310`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/login/src/auth/storage.rs#L310-L347

```rust
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let key = compute_store_key(&self.codex_home)?;
        self.load_from_keyring(&key)
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        let key = compute_store_key(&self.codex_home)?;
        // Simpler error mapping per style: prefer method reference over closure
        let serialized = serde_json::to_string(auth).map_err(std::io::Error::other)?;
        self.save_to_keyring(&key, &serialized)?;
        let mut telemetry = storage_telemetry::telemetry(
            self.mode,
            AuthKeyringBackendKind::Direct,
            Operation::Cleanup,
        );
        let result = delete_file_if_exists(&self.codex_home);
        telemetry.record_delete_attempt(Store::File, &result);
        if let Err(err) = &result {
            warn!("failed to remove CLI auth fallback file: {err}");
        }
        drop(telemetry);
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        let key = compute_store_key(&self.codex_home)?;
        let keyring_removed = self
            .keyring_store
            .delete(KEYRING_SERVICE, &key)
            .map_err(std::io::Error::from)
            .map_err(|error| {
                storage_error::with_context("failed to delete auth from keyring", error)
            })?;
        let file_removed = delete_file_if_exists(&self.codex_home)?;
        Ok(keyring_removed || file_removed)
    }
```

Adaptation: use a coherent backend policy in fspec, not file-only mutations after
keychain-first reads. Do not copy upstream's whole-account delete literally for
OAuth-only disconnect: fspec's independent API-key preservation is an existing
contract. Also handle/report partial cleanup failures honestly.

## S2 — Shared refresh gate and guarded reload (F3/F4)

Source: `codex-rs/login/src/auth/manager.rs:2856`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/login/src/auth/manager.rs#L2856-L2895

```rust
    async fn refresh_token_with_reason(
        &self,
        reason: RefreshReason,
    ) -> Result<(), RefreshTokenError> {
        let _refresh_guard = self.refresh_lock.acquire().await.map_err(|_| {
            RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                REFRESH_TOKEN_UNKNOWN_MESSAGE.to_string(),
            ))
        })?;
        let auth_before_reload = self.auth_cached();
        if auth_before_reload
            .as_ref()
            .is_some_and(|auth| auth.is_api_key_auth() || auth.is_personal_access_token_auth())
        {
            return Ok(());
        }
        let expected_account_id = auth_before_reload
            .as_ref()
            .and_then(CodexAuth::get_account_id);

        match self
            .reload_if_account_id_matches(expected_account_id.as_deref())
            .await
        {
            ReloadOutcome::ReloadedChanged => {
                tracing::info!("Skipping token refresh because auth changed after guarded reload.");
                Ok(())
            }
            ReloadOutcome::ReloadedNoChange => self.refresh_token_from_authority_impl(reason).await,
            ReloadOutcome::Skipped => {
                Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                    RefreshTokenFailedReason::Other,
                    REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE.to_string(),
                )))
            }
        }
    }
```

Adaptation: serialize independent clients through the credential authority,
reuse a winner's refreshed tokens and reject unexpected account changes.
This snippet alone does NOT provide a cross-process lock or a post-network
compare-and-swap. fspec must additionally fence late persistence against logout,
same-account re-login, and different-account login (F4/A7–A9).

## S3 — Logout explicitly clears/reloads cached auth (F2)

Source: `codex-rs/login/src/auth/manager.rs:2972`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/login/src/auth/manager.rs#L2972-L2983

```rust
    pub async fn logout(&self) -> std::io::Result<bool> {
        self.ensure_logout_allowed()?;
        let removed = logout_all_stores(
            &self.codex_home,
            self.auth_credentials_store_mode,
            self.keyring_backend_kind,
        )?;
        // Always reload to clear cached auth, even if no stored credentials were present.
        self.clear_external_auth();
        self.reload().await;
        Ok(removed)
    }
```

Adaptation: mutating storage and reloading a credential navigation list is not
enough. Already-created transports/providers must observe the auth transition.
Audit HTTP, WebSocket connections and account-scoped routing state together.

## S4 — Expiry-aware proactive refresh (F7)

Source: `codex-rs/login/src/auth/manager.rs:3030`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/login/src/auth/manager.rs#L3030-L3048

```rust
    fn proactive_refresh_reason(auth: &CodexAuth) -> Option<RefreshReason> {
        let chatgpt_auth = match auth {
            CodexAuth::Chatgpt(chatgpt_auth) => chatgpt_auth,
            _ => return None,
        };

        let auth_dot_json = chatgpt_auth.current_auth_json()?;
        if let Some(tokens) = auth_dot_json.tokens.as_ref()
            && let Ok(Some(expires_at)) = parse_jwt_expiration(&tokens.access_token)
        {
            return (expires_at
                <= Utc::now()
                    + chrono::Duration::minutes(CHATGPT_ACCESS_TOKEN_REFRESH_WINDOW_MINUTES))
            .then_some(RefreshReason::ProactiveExpiry);
        }
        let last_refresh = auth_dot_json.last_refresh?;
        (last_refresh < Utc::now() - chrono::Duration::days(TOKEN_REFRESH_INTERVAL))
            .then_some(RefreshReason::Scheduled)
    }
```

Adaptation: refresh based on freshness and a safety window rather than always
passing `Some(0)` for disk tokens. Choose and test fspec's own conservative
fallback policy; upstream constants need not be copied.

## S5 — Restrictive auth-file creation permissions (F8)

Source: `codex-rs/login/src/auth/storage.rs:214`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/login/src/auth/storage.rs#L214-L231

```rust
    fn save(&self, auth_dot_json: &AuthDotJson) -> std::io::Result<()> {
        let auth_file = get_auth_file(&self.codex_home);

        if let Some(parent) = auth_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json_data = serde_json::to_string_pretty(auth_dot_json)?;
        let mut options = OpenOptions::new();
        options.truncate(true).write(true).create(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = options.open(auth_file)?;
        file.write_all(json_data.as_bytes())?;
        file.flush()?;
        Ok(())
    }
```

Limit: this sample sets the Unix creation mode; it does NOT make a write atomic
or necessarily tighten permissions on an already existing permissive file.
fspec's safe persistence requirements go beyond this quoted sample.

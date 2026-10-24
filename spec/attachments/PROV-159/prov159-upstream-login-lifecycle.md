# PROV-159 — Original Codex interactive login lifecycle samples

Repository: https://github.com/openai/codex
Commit: `bb64b894fd67158be0f874872e6d7bfe717e7368` (reviewed 2026-10-16).
Verbatim excerpts, not drop-in fspec implementations. Findings/acceptance tests
are in `prov159-oauth-research.md`; storage/refresh samples are separate.

## L1 — A login attempt owns real backend cancellation (F5)

Source: `codex-rs/app-server/src/request_processors/account_processor.rs:42`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L42-L70

```rust
enum ActiveLogin {
    Browser {
        shutdown_handle: ShutdownHandle,
        login_id: Uuid,
    },
    DeviceCode {
        cancel: CancellationToken,
        login_id: Uuid,
    },
}

impl ActiveLogin {
    fn login_id(&self) -> Uuid {
        match self {
            ActiveLogin::Browser { login_id, .. } | ActiveLogin::DeviceCode { login_id, .. } => {
                *login_id
            }
        }
    }

    fn cancel(&self) {
        match self {
            ActiveLogin::Browser {
                shutdown_handle, ..
            } => shutdown_handle.shutdown(),
            ActiveLogin::DeviceCode { cancel, .. } => cancel.cancel(),
        }
    }
}
```

Source: same file, `:92`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L92-L96

```rust
impl Drop for ActiveLogin {
    fn drop(&mut self) {
        self.cancel();
    }
}
```

Adaptation: keep generation filtering for UI results, but also cancel the
backend operation and prevent its credential commit. Ensure listener shutdown
has completed before fspec immediately binds port 1455 for a replacement attempt.

## L2 — Prompt start response, asynchronous completion, attempt identity (F5/F6)

Source: same file, `:677`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L677-L692

```rust
        let login_id = Uuid::new_v4();
        let shutdown_handle = server.cancel_handle();

        // Replace active login if present.
        {
            let mut guard = self.active_login.lock().await;
            if let Some(existing) = guard.take() {
                drop(existing);
            }
            *guard = Some(ActiveLogin::Browser {
                shutdown_handle: shutdown_handle.clone(),
                login_id,
            });
        }
```

Source: same file, `:727`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L727-L741

```rust
            // Clear the active login if it matches this attempt. It may have been replaced or cancelled.
            let mut guard = active_login.lock().await;
            if guard.as_ref().map(ActiveLogin::login_id) == Some(login_id) {
                *guard = None;
            }
        };
        tokio::spawn(AuthStorageOriginator::current().scope(task));

        Ok(LoginAccountResponse::Chatgpt {
            login_id: login_id.to_string(),
            auth_url,
        })
```

Adaptation: upstream separates the short request from the human authorization
wait. fspec can use this pattern or explicit operation-specific RPC deadlines.
Merely increasing all RPC deadlines does not fix cancellation or stale commits.
The matching-attempt cleanup shown here is not itself a token-write fence.

## L3 — Device polling selects on cancellation; cancellation is attempt-scoped (F5)

Source: same file, `:777`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L777-L789

```rust
        let task = async move {
            let (success, error_msg) = tokio::select! {
                _ = cancel.cancelled() => {
                    (false, Some("Login was not completed".to_string()))
                }
                r = complete_device_code_login(opts, device_code) => {
                    match r {
                        Ok(()) => (true, None),
                        Err(err) => (false, Some(err.to_string())),
                    }
                }
            };
```

Source: same file, `:813`.
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L813-L826

```rust
    async fn cancel_login_chatgpt_common(
        &self,
        login_id: Uuid,
    ) -> std::result::Result<(), CancelLoginError> {
        let mut guard = self.active_login.lock().await;
        if guard.as_ref().map(ActiveLogin::login_id) == Some(login_id) {
            if let Some(active) = guard.take() {
                drop(active);
            }
            Ok(())
        } else {
            Err(CancelLoginError::NotFound)
        }
    }
```

Adaptation: cancelling old attempt A must not cancel replacement B. Cover
cancellation during polling, token exchange, and immediately before persistence,
not just cancellation before the first request.

## L4 — Successful login reloads authoritative auth before downstream work (F2)

Source: same file, `:980` (opening excerpt of the function).
https://github.com/openai/codex/blob/bb64b894fd67158be0f874872e6d7bfe717e7368/codex-rs/app-server/src/request_processors/account_processor.rs#L980-L991

```rust
    async fn send_chatgpt_login_completion_notifications(
        &self,
        mut payload_v2: AccountLoginCompletedNotification,
    ) {
        if payload_v2.success {
            self.auth_manager.reload().await;
            let auth_changes = self.auth_manager.auth_change_state_receiver();
            let owner_generation = auth_changes.borrow().owner_generation;
            self.thread_manager.invalidate_mcp_runtimes().await;
            self.config_manager.replace_cloud_config_bundle_loader(
                self.auth_manager.clone(),
                self.config.chatgpt_base_url.clone(),
```

Adaptation: auth transitions invalidate downstream account-bound state, not just
the settings list. fspec should implement only its applicable invalidation needs,
not transplant unrelated upstream enterprise/plugin machinery.

## Browser-session caveat

Neither these lifecycle snippets nor local credential deletion clear browser
OpenAI SSO cookies. An immediate login may legitimately authorize the same account
without another password prompt. Diagnose stale persisted/client auth separately;
do not promise an account chooser or remote session revocation without a defined
product requirement and verified provider support.

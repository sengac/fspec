//! PROV-112 — OAuth disconnect/logout: per-provider token clearing.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//!
//! `FspecService::oauth_clear_tokens` and `oauth_get_tokens` delegate here.
//! Dispatch is keyed by `provider_id` and forwards to the SAME
//! `codelet_providers` primitives the napi `*_oauth.rs` wrappers use, so the
//! Rust frontend gets a real, napi-direct clear WITHOUT a `codelet-napi`
//! dependency (the arrow rpc → napi is forbidden; rpc → providers is not) and
//! WITHOUT routing through `SessionManagerHandle`/core.
//!
//! Provider routing (mirrors `useProviderSettingsState.disconnectOauth`):
//!   * `anthropic`      → delete `claude_auth.json` (idempotent).
//!   * `github-copilot` → `delete_copilot_auth` (idempotent).
//!   * `codex` / *else* → strip the `tokens` field from `auth.json`, keeping
//!     the cached `OPENAI_API_KEY`.
//!
//! Every clear is idempotent (a missing file/credential resolves to `Ok`).

use codelet_providers::claude_auth::{get_claude_auth_path, read_claude_auth};
use codelet_providers::codex::codex_auth::{read_codex_auth, write_codex_auth, CodexAuthJson};
use codelet_providers::copilot::auth::{delete_copilot_auth, read_copilot_auth};

/// Clear the OAuth tokens for `provider_id`. Errors are returned as `String`
/// (the tarpc `Result<(), String>` shape); the frontend swallows them so the
/// RPC/method name never leaks into the UI.
pub async fn clear_oauth_tokens(provider_id: &str) -> Result<(), String> {
    match provider_id {
        "anthropic" => clear_claude_tokens().await,
        "github-copilot" => delete_copilot_auth()
            .await
            .map_err(|e| format!("clear failed: {e}")),
        // codex AND fallback: strip only the tokens field, keep OPENAI_API_KEY.
        _ => clear_codex_tokens(),
    }
}

/// Whether `provider_id` currently has OAuth tokens persisted. Used by the
/// nav reload to decide if the `oauth-status` (Logout) row should be shown.
pub async fn has_oauth_tokens(provider_id: &str) -> Result<bool, String> {
    match provider_id {
        "anthropic" => Ok(read_claude_auth()
            .await
            .map_err(|e| format!("read failed: {e}"))?
            .is_some()),
        "github-copilot" => Ok(read_copilot_auth()
            .await
            .map_err(|e| format!("read failed: {e}"))?
            .is_some()),
        _ => Ok(read_codex_auth()
            .map_err(|e| format!("read failed: {e}"))?
            .map(|a| a.tokens.is_some())
            .unwrap_or(false)),
    }
}

/// Delete `claude_auth.json`. Idempotent — a missing file resolves to `Ok`.
async fn clear_claude_tokens() -> Result<(), String> {
    let path = get_claude_auth_path();
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("clear failed: {e}")),
    }
}

/// Strip the `tokens` field from the codex `auth.json`, preserving the cached
/// `OPENAI_API_KEY`. Idempotent — a missing file resolves to `Ok`.
fn clear_codex_tokens() -> Result<(), String> {
    match read_codex_auth().map_err(|e| format!("read failed: {e}"))? {
        Some(auth) => {
            let stripped = strip_codex_tokens(auth);
            write_codex_auth(&stripped).map_err(|e| format!("clear failed: {e}"))
        }
        None => Ok(()),
    }
}

/// Pure transform: drop the OAuth `tokens` while leaving every sibling field
/// (notably `OPENAI_API_KEY`) untouched. Factored out so the
/// preserve-API-key rule is unit-testable without touching the filesystem.
pub(crate) fn strip_codex_tokens(mut auth: CodexAuthJson) -> CodexAuthJson {
    auth.tokens = None;
    auth
}

#[cfg(test)]
mod tests {
    use super::*;
    use codelet_providers::codex::codex_auth::CodexTokens;

    fn tokens() -> CodexTokens {
        CodexTokens {
            id_token: "id".into(),
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            account_id: "acct".into(),
        }
    }

    #[test]
    fn strip_codex_tokens_preserves_openai_api_key() {
        let auth = CodexAuthJson {
            openai_api_key: Some("sk-keep-me".into()),
            tokens: Some(tokens()),
            last_refresh: Some("2026-01-01".into()),
        };

        let stripped = strip_codex_tokens(auth);

        assert!(stripped.tokens.is_none(), "tokens must be cleared");
        assert_eq!(
            stripped.openai_api_key.as_deref(),
            Some("sk-keep-me"),
            "cached OPENAI_API_KEY must be preserved"
        );
        assert_eq!(stripped.last_refresh.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn strip_codex_tokens_idempotent_when_already_empty() {
        let auth = CodexAuthJson {
            openai_api_key: None,
            tokens: None,
            last_refresh: None,
        };
        let stripped = strip_codex_tokens(auth);
        assert!(stripped.tokens.is_none());
    }
}

//! Token refresh decision logic for the Copilot provider (PROV-057).
//!
//! This module owns the pure-function helpers that decide whether the
//! short-lived Copilot token needs to be exchanged and how to apply the
//! exchange response to the in-memory auth state.
//!
//! Extracted from `provider.rs` to satisfy the 300-line file budget and
//! to keep the refresh concern (a single responsibility) in its own module.

use crate::copilot::auth::CopilotAuthJson;
use crate::copilot::token_exchange::TokenExchangeResponse;

/// Apply a [`TokenExchangeResponse`] to a mutable [`CopilotAuthJson`].
///
/// Pure logic extracted so the refresh-decision tests do not need to spin
/// up a mock HTTP server — they can feed the response straight in.
pub(crate) fn apply_exchange_response(
    auth: &mut CopilotAuthJson,
    exchange: TokenExchangeResponse,
) {
    auth.copilot_token = Some(exchange.token);
    auth.copilot_token_expires_at = Some(exchange.expires_at);
    if !exchange.endpoints_api.is_empty() {
        auth.endpoints_api = Some(exchange.endpoints_api);
    }
}

/// PROV-057 Rule 4: decide whether the cached Copilot token needs to be
/// refreshed. A refresh is needed if:
///
/// 1. There is no cached Copilot token at all, or
/// 2. The cached token's `expires_at` is within 60 seconds of `now`.
///
/// This is a pure function so tests can feed a deterministic `now` rather
/// than relying on wall-clock time.
#[must_use]
pub(crate) fn needs_copilot_token_refresh(auth: &CopilotAuthJson, now: u64) -> bool {
    match (auth.copilot_token.as_deref(), auth.copilot_token_expires_at) {
        (Some(tok), Some(exp)) if !tok.is_empty() => exp <= now + 60,
        _ => true,
    }
}

/// Current unix seconds. Extracted as a free function so tests can bypass
/// it by calling [`needs_copilot_token_refresh`] directly.
pub(crate) fn unix_timestamp_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::copilot::auth::CopilotAuthJson;
    use crate::copilot::token_exchange::TokenExchangeResponse;

    /// Build a throwaway `CopilotAuthJson` with a fresh (not-near-expiry)
    /// cached Copilot token.
    fn auth_with_fresh_copilot_token(now: u64) -> CopilotAuthJson {
        CopilotAuthJson {
            github_oauth_token: "gho_long_lived".to_string(),
            copilot_token: Some("tid=fresh;:sig".to_string()),
            copilot_token_expires_at: Some(now + 20 * 60),
            endpoints_api: Some("https://api.githubcopilot.com".to_string()),
            enterprise_url: None,
        }
    }

    /// Build an auth with a cached Copilot token that's about to expire.
    fn auth_with_expiring_copilot_token(now: u64) -> CopilotAuthJson {
        CopilotAuthJson {
            github_oauth_token: "gho_long_lived".to_string(),
            copilot_token: Some("tid=expiring;:sig".to_string()),
            copilot_token_expires_at: Some(now + 30),
            endpoints_api: Some("https://api.githubcopilot.com".to_string()),
            enterprise_url: None,
        }
    }

    /// Build an auth without any cached Copilot token (fresh login).
    fn auth_without_copilot_token() -> CopilotAuthJson {
        CopilotAuthJson::from_github_oauth_token("gho_long_lived".to_string(), None)
    }

    #[test]
    fn needs_refresh_is_true_when_no_copilot_token_cached() {
        let auth = auth_without_copilot_token();
        assert!(needs_copilot_token_refresh(&auth, 1_700_000_000));
    }

    #[test]
    fn needs_refresh_is_true_when_within_60_seconds_of_expiry() {
        let now = 1_700_000_000;
        let auth = auth_with_expiring_copilot_token(now);
        assert!(needs_copilot_token_refresh(&auth, now));
    }

    #[test]
    fn needs_refresh_is_true_exactly_at_the_60_second_boundary() {
        let now = 1_700_000_000;
        let mut auth = auth_with_fresh_copilot_token(now);
        auth.copilot_token_expires_at = Some(now + 60);
        assert!(
            needs_copilot_token_refresh(&auth, now),
            "an expires_at exactly at now+60 must refresh per PROV-057 Rule 4"
        );
    }

    #[test]
    fn needs_refresh_is_false_when_token_is_not_near_expiry() {
        let now = 1_700_000_000;
        let auth = auth_with_fresh_copilot_token(now);
        assert!(
            !needs_copilot_token_refresh(&auth, now),
            "20-minute-away expiry must NOT trigger refresh"
        );
    }

    #[test]
    fn needs_refresh_is_true_when_expires_at_is_missing_even_with_token() {
        let auth = CopilotAuthJson {
            github_oauth_token: "gho_long_lived".to_string(),
            copilot_token: Some("orphaned".to_string()),
            copilot_token_expires_at: None,
            endpoints_api: None,
            enterprise_url: None,
        };
        assert!(needs_copilot_token_refresh(&auth, 1_700_000_000));
    }

    #[test]
    fn needs_refresh_is_true_when_cached_copilot_token_string_is_empty() {
        let auth = CopilotAuthJson {
            github_oauth_token: "gho_long_lived".to_string(),
            copilot_token: Some(String::new()),
            copilot_token_expires_at: Some(9_999_999_999),
            endpoints_api: None,
            enterprise_url: None,
        };
        assert!(needs_copilot_token_refresh(&auth, 1_700_000_000));
    }

    #[test]
    fn apply_exchange_response_populates_all_fields() {
        let mut auth = auth_without_copilot_token();
        let exchange = TokenExchangeResponse {
            token: "tid=new;:sig".to_string(),
            expires_at: 2_000_000_000,
            endpoints_api: "https://copilot-api.ghe.example.com".to_string(),
        };
        apply_exchange_response(&mut auth, exchange);
        assert_eq!(auth.copilot_token.as_deref(), Some("tid=new;:sig"));
        assert_eq!(auth.copilot_token_expires_at, Some(2_000_000_000));
        assert_eq!(
            auth.endpoints_api.as_deref(),
            Some("https://copilot-api.ghe.example.com")
        );
        assert_eq!(auth.github_oauth_token, "gho_long_lived");
    }

    #[test]
    fn apply_exchange_response_preserves_endpoints_api_when_response_is_empty() {
        let mut auth = CopilotAuthJson {
            github_oauth_token: "gho_long_lived".to_string(),
            copilot_token: None,
            copilot_token_expires_at: None,
            endpoints_api: Some("https://copilot-api.ghe.example.com".to_string()),
            enterprise_url: Some("ghe.example.com".to_string()),
        };
        let exchange = TokenExchangeResponse {
            token: "tid=new;:sig".to_string(),
            expires_at: 2_000_000_000,
            endpoints_api: String::new(),
        };
        apply_exchange_response(&mut auth, exchange);
        assert_eq!(
            auth.endpoints_api.as_deref(),
            Some("https://copilot-api.ghe.example.com"),
            "empty endpoints_api from exchange must not clobber the existing value"
        );
    }
}

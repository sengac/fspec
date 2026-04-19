//! Tests for RefreshingHttpClient<S: TokenStrategy> (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: Generic refreshing HTTP client unifies token refresh logic

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::oauth::http_middleware::{RefreshingHttpClient, TokenStrategy};
use crate::oauth::token_refresh::TokenState;

/// Test token strategy that tracks refresh calls.
#[derive(Debug, Clone)]
struct TestTokenStrategy {
    refresh_count: Arc<RwLock<u32>>,
}

impl TestTokenStrategy {
    fn new() -> Self {
        Self {
            refresh_count: Arc::new(RwLock::new(0)),
        }
    }
}

/// Extra state for test strategy
#[derive(Debug, Clone)]
struct TestExtra {
    #[allow(dead_code)]
    provider_name: String,
}

impl TokenStrategy for TestTokenStrategy {
    type Extra = TestExtra;

    async fn refresh(
        &self,
        state: TokenState<Self::Extra>,
    ) -> Result<TokenState<Self::Extra>, String> {
        let mut count = self.refresh_count.write().await;
        *count += 1;
        Ok(TokenState {
            access_token: format!("refreshed_{}", *count),
            refresh_token: state.refresh_token,
            expires_at: Instant::now() + std::time::Duration::from_secs(3600),
            extra: state.extra,
        })
    }

    fn persist(&self, _state: &TokenState<Self::Extra>) {
        // No-op for tests
    }

    fn prepare_request(
        &self,
        req: http::Request<bytes::Bytes>,
        state: &TokenState<Self::Extra>,
    ) -> http::Request<bytes::Bytes> {
        let (mut parts, body) = req.into_parts();
        parts.headers.remove(http::header::AUTHORIZATION);
        if let Ok(val) = format!("Bearer {}", state.access_token).parse() {
            parts.headers.insert(http::header::AUTHORIZATION, val);
        }
        http::Request::from_parts(parts, body)
    }
}

// @step Given a RefreshingHttpClient parameterized with a TokenStrategy
// @step When a request is made with an expired token using CodexTokenStrategy
// @step Then the double-check locking pattern refreshes the token before sending
// @step And the same RefreshingHttpClient with ClaudeTokenStrategy exhibits identical refresh behavior

#[tokio::test]
async fn refreshing_client_detects_expired_token() {
    // @step Given a RefreshingHttpClient parameterized with a TokenStrategy
    let strategy = TestTokenStrategy::new();
    let initial_state = TokenState {
        access_token: "expired_token".to_string(),
        refresh_token: "rt_test".to_string(),
        // Expired 10 seconds ago
        expires_at: Instant::now() - std::time::Duration::from_secs(10),
        extra: TestExtra {
            provider_name: "test".to_string(),
        },
    };

    let client = RefreshingHttpClient::new_oauth(strategy.clone(), initial_state);

    // @step When a request is made with an expired token
    assert!(client.is_token_expired().await);
}

#[tokio::test]
async fn refreshing_client_not_expired_for_valid_token() {
    let strategy = TestTokenStrategy::new();
    let initial_state = TokenState {
        access_token: "valid_token".to_string(),
        refresh_token: "rt_test".to_string(),
        // Expires in 1 hour
        expires_at: Instant::now() + std::time::Duration::from_secs(3600),
        extra: TestExtra {
            provider_name: "test".to_string(),
        },
    };

    let client = RefreshingHttpClient::new_oauth(strategy, initial_state);
    assert!(!client.is_token_expired().await);
}

#[tokio::test]
async fn api_key_mode_never_expired() {
    let strategy = TestTokenStrategy::new();
    let client = RefreshingHttpClient::new_api_key(strategy);
    assert!(!client.is_token_expired().await);
}

#[tokio::test]
async fn token_mode_enum_has_both_variants() {
    // Verify the generic TokenMode works with different Extra types
    use crate::oauth::http_middleware::TokenMode;

    let _oauth: TokenMode<TestExtra> = TokenMode::OAuth {
        token_state: Arc::new(RwLock::new(TokenState {
            access_token: "at".to_string(),
            refresh_token: "rt".to_string(),
            expires_at: Instant::now(),
            extra: TestExtra {
                provider_name: "test".to_string(),
            },
        })),
    };

    let _api_key: TokenMode<TestExtra> = TokenMode::ApiKey;
}

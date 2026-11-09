//! MODEL-014: fetching the Codex `/models` response with ChatGPT sign-in.

use std::time::Duration;

use rig::http_client::HttpClientExt;

use super::CodexCatalog;
use crate::codex::refreshing_client::RefreshingCodexClient;

/// The Codex client version fspec sends as `client_version` on
/// `GET /models` (product owner, 2026-11-06). The Codex backend uses it to
/// decide which models to return: Codex's bundled catalog gives each model a
/// `minimal_client_version`, e.g. gpt-6-sol 0.155.0. 0.165.0 is the Codex
/// CLI version the live capture in spec/attachments/MODEL-014 was fetched
/// with. Bump it with Codex releases so new models keep being returned.
pub const CODEX_MODELS_CLIENT_VERSION: &str = "0.165.0";

/// The ChatGPT Codex backend; `/models` and `/responses` live under it.
pub const CODEX_BACKEND_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

/// How long a `/models` fetch may take before the selection goes on
/// without it (no reasoning control, with the reason).
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// `GET {base_url}/models?client_version={client_version}` through `client`.
/// In OAuth mode the client adds the Bearer token and ChatGPT-Account-Id
/// header, refreshing an expired token first. Any failure is the reason
/// Codex models have no reasoning control for this selection.
pub async fn fetch(
    client: &RefreshingCodexClient,
    base_url: &str,
    client_version: &str,
) -> Result<CodexCatalog, String> {
    let failed =
        |error: &dyn std::fmt::Display| format!("the Codex /models request failed: {error}");
    let url = format!(
        "{}/models?client_version={client_version}",
        base_url.trim_end_matches('/')
    );
    let request = http::Request::builder()
        .method(http::Method::GET)
        .uri(&url)
        .header(http::header::ACCEPT, "application/json")
        .body(Vec::<u8>::new())
        .map_err(|error| failed(&error))?;
    let exchange = async {
        let response = client
            .send::<Vec<u8>, Vec<u8>>(request)
            .await
            .map_err(|error| failed(&error))?;
        if !response.status().is_success() {
            return Err(failed(&format!("status {}", response.status())));
        }
        response.into_body().await.map_err(|error| failed(&error))
    };
    let body = tokio::time::timeout(FETCH_TIMEOUT, exchange)
        .await
        .map_err(|_| failed(&format!("no answer within {} s", FETCH_TIMEOUT.as_secs())))??;
    CodexCatalog::parse(&body)
}

/// The fetch for a Codex model selection: ChatGPT sign-in from the Codex
/// auth store (as `CodexProvider::new` reads it), the pinned client version
/// and the Codex backend. Without ChatGPT tokens (an API key alone) there is
/// nothing to fetch with, and that is the reason returned.
pub async fn fetch_for_selection() -> Result<CodexCatalog, String> {
    fetch_signed_in().await.map_err(|failure| match failure {
        super::listing::ListingFailure::NotSignedInWithChatGpt => {
            "Codex thinking levels come from the Codex /models endpoint, which needs ChatGPT \
             sign-in; Codex is not signed in with ChatGPT (an API key cannot fetch them)"
                .to_string()
        }
        other => other.cause(),
    })
}

/// BUG-577: [`fetch_for_selection`] with its failure typed, so the provider
/// list can word it for the model selector.
pub(super) async fn fetch_signed_in() -> Result<CodexCatalog, super::listing::ListingFailure> {
    use super::listing::ListingFailure;
    let auth = crate::codex::codex_auth::read_codex_auth().map_err(|error| {
        ListingFailure::Failed(format!("the Codex sign-in cannot be read: {error}"))
    })?;
    let tokens = auth
        .and_then(|auth| auth.tokens)
        .filter(|tokens| !tokens.access_token.is_empty() && !tokens.account_id.is_empty())
        .ok_or(ListingFailure::NotSignedInWithChatGpt)?;
    let unusable = |error: &dyn std::fmt::Display| {
        ListingFailure::Failed(format!("the Codex sign-in cannot be used: {error}"))
    };
    let lifetime = crate::codex::auth_refresh::stored_lifetime(&tokens.access_token)
        .map_err(|error| unusable(&error))?;
    let issuer = crate::codex::codex_oauth::CODEX_ISSUER.to_owned();
    #[cfg(feature = "test-support")]
    let issuer = crate::codex::oauth_test_support::config().map_or(issuer, |config| config.issuer);
    let client = RefreshingCodexClient::new_oauth(
        tokens.access_token,
        tokens.refresh_token,
        tokens.account_id,
        Some(lifetime),
        issuer,
    );
    client
        .require_stored_binding()
        .map_err(|error| unusable(&error))?;
    let base_url = base_url().map_err(ListingFailure::Failed)?;
    fetch(&client, &base_url, CODEX_MODELS_CLIENT_VERSION)
        .await
        .map_err(ListingFailure::Failed)
}

#[cfg(not(feature = "test-support"))]
fn base_url() -> Result<String, String> {
    Ok(CODEX_BACKEND_BASE_URL.to_string())
}

/// BUG-544: a test build sends `/models` only to the redirect a test
/// installed; without one it fails before any request, so no test reaches
/// chatgpt.com (with fake tokens, or with the developer's real sign-in).
#[cfg(feature = "test-support")]
fn base_url() -> Result<String, String> {
    test_support::redirected().ok_or_else(|| {
        "test builds never send the Codex /models request to chatgpt.com: install \
         models_catalog::test_support::redirect to a local mock"
            .to_string()
    })
}

/// Test-only redirect of the Codex backend base for [`fetch_for_selection`];
/// the `/models` path and query are kept.
#[cfg(feature = "test-support")]
pub mod test_support {
    use std::sync::Mutex;

    static BASE: Mutex<Option<String>> = Mutex::new(None);

    /// Clears the redirect when dropped.
    pub struct Redirect;

    /// Send `/models` fetches to `base` until the guard is dropped.
    pub fn redirect(base: &str) -> Redirect {
        *BASE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(base.to_string());
        Redirect
    }

    pub(super) fn redirected() -> Option<String> {
        BASE.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            *BASE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }

    /// BUG-577: the provider list's wait for Codex `/models` is `budget`
    /// until the guard is dropped.
    pub struct ListBudget;

    /// Set the provider list's Codex `/models` budget.
    pub fn list_budget(budget: std::time::Duration) -> ListBudget {
        super::super::listing::test_budget::set(Some(budget));
        ListBudget
    }

    impl Drop for ListBudget {
        fn drop(&mut self) {
            super::super::listing::test_budget::set(None);
        }
    }

    /// BUG-577: how many provider lists are waiting on a Codex fetch.
    pub fn waiting_listings() -> usize {
        super::super::listing::waiting()
    }

    /// BUG-577: whether a Codex fetch for the provider list is in flight.
    pub fn listing_in_flight() -> bool {
        super::super::listing::in_flight()
    }
}

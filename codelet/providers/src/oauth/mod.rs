//! Shared OAuth Building Blocks (PROV-060)
//!
//! Generic, reusable components for OAuth flows across all providers:
//! - `CredentialStore<T>` — generic credential file I/O
//! - `TokenStrategy` + `RefreshingHttpClient<S>` — token refresh middleware
//! - `DeviceCodeProvider` + `DeviceCodeFlow<P>` — RFC 8628 device flow
//! - `CodeExchangeHandler` + `OAuthCallbackServer<H>` — PKCE callback server
//! - Rhai engine + building blocks — scripted OAuth for custom providers
//! - `ScriptedOAuthProvider` — loads .rhai files for custom OAuth flows

pub mod building_blocks;
pub mod callback_server;
pub mod credential_store;
pub mod device_flow;
pub mod engine;
pub mod http_middleware;
pub mod script_provider;
pub mod token_refresh;

// Re-exports for ergonomic use
pub use building_blocks::register_all_modules;
pub use callback_server::{CodeExchangeHandler, OAuthCallbackServer};
pub use credential_store::CredentialStore;
pub use device_flow::{DeviceCodeFlow, DeviceCodeProvider};
pub use engine::build_sandboxed_engine;
pub use http_middleware::{RefreshingHttpClient, TokenStrategy};
pub use script_provider::ScriptedOAuthProvider;
pub use token_refresh::{ensure_fresh_token, TokenState};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

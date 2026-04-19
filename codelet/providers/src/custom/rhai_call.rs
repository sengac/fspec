//! Thin wrappers around `engine.call_fn` that offload execution to the
//! tokio blocking pool. Kept separate from `provider.rs` so the provider
//! module stays under 300 lines.

use std::sync::Arc;

use rhai::{Dynamic, Engine, Scope, AST};

use super::error_mapping::map_rhai_error_to_provider;
use crate::error::ProviderError;

/// Call a Rhai function with a single argument.
pub(crate) async fn call_fn1(
    engine: Arc<Engine>,
    ast: Arc<AST>,
    provider: String,
    fn_name: &'static str,
    arg: Dynamic,
) -> Result<Dynamic, ProviderError> {
    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn::<Dynamic>(&mut scope, &ast, fn_name, (arg,))
            .map_err(|e| map_rhai_error_to_provider(&provider, fn_name, &e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}

/// Call a Rhai function with two arguments.
pub(crate) async fn call_fn2(
    engine: Arc<Engine>,
    ast: Arc<AST>,
    provider: String,
    fn_name: &'static str,
    arg1: Dynamic,
    arg2: Dynamic,
) -> Result<Dynamic, ProviderError> {
    tokio::task::spawn_blocking(move || -> Result<Dynamic, ProviderError> {
        let mut scope = Scope::new();
        engine
            .call_fn::<Dynamic>(&mut scope, &ast, fn_name, (arg1, arg2))
            .map_err(|e| map_rhai_error_to_provider(&provider, fn_name, &e))
    })
    .await
    .map_err(|e| ProviderError::api("custom", format!("spawn_blocking join failed: {e}")))?
}

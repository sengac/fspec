//! PROV-087: Shared dispatcher for invoking Rhai functions on scripted
//! OAuth providers.
//!
//! Every method on [`ScriptedOAuthProvider`] that calls into a Rhai
//! script shares the same shape:
//!
//! 1. Clone the engine `Arc` + AST.
//! 2. `tokio::task::spawn_blocking` (Rhai is synchronous).
//! 3. `engine.call_fn(&mut scope, &ast, fn_name, args)`.
//! 4. Convert the result into a `Map` (or `bool`).
//!
//! The two helpers in this module capture that boilerplate once so
//! [`super::script_provider`] and [`super::script_provider_aliases`]
//! stay small and declarative.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use rhai::{Dynamic, Engine, FuncArgs, Map, Scope, AST};

/// Invoke a Rhai function and cast the return value to a [`Map`].
///
/// Runs the call inside `tokio::task::spawn_blocking` (Rhai is sync).
/// `args` is any type that implements [`FuncArgs`] — typically a
/// tuple like `(Dynamic,)`, `(Dynamic, String, String)`, or
/// `(Dynamic, Dynamic)`.
pub async fn call_script_map<A>(
    engine: Arc<Engine>,
    ast: AST,
    fn_name: &'static str,
    args: A,
) -> Result<Map>
where
    A: FuncArgs + Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<Map> {
        let mut scope = Scope::new();
        let result: Dynamic = engine
            .call_fn(&mut scope, &ast, fn_name, args)
            .map_err(|e| anyhow!("{fn_name} failed: {e}"))?;
        result
            .try_cast::<Map>()
            .ok_or_else(|| anyhow!("{fn_name} must return a Map"))
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
}

/// Invoke a Rhai function and cast the return value to `bool`.
///
/// Same shape as [`call_script_map`] but returns `bool` for
/// predicates like `auth_needs_refresh` / `needs_refresh`.
pub async fn call_script_bool<A>(
    engine: Arc<Engine>,
    ast: AST,
    fn_name: &'static str,
    args: A,
) -> Result<bool>
where
    A: FuncArgs + Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<bool> {
        let mut scope = Scope::new();
        let result: Dynamic = engine
            .call_fn(&mut scope, &ast, fn_name, args)
            .map_err(|e| anyhow!("{fn_name} failed: {e}"))?;
        result
            .as_bool()
            .map_err(|_| anyhow!("{fn_name} must return a bool"))
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
}

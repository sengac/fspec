//! Credential Management Module (NAPI shim)
//!
//! Lifted to `codelet-sessions` by **RPC-040**. The NAPI-free portion
//! (resolver, store, types) now lives in
//! `rust/sessions/src/credentials/`. This module re-exports from
//! `codelet_sessions::credentials` and keeps `napi_bindings.rs` so the
//! `#[napi] pub fn credentials_reload` symbol stays available to TS.

#[cfg(not(feature = "noop"))]
mod napi_bindings;

#[cfg(test)]
mod tests;

pub use codelet_sessions::credentials::*;

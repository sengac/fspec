//! NAPI Bindings for Credential Management
//!
//! SECURITY: We intentionally do NOT expose credential resolution to TypeScript.
//! Credentials stay in Rust - TypeScript only triggers reload after saving.

use napi::Result;
use napi_derive::napi;

/// Force reload credentials from disk
///
/// Call this after TypeScript saves credentials to ensure Rust picks up changes.
/// Returns true if credentials were reloaded (file changed), false otherwise.
///
/// SECURITY: This function does NOT return credentials - it only triggers a reload.
#[napi]
#[allow(dead_code)]
pub fn credentials_reload() -> Result<bool> {
    codelet_sessions::credentials::credentials_reload().map_err(napi::Error::from_reason)
}

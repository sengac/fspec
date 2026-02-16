//! NAPI Bindings for Credential Management
//!
//! SECURITY: We intentionally do NOT expose credential resolution to TypeScript.
//! Credentials stay in Rust - TypeScript only triggers reload after saving.

use napi::Result;
use napi_derive::napi;

use super::store::credentials_reload as internal_credentials_reload;

/// Force reload credentials from disk
///
/// Call this after TypeScript saves credentials to ensure Rust picks up changes.
/// Returns true if credentials were reloaded (file changed), false otherwise.
///
/// SECURITY: This function does NOT return credentials - it only triggers a reload.
#[napi]
#[allow(dead_code)] // This function is called via NAPI from TypeScript, not from Rust code
pub fn credentials_reload() -> Result<bool> {
    internal_credentials_reload().map_err(napi::Error::from_reason)
}

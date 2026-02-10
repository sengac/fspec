//! Credential Management Module
//!
//! Implements credential resolution for provider API keys with support for:
//! - Credentials file (~/.fspec/credentials/credentials.json)
//! - Environment variables
//! - Project .env files
//!
//! This module is the single source of truth for credential resolution.
//! TypeScript only saves/deletes credentials - Rust handles all resolution.

mod resolver;
mod store;
mod types;

#[cfg(not(feature = "noop"))]
mod napi_bindings;

#[cfg(test)]
mod tests;

pub use resolver::*;
pub use store::*;
pub use types::*;

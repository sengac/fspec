//! Credential Types
//!
//! Data structures for credential management matching the TypeScript CredentialsFile format.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Credential for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredential {
    pub api_key: String,
    pub last_updated: DateTime<Utc>,
}

/// Credentials file structure (matches TypeScript CredentialsFile)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsFile {
    pub version: u32,
    pub providers: HashMap<String, ProviderCredential>,
}

/// Source of a resolved credential
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// From ~/.fspec/credentials/credentials.json
    File,
    /// From environment variable
    Env,
    /// From .env file in project directory
    DotEnv,
}

/// Result of credential resolution
#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub api_key: String,
    pub source: CredentialSource,
}

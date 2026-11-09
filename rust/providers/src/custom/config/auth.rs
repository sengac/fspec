//! Authentication configuration variants for a custom provider (split from config.rs by REFAC-030).

use serde::{Deserialize, Serialize};

/// Default bearer `token_prefix` when not specified in JSON.
fn default_bearer_prefix() -> String {
    "Bearer".to_string()
}

/// Default API key header name.
fn default_api_key_header() -> String {
    "x-api-key".to_string()
}

/// Authentication configuration variants for a custom provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// Bearer token read from an environment variable.
    Bearer {
        /// Environment variable holding the bearer token.
        env_var: String,
        /// Authorization header prefix. Defaults to `"Bearer"`.
        #[serde(default = "default_bearer_prefix")]
        token_prefix: String,
    },
    /// API key placed in a custom HTTP header.
    ApiKeyHeader {
        /// Environment variable holding the API key.
        env_var: String,
        /// HTTP header name (default `x-api-key`).
        #[serde(default = "default_api_key_header")]
        header_name: String,
    },
    /// RFC 8628 device authorization flow.
    OauthDeviceCode {
        /// OAuth client_id.
        client_id: String,
        /// Device authorization endpoint.
        device_code_url: String,
        /// Token exchange endpoint.
        token_url: String,
        /// Optional space-separated scopes.
        #[serde(default)]
        scopes: Option<String>,
        /// Credential filename inside the fspec credentials dir.
        credential_file: String,
    },
    /// Authorization code with PKCE flow.
    OauthPkce {
        /// OAuth client_id.
        client_id: String,
        /// Authorization endpoint.
        authorize_url: String,
        /// Token exchange endpoint.
        token_url: String,
        /// Optional redirect URI (for non-loopback setups).
        #[serde(default)]
        redirect_uri: Option<String>,
        /// Optional space-separated scopes.
        #[serde(default)]
        scopes: Option<String>,
        /// Credential filename inside the fspec credentials dir.
        credential_file: String,
    },
    /// Fully custom — auth is handled by the Rhai script's
    /// `get_auth_headers()` function.
    Custom {
        /// Optional credential filename inside the fspec credentials dir.
        #[serde(default)]
        credential_file: Option<String>,
    },
}

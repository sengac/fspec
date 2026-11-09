//! MCP-004: configured HTTP headers for the Streamable HTTP MCP transport.
//!
//! Feature: spec/features/mcp-http-transport-headers.feature
//!
//! Errors name the offending header but never carry its value, and every
//! parsed value is marked sensitive so it is never printed by `http`.

use std::collections::HashMap;

use reqwest::header::{HeaderName, HeaderValue};

/// A configured header that is not a valid HTTP header.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpHttpHeaderError {
    #[error("HTTP header name '{0}' is invalid")]
    InvalidName(String),
    #[error("HTTP header '{0}' has an invalid value")]
    InvalidValue(String),
}

/// Parse configured `name: value` pairs into transport headers.
pub fn parse_mcp_http_headers<'a>(
    headers: impl IntoIterator<Item = (&'a String, &'a String)>,
) -> Result<HashMap<HeaderName, HeaderValue>, McpHttpHeaderError> {
    headers
        .into_iter()
        .map(|(name, value)| {
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| McpHttpHeaderError::InvalidName(name.clone()))?;
            let mut header_value = HeaderValue::from_str(value)
                .map_err(|_| McpHttpHeaderError::InvalidValue(name.clone()))?;
            header_value.set_sensitive(true);
            Ok((header_name, header_value))
        })
        .collect()
}

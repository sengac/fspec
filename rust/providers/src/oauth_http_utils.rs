//! Shared HTTP utilities for OAuth servers (Codex and Claude).
//!
//! Provides common functions used by both `codex_oauth_server.rs` and
//! `claude_oauth_server.rs` to avoid code duplication.

use std::collections::HashMap;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};

/// Build an HTML response with the given status and body.
pub fn html_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Internal Server Error"))))
}

/// Parse URL-encoded key-value pairs (query strings or form bodies).
///
/// Handles `&`-separated pairs with `=` delimiters and percent-decoding.
pub fn parse_urlencoded_params(input: &str) -> HashMap<String, String> {
    input
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or_default();
            Some((urlencoded_decode(key), urlencoded_decode(value)))
        })
        .collect()
}

/// Simple percent-decoding for URL-encoded values.
///
/// Handles `%XX` hex sequences and `+` as space.
pub fn urlencoded_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_urlencoded_params_basic() {
        let params = parse_urlencoded_params("code=abc123&state=xyz");
        assert_eq!(params.get("code"), Some(&"abc123".to_string()));
        assert_eq!(params.get("state"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_parse_urlencoded_params_encoded() {
        let params = parse_urlencoded_params("code=abc%23def");
        assert_eq!(params.get("code"), Some(&"abc#def".to_string()));
    }

    #[test]
    fn test_parse_urlencoded_params_empty() {
        let params = parse_urlencoded_params("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_urlencoded_decode_plus_as_space() {
        assert_eq!(urlencoded_decode("hello+world"), "hello world");
    }

    #[test]
    fn test_html_response_sets_content_type() {
        let resp = html_response(StatusCode::OK, "<h1>Test</h1>");
        assert_eq!(
            resp.headers()
                .get("Content-Type")
                .map(|v| v.to_str().unwrap_or_default()),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

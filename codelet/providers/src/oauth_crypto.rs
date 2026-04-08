//! Shared OAuth Cryptographic Primitives
//!
//! Provider-agnostic PKCE (RFC 7636) code challenge generation and
//! URL-encoding utilities used by both Codex and Claude OAuth flows.
//!
//! Extracted from `codex_oauth.rs` to eliminate cross-boundary imports
//! (Claude modules should not depend on Codex modules for generic crypto).

use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Characters allowed in PKCE code verifier per RFC 7636 Section 4.1
const PKCE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// PKCE code challenge and verifier pair
#[derive(Debug, Clone)]
pub struct PkceCodes {
    pub verifier: String,
    pub challenge: String,
    pub challenge_method: String,
}

impl PkceCodes {
    /// Create PkceCodes from an existing verifier (deterministic challenge derivation)
    pub fn from_verifier(verifier: String) -> Self {
        let challenge = compute_s256_challenge(&verifier);
        Self {
            verifier,
            challenge,
            challenge_method: "S256".to_string(),
        }
    }
}

/// Generate a new PKCE code verifier and S256 challenge per RFC 7636
///
/// The verifier is 43 characters of unreserved URI characters.
/// The challenge is the Base64URL-encoded SHA-256 hash of the verifier.
pub fn generate_pkce() -> PkceCodes {
    let verifier = generate_random_string(43);
    PkceCodes::from_verifier(verifier)
}

/// Generate a random string of the given length using unreserved URI characters
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..PKCE_CHARSET.len());
            PKCE_CHARSET[idx] as char
        })
        .collect()
}

/// Compute S256 code challenge: Base64URL(SHA-256(verifier))
fn compute_s256_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// Simple URL encoding for query parameters
pub fn urlencoded(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    result
}

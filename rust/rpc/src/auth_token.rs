//! RPC-447: optional bearer token a daemon requires on WebSocket handshakes.
//!
//! Feature: spec/features/daemon-bearer-auth.feature

use crate::{FspecServiceImpl, SharedFspecService};

/// MCP-003: the error when the daemon was started without a bearer token.
pub const MCP_CONFIGURATION_DISABLED: &str = "MCP server configuration is disabled: start the daemon with FSPEC_DAEMON_TOKEN set and connect with `Authorization: Bearer <token>`";

/// MCP-003: the error when this connection did not present the bearer.
pub const MCP_CONFIGURATION_UNAUTHENTICATED: &str = "MCP server configuration requires a connection authenticated with the daemon's FSPEC_DAEMON_TOKEN bearer";

impl SharedFspecService {
    /// Require `Authorization: Bearer <token>` on every WebSocket client.
    /// Only the first call takes effect; an empty token is ignored.
    pub fn set_auth_token(&self, token: String) {
        if !token.is_empty() {
            let _ = self.auth_token.set(token);
        }
    }

    /// Whether the daemon was armed with a bearer token (FSPEC_DAEMON_TOKEN).
    pub fn requires_bearer(&self) -> bool {
        self.auth_token.get().is_some()
    }

    /// Whether a handshake's `Authorization` header value is accepted:
    /// always when no token is configured, else only the exact bearer.
    pub fn authorizes(&self, authorization: Option<&str>) -> bool {
        match self.auth_token.get() {
            None => true,
            Some(token) => authorization
                .and_then(|value| value.strip_prefix("Bearer "))
                .is_some_and(|presented| constant_time_eq(presented.as_bytes(), token.as_bytes())),
        }
    }
}

impl FspecServiceImpl {
    /// MCP-003: configuring MCP servers needs a token-armed daemon AND a
    /// connection that authenticated with its bearer (no loopback trust).
    pub(crate) fn require_bearer_for_mcp_configuration(&self) -> Result<(), String> {
        if !self.inner.requires_bearer() {
            return Err(MCP_CONFIGURATION_DISABLED.to_string());
        }
        if !self.bearer_authenticated {
            return Err(MCP_CONFIGURATION_UNAUTHENTICATED.to_string());
        }
        Ok(())
    }
}

/// Byte comparison whose running time does not depend on where the inputs
/// first differ (lengths still leak, which a bearer check can afford).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

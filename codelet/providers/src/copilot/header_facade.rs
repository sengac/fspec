//! CopilotHeaderFacade — builds the required HTTP header set for every
//! outgoing Copilot API request.
//!
//! PROV-055: Rule 2 — every Copilot request must carry the following headers:
//!
//! | Header                     | Value                                                    |
//! |----------------------------|----------------------------------------------------------|
//! | `x-initiator`              | `"agent"` if `is_agent`, otherwise `"user"`              |
//! | `User-Agent`               | `"codelet/<CARGO_PKG_VERSION>"`                          |
//! | `Authorization`            | `"Bearer <access_token>"`                                |
//! | `Openai-Intent`            | `"conversation-edits"`                                   |
//! | `Copilot-Vision-Request`   | `"true"` *only if* `classification.is_vision`            |
//!
//! This module mirrors the header-building pattern used by
//! `CacheOptimizationFacade::build_headers` in
//! `codelet/providers/src/cache_optimization.rs:96`.

use crate::copilot::classifier::RequestClassification;
use http::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, USER_AGENT};

/// Header name: `x-initiator` (Copilot-specific — identifies whether the
/// request came from a user or an autonomous agent).
const HEADER_X_INITIATOR: HeaderName = HeaderName::from_static("x-initiator");

/// Header name: `openai-intent` (Copilot-specific — always
/// `conversation-edits` for codelet traffic).
const HEADER_OPENAI_INTENT: HeaderName = HeaderName::from_static("openai-intent");

/// Header name: `copilot-vision-request` (Copilot-specific — set only when
/// the outgoing body contains image parts).
const HEADER_COPILOT_VISION_REQUEST: HeaderName = HeaderName::from_static("copilot-vision-request");

/// Static header value: `conversation-edits`.
const OPENAI_INTENT_VALUE: HeaderValue = HeaderValue::from_static("conversation-edits");

/// Static header value: `true` (for `Copilot-Vision-Request`).
const TRUE_VALUE: HeaderValue = HeaderValue::from_static("true");

/// Static header value: `user` (for `x-initiator`).
const INITIATOR_USER: HeaderValue = HeaderValue::from_static("user");

/// Static header value: `agent` (for `x-initiator`).
const INITIATOR_AGENT: HeaderValue = HeaderValue::from_static("agent");

/// Facade that produces the Copilot header set for a single request.
///
/// # Example
///
/// ```
/// use codelet_providers::copilot::classifier::RequestClassification;
/// use codelet_providers::copilot::header_facade::CopilotHeaderFacade;
///
/// let classification = RequestClassification { is_vision: false, is_agent: false };
/// let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_token");
/// assert_eq!(headers.get("x-initiator").unwrap(), "user");
/// assert_eq!(headers.get("openai-intent").unwrap(), "conversation-edits");
/// assert!(headers.get("copilot-vision-request").is_none());
/// ```
pub struct CopilotHeaderFacade;

impl CopilotHeaderFacade {
    /// Build the full Copilot header set for a single outgoing request.
    ///
    /// # Arguments
    ///
    /// * `classification` - The classification derived from the request body
    ///   by [`CopilotRequestClassifier`](super::classifier::CopilotRequestClassifier).
    /// * `access_token` - The current OAuth access token (opaque, `ghu_*`).
    ///
    /// # Returns
    ///
    /// A [`HeaderMap`] containing all five required Copilot headers (four if
    /// the request is text-only — `Copilot-Vision-Request` is omitted in that
    /// case).
    #[must_use]
    pub fn build_headers(classification: &RequestClassification, access_token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // x-initiator: "agent" or "user"
        headers.insert(
            HEADER_X_INITIATOR,
            if classification.is_agent {
                INITIATOR_AGENT
            } else {
                INITIATOR_USER
            },
        );

        // User-Agent: codelet/<version>
        // We build this dynamically so cargo version bumps flow through
        // without requiring a manual header update.
        if let Ok(ua) = HeaderValue::from_str(&format!("codelet/{}", env!("CARGO_PKG_VERSION"))) {
            headers.insert(USER_AGENT, ua);
        }

        // Authorization: Bearer <access_token>
        if let Ok(auth) = HeaderValue::from_str(&format!("Bearer {access_token}")) {
            headers.insert(AUTHORIZATION, auth);
        }

        // Openai-Intent: conversation-edits
        headers.insert(HEADER_OPENAI_INTENT, OPENAI_INTENT_VALUE);

        // Copilot-Vision-Request: true (only when vision content is present)
        if classification.is_vision {
            headers.insert(HEADER_COPILOT_VISION_REQUEST, TRUE_VALUE);
        }

        headers
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn text_only_user_request_has_four_headers_no_vision() {
        let c = RequestClassification {
            is_vision: false,
            is_agent: false,
        };
        let h = CopilotHeaderFacade::build_headers(&c, "ghu_tok");
        assert_eq!(h.get("x-initiator").unwrap(), "user");
        assert!(h
            .get(USER_AGENT)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("codelet/"));
        assert_eq!(h.get(AUTHORIZATION).unwrap(), "Bearer ghu_tok");
        assert_eq!(h.get("openai-intent").unwrap(), "conversation-edits");
        assert!(h.get("copilot-vision-request").is_none());
    }

    #[test]
    fn vision_request_adds_vision_header() {
        let c = RequestClassification {
            is_vision: true,
            is_agent: false,
        };
        let h = CopilotHeaderFacade::build_headers(&c, "ghu_tok");
        assert_eq!(h.get("copilot-vision-request").unwrap(), "true");
    }

    #[test]
    fn agent_mode_sets_initiator_to_agent() {
        let c = RequestClassification {
            is_vision: false,
            is_agent: true,
        };
        let h = CopilotHeaderFacade::build_headers(&c, "ghu_tok");
        assert_eq!(h.get("x-initiator").unwrap(), "agent");
    }

    #[test]
    fn access_token_is_wrapped_with_bearer_prefix() {
        let c = RequestClassification::default();
        let h = CopilotHeaderFacade::build_headers(&c, "ghu_custom_xyz");
        assert_eq!(h.get(AUTHORIZATION).unwrap(), "Bearer ghu_custom_xyz");
    }
}

//! Session manifests and persistence facade shared by native entry points.

use chrono::{DateTime, Utc};
use codelet_common::token_estimator::count_tokens;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::persistence::messages::{MessageRef, MessageSource, MessageStore, StoredMessage};

/// Token usage tracking for a session.
///
/// CTX-003: distinguishes between current context size and cumulative
/// billing. The Anthropic API reports `input_tokens` as the TOTAL context
/// size per call (absolute, not incremental); display surfaces should use
/// `current_context_tokens`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TokenUsage {
    /// Current context size: the latest request's total input (input +
    /// cache read + cache creation, as the live tracker's `total_input`) —
    /// overwritten, not accumulated (BUG-326).
    #[serde(default)]
    pub current_context_tokens: u64,
    /// BUG-326: output + reasoning tokens of the request that measured
    /// `current_context_tokens` (zero after a compaction commit); with it the
    /// context's inclusive occupancy.
    #[serde(default)]
    pub current_context_output_tokens: u64,
    /// Cumulative billed input tokens (sum across all API calls).
    #[serde(default)]
    pub cumulative_billed_input: u64,
    /// Cumulative billed output tokens (sum across all API calls).
    #[serde(default)]
    pub cumulative_billed_output: u64,
    /// Cache read tokens from current API call.
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Cache creation tokens from current API call.
    #[serde(default)]
    pub cache_creation_tokens: u64,
    /// Session-cumulative reasoning tokens (TOKEN-003).
    ///
    /// STORED (not added) on each persist — the caller passes the
    /// session-wide 🧠 cumulative so /resume can re-seed the SessionHeader.
    #[serde(default)]
    pub reasoning_tokens: u64,
}

impl TokenUsage {
    /// BUG-326: inclusive occupancy of the current context (input plus the
    /// output of the request that measured it) — the same quantity as
    /// `RequestContextSnapshot::context_fill` for a live request.
    pub fn context_occupancy(&self) -> Option<u64> {
        self.current_context_tokens
            .checked_add(self.current_context_output_tokens)
    }

    /// BUG-326: the session's reply output without reasoning, which is what
    /// the live token tracker's `output_tokens` counts (reasoning is
    /// `reasoning_tokens`); `cumulative_billed_output` includes reasoning.
    pub fn reply_output_tokens(&self) -> Option<u64> {
        self.cumulative_billed_output
            .checked_sub(self.reasoning_tokens)
    }
}

include!("manifest_types.rs");
include!("manifest_store.rs");
include!("manifest_registry.rs");
include!("manifest_branches.rs");
include!("manifest_messages.rs");
include!("manifest_state.rs");

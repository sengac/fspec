//! RLCD-004 — security layer config section (`rlcd.security`).
//!
//! Feature: spec/features/rlcd-blocklist-security.feature
//!
//! Which ops the RLCD stage covers (`checkMode`), the probability
//! thresholds (`blockThreshold`, `promptThreshold` for Bash;
//! `fileBlockThreshold`, `filePromptThreshold` for Read/Write/Edit/
//! ApplyPatch) and the per-session latency guard
//! (`maxChecksPerSession`). All optional with calibrated defaults
//! (2026-09-24, live 'typed-decisions' backend, 390-item labeled battery —
//! see the RLCD-004 card rules 2/3); lenient deserialization (unknown keys
//! preserved by the load path).

use serde::{Deserialize, Serialize};

/// Default: stage every regex-allowed op.
pub fn default_check_mode() -> String {
    "all".to_string()
}

/// P(risk) at/above this hard-rejects (bash profile, rule id `rlcd-security`).
pub fn default_block_threshold() -> f64 {
    0.65
}

/// P(risk) at/above this asks the user (bash profile, Triple pause).
pub fn default_prompt_threshold() -> f64 {
    0.45
}

/// P(risk) at/above this hard-rejects (file profile — Read/Write/Edit/
/// ApplyPatch score on a compressed model scale, max ~0.36 in the battery).
pub fn default_file_block_threshold() -> f64 {
    0.35
}

/// P(risk) at/above this asks the user (file profile, Triple pause).
pub fn default_file_prompt_threshold() -> f64 {
    0.28
}

/// Per-session RLCD security checks before the stage skips (0 = unlimited).
pub fn default_max_checks_per_session() -> u32 {
    200
}

/// The effective (block, prompt) thresholds for one stage profile.
///
/// Pure data — the profile is picked at the call site (bash vs file) so the
/// stage itself stays threshold-agnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecurityThresholds {
    /// P(risk) at/above this -> hard reject.
    pub block: f64,
    /// P(risk) at/above this (and below `block`) -> Triple pause.
    pub prompt: f64,
}

impl SecurityThresholds {
    /// The Bash profile (0.65 / 0.45 by default).
    #[must_use]
    pub fn bash(cfg: &RlcdSecurityConfig) -> Self {
        Self {
            block: cfg.block_threshold,
            prompt: cfg.prompt_threshold,
        }
    }

    /// The file-operation profile (0.35 / 0.28 by default).
    #[must_use]
    pub fn file(cfg: &RlcdSecurityConfig) -> Self {
        Self {
            block: cfg.file_block_threshold,
            prompt: cfg.file_prompt_threshold,
        }
    }
}

/// Semantic security layer section (RLCD-004): the blocklist second stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RlcdSecurityConfig {
    /// `all` (default): every regex-allowed/prompt-allowed op is staged.
    /// `prompt-only`: only ops the regex pass already flagged prompt (and
    /// the user allowed) are staged — a latency saver. Any other value
    /// disables the stage (warned once at load).
    #[serde(default = "default_check_mode")]
    pub check_mode: String,
    /// P(risk) at/above this (bash) -> hard reject.
    #[serde(default = "default_block_threshold")]
    pub block_threshold: f64,
    /// P(risk) at/above this (bash) -> Triple pause.
    #[serde(default = "default_prompt_threshold")]
    pub prompt_threshold: f64,
    /// P(risk) at/above this (file ops) -> hard reject.
    #[serde(default = "default_file_block_threshold")]
    pub file_block_threshold: f64,
    /// P(risk) at/above this (file ops) -> Triple pause.
    #[serde(default = "default_file_prompt_threshold")]
    pub file_prompt_threshold: f64,
    /// Per-session check counter cap; 0 = unlimited. Above the cap the
    /// stage skips with a one-time warn.
    #[serde(default = "default_max_checks_per_session")]
    pub max_checks_per_session: u32,
}

impl Default for RlcdSecurityConfig {
    fn default() -> Self {
        Self {
            check_mode: default_check_mode(),
            block_threshold: default_block_threshold(),
            prompt_threshold: default_prompt_threshold(),
            file_block_threshold: default_file_block_threshold(),
            file_prompt_threshold: default_file_prompt_threshold(),
            max_checks_per_session: default_max_checks_per_session(),
        }
    }
}

//! RLCD-003 — semantic gate config section (`rlcd.gate`).
//!
//! Feature: spec/features/rlcd-semantic-gate-for-fspec-workflow-commands-agent-mode.feature
//!
//! Which fspec workflow commands the engine reviews, plus the decision
//! thresholds (all optional with defaults — lenient deserialization).

use serde::{Deserialize, Serialize};

// ============================================================================
// gate section (RLCD-003)
// ============================================================================

/// Default list of gated workflow commands.
pub fn default_gate_commands() -> Vec<String> {
    vec!["update-work-unit-status".to_string()]
}

/// Semantic gate section (RLCD-003): which fspec workflow commands the
/// engine reviews, plus the decision thresholds (all optional with
/// defaults — lenient deserialization).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RlcdGateConfig {
    /// Commands that pass through the gate (config-extensible).
    #[serde(default = "default_gate_commands")]
    pub commands: Vec<String>,
    /// P(hold) at/above this rejects (hold beats argmax).
    #[serde(default = "default_hold_threshold")]
    pub hold_threshold: f64,
    /// P(ask_user) at/above this asks the user (Triple pause).
    #[serde(default = "default_ask_threshold")]
    pub ask_threshold: f64,
    /// Minimum P(proceed)-max(others) margin for an auto-proceed.
    #[serde(default = "default_proceed_margin")]
    pub proceed_margin: f64,
}

impl Default for RlcdGateConfig {
    fn default() -> Self {
        Self {
            commands: default_gate_commands(),
            hold_threshold: default_hold_threshold(),
            ask_threshold: default_ask_threshold(),
            proceed_margin: default_proceed_margin(),
        }
    }
}

fn default_hold_threshold() -> f64 {
    0.65
}
fn default_ask_threshold() -> f64 {
    0.40
}
fn default_proceed_margin() -> f64 {
    0.15
}

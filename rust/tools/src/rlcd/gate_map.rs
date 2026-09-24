//! RLCD-003 — the gate's pure decision core (answer → outcome).
//!
//! Feature: spec/features/rlcd-semantic-gate-for-fspec-workflow-commands-agent-mode.feature
//!
//! `map_gate_answer` maps ONE gate answer (the answer's `probabilities`
//! object) onto a [`GateOutcome`] using the configured thresholds. Kept in
//! its own file (pure, no I/O) so the gate entry stays under the 300-line
//! limit (same split as decision.rs / decision_validate.rs in RLCD-002).

use serde_json::Value;

use crate::rlcd::gate_config::RlcdGateConfig;

/// The gate's decision for one command (before any pause interaction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// Execute via the normal fspec handler path.
    Proceed,
    /// Reject: the engine judged the timing wrong / risky.
    Hold { reason: String },
    /// Ask the user (Triple pause).
    AskUser { reason: String },
}

/// Read a probability from the answer's `probabilities` object (missing or
/// non-numeric values are 0.0 — never fail on a malformed answer).
fn prob(probs: &Value, key: &str) -> f64 {
    probs.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Map ONE gate answer (its `probabilities` object) onto a [`GateOutcome`].
///
/// Priority: hold > ask_user > proceed. Hold when argmax is hold OR
/// P(hold) >= holdThreshold. Otherwise AskUser when argmax is ask_user OR
/// P(ask_user) >= askThreshold. Otherwise Proceed only when argmax is
/// proceed AND the margin over the next criterion exceeds proceedMargin —
/// a thin-margin proceed (or any other shape) conservatively degrades to
/// AskUser; the gate never auto-proceeds on a low-confidence answer.
#[must_use]
pub fn map_gate_answer(probs: &Value, gate: &RlcdGateConfig) -> GateOutcome {
    let p_proceed = prob(probs, "proceed");
    let p_hold = prob(probs, "hold");
    let p_ask = prob(probs, "ask_user");
    let argmax = ["proceed", "hold", "ask_user"]
        .into_iter()
        .max_by(|a, b| prob(probs, a).total_cmp(&prob(probs, b)))
        .unwrap_or("proceed"); // static criterion list is never empty

    // (b) hold: argmax hold OR the threshold trips (beats every other branch)
    if argmax == "hold" || p_hold >= gate.hold_threshold {
        return GateOutcome::Hold {
            reason: format!(
                "P(hold)={p_hold} (argmax {argmax}; hold threshold {:.2}) — the engine judged this workflow command mistimed or risky",
                gate.hold_threshold
            ),
        };
    }
    // (c) ask_user: argmax ask_user OR the threshold trips
    if argmax == "ask_user" || p_ask >= gate.ask_threshold {
        return GateOutcome::AskUser {
            reason: format!(
                "P(ask_user)={p_ask} (argmax {argmax}; ask threshold {:.2}) — the engine wants user input",
                gate.ask_threshold
            ),
        };
    }
    // (a) proceed: only with a comfortable margin over the next criterion
    let margin = p_proceed - p_hold.max(p_ask);
    if argmax == "proceed" && margin > gate.proceed_margin {
        return GateOutcome::Proceed;
    }
    GateOutcome::AskUser {
        reason: format!(
            "P(proceed)={p_proceed} with margin {margin} (<= proceed margin {:.2}) — low-confidence proceed degrades to a user ask",
            gate.proceed_margin
        ),
    }
}

//! MODEL-015: Claude effort that changes between turns.
//!
//! The stream-loop driver hands every turn's agent to
//! [`configure_turn_effort`] once the history the turn is sent with is
//! known: the turn's effort message goes right before the new user message,
//! and the turn's requests keep the conversation's top-level effort.
//!
//! Feature: spec/features/claude-effort-can-change-between-turns.feature

use codelet_core::RigAgent;
use codelet_providers::claude_effort::EffortLog;
use rig::completion::CompletionModel;
use rig::message::Message;

/// The turn's agent: with the effort log's per-turn params when the turn is
/// a per-message turn, as built otherwise. The new user message goes right
/// after `history`.
pub fn configure_turn_effort<M: CompletionModel + 'static>(
    log: &mut EffortLog,
    agent: RigAgent<M>,
    history: &[Message],
) -> RigAgent<M> {
    match log.turn_params(agent.additional_params(), history, history.len()) {
        Some(params) => agent.with_turn_params(params, history),
        None => agent,
    }
}

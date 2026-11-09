//! MODEL-015: per-turn request params (Claude per-message effort).
//!
//! A turn's params apply to every request of the turn while the history
//! still starts with the history the turn was planned on. After a compaction
//! or any other edit of that history the requests go with the agent's built
//! params: the level in effect, at the top level.
//!
//! Feature: spec/features/claude-effort-can-change-between-turns.feature

use super::RigAgent;
use rig::agent::Agent;
use rig::completion::CompletionModel;
use rig::message::{AssistantContent, Message};
use rig::providers::anthropic::effort::EFFORT_MESSAGES_PARAM;
use serde_json::Value;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

/// The params of a turn and the history prefix they were planned on.
#[derive(Debug, Clone)]
pub(super) struct TurnParams {
    params: Value,
    prefix_len: usize,
    prefix: Option<u64>,
}

fn fingerprint(history: &[Message]) -> Option<u64> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for message in history {
        serde_json::to_vec(message).ok()?.hash(&mut hasher);
    }
    Some(hasher.finish())
}

/// PROV-143: an assistant message of only reasoning is left out of the
/// outgoing history when thinking is not preserved.
fn stripped(message: &Message) -> bool {
    matches!(message, Message::Assistant { content, .. }
        if content.iter().all(|block| matches!(block, AssistantContent::Reasoning(_))))
}

impl TurnParams {
    pub(super) fn new(params: Value, history: &[Message]) -> Self {
        Self {
            params,
            prefix_len: history.len(),
            prefix: fingerprint(history),
        }
    }

    /// The params for a request with `history`; `None` when the history no
    /// longer starts with the planned prefix.
    fn params_for(&self, history: &[Message], preserve_thinking: bool) -> Option<Value> {
        let prefix = history.get(..self.prefix_len)?;
        if self.prefix.is_none() || fingerprint(prefix) != self.prefix {
            return None;
        }
        let mut params = self.params.clone();
        if preserve_thinking {
            return Some(params);
        }
        if let Some(Value::Array(marks)) = params.get_mut(EFFORT_MESSAGES_PARAM) {
            for mark in marks {
                let Some(index) = mark
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|index| usize::try_from(index).ok())
                else {
                    continue;
                };
                let dropped = history
                    .get(..index.min(history.len()))
                    .map_or(0, |before| before.iter().filter(|m| stripped(m)).count());
                mark["index"] = Value::from(index.saturating_sub(dropped));
            }
        }
        Some(params)
    }
}

impl<M: CompletionModel + 'static> RigAgent<M> {
    /// The params the agent was built with (the turn's thinking parameters).
    pub fn additional_params(&self) -> Option<&Value> {
        self.agent.additional_params.as_ref()
    }

    /// MODEL-015: the turn's requests carry `params` instead of the built
    /// params while their history starts with `history`.
    pub fn with_turn_params(mut self, params: Value, history: &[Message]) -> Self {
        self.turn_params = Some(TurnParams::new(params, history));
        self
    }

    /// The agent a request with `history` is sent through.
    pub(super) fn agent_for(&self, history: &[Message]) -> Cow<'_, Agent<M>> {
        let Some(params) = self
            .turn_params
            .as_ref()
            .and_then(|turn| turn.params_for(history, self.preserve_thinking))
        else {
            return Cow::Borrowed(&self.agent);
        };
        let mut agent = self.agent.clone();
        agent.additional_params = Some(params);
        Cow::Owned(agent)
    }
}

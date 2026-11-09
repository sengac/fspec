//! MODEL-015: Claude effort that changes between turns of one conversation.
//!
//! On a Claude model that takes per-message effort (the explicit table
//! [`PER_MESSAGE_EFFORT_MODELS`], never a model-id pattern) with adaptive
//! thinking, a level changed between turns is sent as an effort-only message
//! right before the user's next message, and the top-level effort stays the
//! level the conversation started with, so the prompt cache is kept. Every
//! other change (thinking off, a budget, a model outside the table, another
//! provider) goes at the top level as before and starts a new base. The
//! level sent is the level chosen; nothing is remapped.
//!
//! Feature: spec/features/claude-effort-can-change-between-turns.feature

use rig::message::Message;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

pub use rig::providers::anthropic::effort::{EFFORT_MESSAGES_PARAM, PER_MESSAGE_EFFORT_BETA};

/// The Claude models that take per-message effort (Claude API model
/// migration guide, "Per-message effort"; MODEL-015 research §1).
pub const PER_MESSAGE_EFFORT_MODELS: [&str; 6] = [
    "claude-fable-5-1",
    "claude-mythos-5-1",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
    "claude-haiku-5-5",
];

/// Whether the Claude API model `api_model` takes per-message effort.
pub fn takes_per_message_effort(api_model: &str) -> bool {
    PER_MESSAGE_EFFORT_MODELS.contains(&api_model)
}

/// The named effort of an adaptive-thinking Claude turn; `None` for any
/// other thinking form (off, a budget, nothing).
fn adaptive_effort(config: Option<&Value>) -> Option<String> {
    let config = config?;
    if config.pointer("/thinking/type").and_then(Value::as_str) != Some("adaptive") {
        return None;
    }
    config
        .pointer("/output_config/effort")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// An effort message: it precedes the user message `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortMark {
    message: Uuid,
    effort: String,
    /// The history index of `message`, once it is in the history.
    #[serde(skip)]
    index: Option<usize>,
    /// A fingerprint of the message at `index`, to notice an edited history.
    #[serde(skip)]
    anchor: Option<u64>,
}

impl EffortMark {
    pub fn message(&self) -> Uuid {
        self.message
    }

    pub fn effort(&self) -> &str {
        &self.effort
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }
}

/// A conversation's effort log: the top-level effort its requests carry and
/// the effort messages placed in it since then.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffortLog {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    marks: Vec<EffortMark>,
    /// Whether the planned turn renders the log (top-level effort = base).
    #[serde(skip)]
    per_message_turn: bool,
}

impl PartialEq for EffortLog {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.marks == other.marks
    }
}

fn fingerprint(message: &Message) -> Option<u64> {
    let bytes = serde_json::to_vec(message).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    Some(hasher.finish())
}

impl EffortLog {
    /// The top-level effort of a per-message conversation.
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    pub fn marks(&self) -> &[EffortMark] {
        &self.marks
    }

    /// Plans an admitted turn whose user message is `message`, sent with the
    /// thinking parameters `config`. Returns whether the persisted log
    /// changed. A level change becomes an effort message only on a
    /// per-message model with adaptive thinking, for fresh user input
    /// (MODEL-015 decision: a change applies from the user's next message);
    /// anything else re-bases the log and the turn goes at the top level.
    pub fn plan_turn(
        &mut self,
        per_message: bool,
        config: Option<&Value>,
        message: Uuid,
        fresh_input: bool,
    ) -> bool {
        let before = self.clone();
        // A mark whose turn never reached the provider was never sent.
        self.marks.retain(|mark| mark.index.is_some());
        let effort = adaptive_effort(config);
        match (per_message, effort, self.base.clone()) {
            (true, Some(effort), Some(base)) => {
                let in_effect = self.marks.last().map_or(base, |mark| mark.effort.clone());
                if fresh_input && effort != in_effect {
                    self.marks.push(EffortMark {
                        message,
                        effort,
                        index: None,
                        anchor: None,
                    });
                }
                self.per_message_turn = true;
            }
            (per_message, effort, _) => {
                self.base = effort.filter(|_| per_message);
                self.marks.clear();
                self.per_message_turn = false;
            }
        }
        before != *self
    }

    /// The params the turn's requests carry instead of the agent's `built`
    /// params, with the turn's user message going at `prompt_index` of
    /// `history`; `None` when the turn goes as built (at the top level). An
    /// earlier history that no longer holds the marked messages (compacted
    /// or edited) re-bases the log, and the turn goes as built.
    pub fn turn_params(
        &mut self,
        built: Option<&Value>,
        history: &[Message],
        prompt_index: usize,
    ) -> Option<Value> {
        if !self.per_message_turn {
            return None;
        }
        let base = self.base.clone()?;
        for mark in &mut self.marks {
            let Some(index) = mark.index else {
                mark.index = Some(prompt_index);
                continue;
            };
            let current = history
                .get(index)
                .filter(|_| index < prompt_index)
                .and_then(fingerprint);
            match (current, mark.anchor) {
                (Some(current), Some(anchor)) if current == anchor => {}
                (Some(current), None) => mark.anchor = Some(current),
                _ => {
                    self.reset();
                    return None;
                }
            }
        }
        let mut params = built.cloned().unwrap_or_else(|| json!({}));
        let object = params.as_object_mut()?;
        match object
            .get_mut("output_config")
            .and_then(Value::as_object_mut)
        {
            Some(output_config) => {
                output_config.insert("effort".into(), json!(base));
            }
            None => {
                object.insert("output_config".into(), json!({"effort": base}));
            }
        }
        if !self.marks.is_empty() {
            let marks = self
                .marks
                .iter()
                .filter_map(|mark| Some(json!({"index": mark.index?, "effort": mark.effort})))
                .collect::<Vec<_>>();
            object.insert(EFFORT_MESSAGES_PARAM.into(), Value::Array(marks));
        }
        Some(params)
    }

    /// Forgets the log (after a compaction): the next turn declares the
    /// level in effect at the top level.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The persisted log of a restored conversation whose user messages
    /// are at `positions` of the restored history. A mark whose message is
    /// not in the restored history is not carried over.
    pub fn restore(record: Option<EffortLog>, positions: &HashMap<Uuid, usize>) -> Self {
        let Some(record) = record else {
            return Self::default();
        };
        let mut marks = record
            .marks
            .into_iter()
            .filter_map(|mark| {
                let index = *positions.get(&mark.message)?;
                Some(EffortMark {
                    index: Some(index),
                    anchor: None,
                    ..mark
                })
            })
            .collect::<Vec<_>>();
        marks.sort_by_key(|mark| mark.index);
        Self {
            base: record.base,
            marks,
            per_message_turn: false,
        }
    }
}

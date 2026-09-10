//! BUG-170: Surgical removal of a failed tool-call pair from the context
//! stack after an unrecoverable (prompt-too-long) terminal API error.
//!
//! When the next API call after a tool result fails with a context-length
//! error, the offending `Assistant(ToolCall)` + `User(ToolResult)` pair is
//! still in `session.messages`. Without removal, the next user message
//! replays the same oversized tool call and the same error recurs every
//! turn. This module provides the tail-surgery primitive; the stream loop's
//! terminal error arm invokes it (only for prompt-too-long errors) and then
//! invalidates the stale cache-token state at the call site.

use crate::interactive_helpers::tool_call_correlation_key;
use crate::session::system_reminders::is_system_reminder;
use rig::message::{AssistantContent, Message, UserContent};
use rig::one_or_many::OneOrMany;
use tracing::debug;

/// Which tail shape [`strip_failed_tool_call_tail`] removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrippedTail {
    /// Popped a `User(ToolResult)` message AND removed its matching
    /// `ToolCall` item from the preceding `Assistant` message.
    ToolPair,
    /// Popped a trailing `Assistant` message whose items were all
    /// `ToolCall` (the tool never produced a result).
    ToolCallOnly,
    /// Popped a trailing plain `User` prompt (last-resort fallback).
    TrailingUser,
    /// Tail did not match any removable shape — nothing was removed.
    None,
}

/// BUG-170: Surgically remove the failed tool-call pair from the tail of
/// the message stack.
///
/// Tail shapes handled (in priority order):
/// 1. `… User(ToolResult)` — pop the ToolResult message, then remove the
///    matching `ToolCall` item from the preceding `Assistant` message
///    (keeping any `Text`/`Reasoning`/`Image` items; dropping the whole
///    `Assistant` message if it becomes empty). Returns [`StrippedTail::ToolPair`].
/// 2. `… Assistant` whose items are all `ToolCall` — pop it.
///    Returns [`StrippedTail::ToolCallOnly`].
/// 3. `… User` (plain prompt or mixed content) — pop it (existing
///    `begin_compaction_recovery` semantics). Returns [`StrippedTail::TrailingUser`].
///
/// Guarantees:
/// - System reminder messages are never removed.
/// - If the tail matches no removable shape (e.g. an `Assistant` with
///   trailing text from an interrupted turn), nothing is removed and
///   [`StrippedTail::None`] is returned.
/// - The postcondition is checked by the caller via
///   `validate_no_orphan_tool_calls`; removal only ever *resolves* orphans
///   (it removes a call together with its result, or removes a result whose
///   call is removed in the same operation).
pub fn strip_failed_tool_call_tail(messages: &mut Vec<Message>) -> StrippedTail {
    if messages.is_empty() {
        return StrippedTail::None;
    }

    // Case 1: trailing User whose content is (only) ToolResult(s).
    if let Some(last) = messages.last() {
        if let Message::User { content } = last {
            let items: Vec<_> = content.iter().collect();
            if !items.is_empty()
                && items
                    .iter()
                    .all(|i| matches!(i, UserContent::ToolResult(_)))
                && !is_system_reminder(last)
            {
                let correlation_keys: Vec<String> = items
                    .iter()
                    .filter_map(|i| {
                        if let UserContent::ToolResult(tr) = i {
                            Some(tool_call_correlation_key(
                                &tr.id,
                                tr.call_id.as_deref(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                let removed_result = messages.pop();
                debug!(
                    "BUG-170: popped trailing User(ToolResult) ({} result(s))",
                    correlation_keys.len()
                );
                remove_matching_tool_calls(messages, &correlation_keys);
                let _ = removed_result;
                return StrippedTail::ToolPair;
            }
            // Case 3: trailing plain User prompt (or mixed content).
            if !is_system_reminder(last) {
                messages.pop();
                debug!("BUG-170: popped trailing plain User prompt");
                return StrippedTail::TrailingUser;
            }
        }
    }

    // Case 2: trailing Assistant whose items are all ToolCall.
    if let Some(last) = messages.last() {
        if let Message::Assistant { content, .. } = last {
            let items: Vec<_> = content.iter().collect();
            if !items.is_empty()
                && items
                    .iter()
                    .all(|i| matches!(i, AssistantContent::ToolCall(_)))
                && !is_system_reminder(last)
            {
                messages.pop();
                debug!("BUG-170: popped trailing tool-call-only Assistant message");
                return StrippedTail::ToolCallOnly;
            }
        }
    }

    StrippedTail::None
}

/// Remove `ToolCall` items from the nearest preceding `Assistant` message
/// whose correlation keys appear in `keys`.
///
/// If the `Assistant` message becomes empty after filtering, the whole
/// message is dropped. Non-Assistant messages between the tail and the
/// preceding `Assistant` (rare/invalid stacks) leave that message untouched.
fn remove_matching_tool_calls(messages: &mut Vec<Message>, keys: &[String]) {
    if keys.is_empty() {
        return;
    }
    let Some(prev_idx) = messages
        .iter()
        .rposition(|m| matches!(m, Message::Assistant { .. }))
    else {
        return;
    };

    let kept_items: Vec<AssistantContent> = match &messages[prev_idx] {
        Message::Assistant { content, .. } => content
            .iter()
            .filter(|item| match item {
                AssistantContent::ToolCall(tc) => {
                    let key = tool_call_correlation_key(&tc.id, tc.call_id.as_deref());
                    !keys.contains(&key)
                }
                _ => true,
            })
            .cloned()
            .collect(),
        _ => return,
    };

    if kept_items.is_empty() {
        messages.remove(prev_idx);
        debug!("BUG-170: dropped Assistant message that became empty after tool-call removal");
    } else if let Ok(new_content) = OneOrMany::many(kept_items) {
        if let Message::Assistant { content, .. } = &mut messages[prev_idx] {
            *content = new_content;
            debug!("BUG-170: stripped tool-call item(s) from preceding Assistant message");
        }
    }
}

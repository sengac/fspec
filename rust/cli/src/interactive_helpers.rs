//! Helper functions for interactive mode compaction (CLI-010)

use crate::session::Session;
use anyhow::Result;
use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::{
    ConversationTurn, ToolCall as CoreToolCall, ToolResult as CoreToolResult,
};
use rig::message::{AssistantContent, Message, ToolResultContent, UserContent};
use rig::OneOrMany;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, warn};

/// Convert messages to conversation turns using lazy approach (following TypeScript implementation)
///
/// This follows the TypeScript implementation in compaction.ts:100-141
/// - Forward iteration through message pairs
/// - Content extraction including tool calls and results
/// - Token estimation matching TypeScript (sum of user + assistant)
/// - No complex backward iteration or content extraction failures
pub fn convert_messages_to_turns(messages: &[Message]) -> Vec<ConversationTurn> {
    let mut turns = Vec::new();

    // Forward iteration through message pairs (like TypeScript)
    let mut i = 0;
    while i < messages.len() {
        if let Some(user_msg) = messages.get(i) {
            if matches!(user_msg, Message::User { .. }) {
                if let Some(assistant_msg) = messages.get(i + 1) {
                    if matches!(assistant_msg, Message::Assistant { .. }) {
                        // Extract text content
                        let user_text = extract_message_text(user_msg);
                        let assistant_text = extract_message_text(assistant_msg);

                        // Extract tool calls from assistant message
                        let tool_calls = extract_tool_calls(assistant_msg);

                        // Extract tool results from user message (tool results appear in next user message)
                        let tool_results = extract_tool_results(user_msg);

                        // Calculate tokens like TypeScript: userMsg.tokens + assistantMsg.tokens
                        // PROV-002: Use tiktoken-rs for accurate token counting
                        let user_tokens = count_tokens(&user_text) as u64;
                        let assistant_tokens = count_tokens(&assistant_text) as u64;
                        let total_tokens = user_tokens + assistant_tokens;

                        // Create turn with full content extraction
                        turns.push(ConversationTurn {
                            user_message: user_text,
                            tool_calls,
                            tool_results,
                            assistant_response: assistant_text,
                            tokens: total_tokens, // Match TypeScript: sum of user + assistant tokens
                            timestamp: SystemTime::now(),
                            previous_error: None,
                        });

                        i += 2; // Skip both messages (like TypeScript's i++)
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    turns
}

/// Extract tool calls from an assistant message
fn extract_tool_calls(message: &Message) -> Vec<CoreToolCall> {
    use rig::message::AssistantContent;

    let Message::Assistant { content, .. } = message else {
        return vec![];
    };

    collect_items(content)
        .into_iter()
        .filter_map(|item| match item {
            AssistantContent::ToolCall(tc) => Some(CoreToolCall {
                tool: tc.function.name.clone(),
                id: tc.id.clone(),
                parameters: tc.function.arguments,
            }),
            _ => None,
        })
        .collect()
}

/// Extract tool results from a user message
fn extract_tool_results(message: &Message) -> Vec<CoreToolResult> {
    let Message::User { content } = message else {
        return vec![];
    };

    collect_items(content)
        .into_iter()
        .filter_map(|item| match item {
            UserContent::ToolResult(tr) => Some(CoreToolResult {
                success: true, // Assume success if present (errors would have different handling)
                output: extract_tool_result_text(&tr),
                error: None,
            }),
            _ => None,
        })
        .collect()
}

/// Extract text content from a tool result
fn extract_tool_result_text(tr: &rig::message::ToolResult) -> String {
    use rig::message::ToolResultContent;

    // Check for single text item (fast path)
    if tr.content.rest().is_empty() {
        if let ToolResultContent::Text(ref t) = tr.content.first() {
            return t.text.clone();
        }
    }

    // Multiple items: collect and serialize
    let items = collect_items(&tr.content);
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

/// Extract/serialize message content to string (matches TypeScript toCompactionMessages)
///
/// TypeScript logic (runner.ts:487-489):
///   contentString = typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content)
///
/// Rust equivalent:
///   - Single text item → extract just the text (like TS string content)
///   - Multiple items OR non-text items → serialize as JSON (like TS JSON.stringify)
pub fn extract_message_text(message: &Message) -> String {
    match message {
        Message::User { content } => {
            // Check if single text item (equivalent to TypeScript string content)
            if content.rest().is_empty() {
                if let UserContent::Text(t) = content.first() {
                    return t.text;
                }
            }
            // Multiple items or non-text: serialize as JSON (like TypeScript JSON.stringify)
            let items = collect_items(content);
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        }
        Message::Assistant { content, .. } => {
            use rig::message::AssistantContent;
            // Check if single text item (equivalent to TypeScript string content)
            if content.rest().is_empty() {
                if let AssistantContent::Text(t) = content.first() {
                    return t.text;
                }
            }
            // Multiple items or non-text: serialize as JSON (like TypeScript JSON.stringify)
            let items = collect_items(content);
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

/// Helper to collect all items from OneOrMany
pub fn collect_items<T: Clone>(content: &OneOrMany<T>) -> Vec<T> {
    let mut items = vec![content.first()];
    items.extend(content.rest());
    items
}

/// Calculate compression ratio from pre/post compaction token counts.
///
/// Returns a value clamped to [0.0, 1.0] representing the fraction of
/// tokens removed. E.g. 0.7 means 70% of tokens were eliminated.
///
/// CMPCT-039: the result is clamped in THIS helper so no caller can ever
/// receive a negative ratio. When `compacted_tokens > original_tokens`
/// (tiny sessions where surviving reminders plus the injected compaction
/// instruction exceed the original context) the ratio is 0.0 — context
/// growth stays recoverable from the original/compacted token fields that
/// the CompactionComplete producers ship alongside the ratio.
/// `original_tokens == 0` also yields 0.0 via the division guard.
///
/// Used by stream_loop (pre-prompt and post-loop compaction) and both
/// inject_summary_handler twins. RPC-421: the compact_session RPC twins
/// and repl_loop no longer call it — their trough measurement is an
/// acknowledgement, not a reduction, so they ship the 0.0 sentinel
/// directly.
pub fn compression_ratio(original_tokens: u64, compacted_tokens: u64) -> f64 {
    if original_tokens > 0 {
        (1.0 - (compacted_tokens as f64 / original_tokens as f64)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Recalculate token tracker from current session messages.
///
/// Iterates all messages, counts tokens via `count_tokens()`, and updates
/// the session's `token_tracker` (`input_tokens` = sum, `output_tokens` = 0).
///
/// Used by both `execute_compaction()` and the `inject_summary` handler
/// after clearing and reconstructing the message list.
pub fn recalculate_token_tracker(session: &mut Session) {
    let total_tokens: u64 = session
        .messages
        .iter()
        .map(|msg| {
            let text = extract_message_text(msg);
            count_tokens(&text) as u64
        })
        .sum();

    session.token_tracker.input_tokens = total_tokens;
    session.token_tracker.output_tokens = 0;
}

/// Reset a session to only its system reminders, clearing all conversation.
///
/// This is the shared "partition → clear → restore → clear turns" pattern used
/// by both `execute_compaction()` and the `inject_summary` handler.
///
/// Steps:
/// 1. Partition messages into system reminders and compactable conversation
/// 2. Clear all messages
/// 3. Restore system reminders
/// 4. Clear turns
///
/// Returns `(system_reminder_count, compactable_count)` for logging.
///
/// After calling this, the caller should:
/// - Push their specific message (compaction instruction or DAG content)
/// - Call `recalculate_token_tracker(session)`
pub fn reset_session_to_reminders(session: &mut Session) -> (usize, usize) {
    use crate::session::system_reminders::partition_for_compaction;

    let (system_reminders, compactable) = partition_for_compaction(&session.messages);
    let counts = (system_reminders.len(), compactable.len());

    session.messages.clear();
    session.messages.extend(system_reminders);
    session.turns.clear();

    counts
}

/// CMPCT-029: Correlation key used to match Assistant(ToolCall) with its
/// matching User(ToolResult).
///
/// Providers disagree on which field carries the correlation identity:
/// - Anthropic uses `call_id` (the `toolu_*` prefix — `id` is separate).
/// - OpenAI/Gemini fold both into `id` and leave `call_id` as `None`.
///
/// Rig's natural-exit flush at `streaming.rs:616-633` reflects this split:
/// when `call_id` is Some, the ToolResult is constructed via
/// `UserContent::tool_result_with_call_id(&id, call_id.clone(), ...)`. When
/// `call_id` is None, it falls back to `UserContent::tool_result(&id, ...)`.
/// In both cases the **pairing key** on the resulting ToolResult is
/// `call_id.unwrap_or(id)`. We mirror that here.
fn tool_call_correlation_key(id: &str, call_id: Option<&str>) -> String {
    match call_id {
        Some(cid) => cid.to_string(),
        None => id.to_string(),
    }
}

/// CMPCT-029: Walk `messages` and collect the correlation keys of every
/// Assistant(ToolCall) that does NOT have a matching User(ToolResult).
///
/// Returns `Ok(())` when every tool_call has a matching result; returns
/// `Err(Vec<String>)` with the orphan call_ids (in message order) otherwise.
///
/// Used as a defensive guard at the start of [`execute_compaction`]. Any
/// orphan tool_call would cause the next API request (post-compaction) to
/// fail with a "tool_use block must be followed by tool_result" error on
/// Anthropic, or silently produce inconsistent history on OpenAI. Catching
/// the orphan here — and having the caller resolve it via
/// [`inject_synthetic_tool_results_for_orphans`] — makes the contract
/// auditable instead of relying on every recovery path doing the right
/// thing.
///
/// The detector never mutates `messages`; it only reads.
pub fn validate_no_orphan_tool_calls(messages: &[Message]) -> std::result::Result<(), Vec<String>> {
    let mut pending: Vec<String> = Vec::new();
    let mut seen_results: HashSet<String> = HashSet::new();

    for msg in messages {
        match msg {
            Message::Assistant { content, .. } => {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(tc) = item {
                        let key = tool_call_correlation_key(&tc.id, tc.call_id.as_deref());
                        pending.push(key);
                    }
                }
            }
            Message::User { content } => {
                for item in content.iter() {
                    if let UserContent::ToolResult(tr) = item {
                        let key = tool_call_correlation_key(&tr.id, tr.call_id.as_deref());
                        seen_results.insert(key);
                    }
                }
            }
        }
    }

    let orphans: Vec<String> = pending
        .into_iter()
        .filter(|k| !seen_results.contains(k))
        .collect();

    if orphans.is_empty() {
        Ok(())
    } else {
        Err(orphans)
    }
}

/// CMPCT-029: Append any tool_call / tool_result messages from rig's
/// `chat_history` (delivered via `PromptError::PromptCancelled`) that fspec's
/// `session_messages` does not yet contain.
///
/// The rig-side patch at `streaming.rs` cancel site 508 flushes pending
/// tool pairs into `chat_history` before yielding PromptCancelled. This
/// helper drains those pairs into fspec's own message list so the next
/// compaction pass sees the same conversation state as the provider API.
///
/// Dedupe strategy: two messages are considered the same when:
/// - Both are Assistant with matching tool_call correlation keys, OR
/// - Both are User with matching tool_result correlation keys.
///
/// Messages that don't carry tool state are never appended by this helper —
/// fspec already tracks assistant text / user text through the stream
/// handlers, so forwarding text messages from rig would produce duplicates.
///
/// This is idempotent: calling it twice with the same arguments produces
/// the same result as calling it once.
pub fn reconcile_session_messages(
    session_messages: &mut Vec<Message>,
    rig_chat_history: &[Message],
) {
    // Build the set of tool correlation keys fspec already tracks. We track
    // both "calls we've seen" and "results we've seen" separately because
    // it is possible — though rare — for fspec to hold the call but not the
    // result (site 486 recovery drains `tool_calls_buffer` into
    // `session.messages`, producing exactly this shape).
    let mut known_calls: HashSet<String> = HashSet::new();
    let mut known_results: HashSet<String> = HashSet::new();

    for msg in session_messages.iter() {
        match msg {
            Message::Assistant { content, .. } => {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(tc) = item {
                        known_calls
                            .insert(tool_call_correlation_key(&tc.id, tc.call_id.as_deref()));
                    }
                }
            }
            Message::User { content } => {
                for item in content.iter() {
                    if let UserContent::ToolResult(tr) = item {
                        known_results
                            .insert(tool_call_correlation_key(&tr.id, tr.call_id.as_deref()));
                    }
                }
            }
        }
    }

    for msg in rig_chat_history {
        match msg {
            Message::Assistant { content, .. } => {
                // Filter the content to just the ToolCall items we don't
                // already have. Rig may have other content types in the
                // same message (reasoning, text) — we skip those because
                // fspec tracks them through separate stream events.
                let new_calls: Vec<AssistantContent> = content
                    .iter()
                    .filter_map(|item| match item {
                        AssistantContent::ToolCall(tc) => {
                            let key = tool_call_correlation_key(&tc.id, tc.call_id.as_deref());
                            if known_calls.contains(&key) {
                                None
                            } else {
                                known_calls.insert(key);
                                Some(AssistantContent::ToolCall(tc.clone()))
                            }
                        }
                        _ => None,
                    })
                    .collect();

                if !new_calls.is_empty() {
                    match OneOrMany::many(new_calls) {
                        Ok(merged) => {
                            session_messages.push(Message::Assistant {
                                id: None,
                                content: merged,
                            });
                        }
                        Err(e) => {
                            warn!(
                                "[reconcile_session_messages] could not rebuild OneOrMany for Assistant tool_calls: {e}"
                            );
                        }
                    }
                }
            }
            Message::User { content } => {
                let new_results: Vec<UserContent> = content
                    .iter()
                    .filter_map(|item| match item {
                        UserContent::ToolResult(tr) => {
                            let key = tool_call_correlation_key(&tr.id, tr.call_id.as_deref());
                            if known_results.contains(&key) {
                                None
                            } else {
                                known_results.insert(key);
                                Some(UserContent::ToolResult(tr.clone()))
                            }
                        }
                        _ => None,
                    })
                    .collect();

                if !new_results.is_empty() {
                    match OneOrMany::many(new_results) {
                        Ok(merged) => {
                            session_messages.push(Message::User { content: merged });
                        }
                        Err(e) => {
                            warn!(
                                "[reconcile_session_messages] could not rebuild OneOrMany for User tool_results: {e}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// CMPCT-029: Synthetic tool_result body injected for orphan tool_calls
/// when the provider stream was cancelled before the tool could run
/// (site 486) or before the result could be delivered to fspec.
///
/// The agent sees this body in the next turn's context, so it reads the
/// `"cancelled_by_context_limit"` marker as a deliberate cancellation
/// rather than a silent omission. Keeping the payload as structured JSON
/// means future recovery layers can machine-detect it without grepping
/// free-form text.
pub const SYNTHETIC_TOOL_CANCEL_BODY: &str = r#"{"status":"cancelled_by_context_limit"}"#;

/// CMPCT-029: Close every orphan Assistant(ToolCall) in `session_messages`
/// with a synthetic User(ToolResult) carrying the
/// `"cancelled_by_context_limit"` body.
///
/// Applies the minimum mutation required to restore tool-pair invariants:
/// - For each orphan tool_call (no matching tool_result anywhere in the
///   message list), appends a single User message whose ToolResult reuses
///   the original tool call's `id` and `call_id`.
/// - Never removes or reorders any existing message.
/// - Returns the number of synthetic injections performed, so callers can
///   log a structured warning that encodes "recovery happened" without
///   having to re-inspect the orphan detector's output.
///
/// Callers in the stream-loop's compaction-cancel branch invoke this AFTER
/// [`reconcile_session_messages`] has folded in any rig-side tool state.
/// Any orphans that remain at that point are site-486 cancellations where
/// rig yielded before the tool ran — there is no real result to preserve,
/// so we synthesize one with the cancel marker.
pub fn inject_synthetic_tool_results_for_orphans(session_messages: &mut Vec<Message>) -> usize {
    // Collect the orphan call_ids / ids together so we can reuse the
    // correlation identity when constructing the synthetic result.
    let mut orphans: Vec<(String, Option<String>)> = Vec::new();
    let mut seen_results: HashSet<String> = HashSet::new();

    for msg in session_messages.iter() {
        if let Message::User { content } = msg {
            for item in content.iter() {
                if let UserContent::ToolResult(tr) = item {
                    seen_results.insert(tool_call_correlation_key(&tr.id, tr.call_id.as_deref()));
                }
            }
        }
    }

    for msg in session_messages.iter() {
        if let Message::Assistant { content, .. } = msg {
            for item in content.iter() {
                if let AssistantContent::ToolCall(tc) = item {
                    let key = tool_call_correlation_key(&tc.id, tc.call_id.as_deref());
                    if !seen_results.contains(&key) {
                        orphans.push((tc.id.clone(), tc.call_id.clone()));
                    }
                }
            }
        }
    }

    let injected = orphans.len();
    for (id, call_id) in orphans {
        let body = OneOrMany::one(ToolResultContent::Text(rig::message::Text {
            text: SYNTHETIC_TOOL_CANCEL_BODY.to_string(),
        }));
        let tool_result = match call_id {
            Some(cid) => UserContent::tool_result_with_call_id(&id, cid, body),
            None => UserContent::tool_result(&id, body),
        };
        session_messages.push(Message::User {
            content: OneOrMany::one(tool_result),
        });
    }

    if injected > 0 {
        warn!(
            injected,
            "[inject_synthetic_tool_results_for_orphans] Injected synthetic cancelled tool_results for orphan tool_calls"
        );
    }

    injected
}

/// Execute in-view DAG construction compaction.
///
/// In-view DAG compaction flow:
/// 1. Set compaction_in_progress flag to true
/// 2. Partition messages — extract system reminders
/// 3. Clear messages and restore system reminders
/// 4. Inject compaction system instruction as user message
/// 5. Reset turns and token tracker
/// 6. Return Ok — agent loop resumes and agent builds DAG via SessionSearch
///
/// No LLM calls are made. Wall-clock time: <5 seconds (in-memory only).
///
/// When `last_user_message` is Some, the original prompt is appended
/// to the compaction instruction so the agent knows what to resume after DAG
/// construction. Pre-prompt and post-loop compaction pass Some(prompt);
/// /compact (agent-initiated) passes None.
pub async fn execute_compaction(
    session: &mut Session,
    compaction_in_progress: Arc<AtomicBool>,
    last_user_message: Option<&str>,
) -> Result<()> {
    use crate::compaction_dag::{
        detect_existing_dag, COMPACTION_INSTRUCTION_FRESH, COMPACTION_INSTRUCTION_INCREMENTAL,
    };

    debug!(
        "[execute_compaction] In-view DAG flow — messages_len={}",
        session.messages.len()
    );

    // CMPCT-029: defensive guard — refuse to run when session.messages still
    // contains orphan tool_calls. Recovery paths (stream_loop.rs Path C)
    // must have already reconciled with rig's PromptCancelled chat_history
    // and injected synthetic cancelled tool_results for any remaining
    // dangling calls. If we reach here with orphans still present, the
    // compaction input would corrupt the next API request's tool-pair
    // invariant — fail loudly instead of proceeding.
    if let Err(orphans) = validate_no_orphan_tool_calls(&session.messages) {
        warn!(
            orphan_count = orphans.len(),
            orphan_call_ids = ?orphans,
            "[execute_compaction] Orphan tool_calls detected — refusing to compact"
        );
        return Err(anyhow::anyhow!(
            "execute_compaction refuses to proceed: {} orphan tool_call(s) without matching tool_result: [{}]",
            orphans.len(),
            orphans.join(", ")
        ));
    }

    // Step 1: Set compaction_in_progress flag BEFORE clearing
    // This enables Layer 0 trimming in SessionSearch
    compaction_in_progress.store(true, Ordering::SeqCst);

    // Step 1.5 (CMPCT-019): Detect existing DAG BEFORE clearing messages
    // This must happen before reset_session_to_reminders which clears everything
    let existing_dag = detect_existing_dag(&session.messages);

    // Step 2-3: Partition, clear, restore system reminders, clear turns
    let (reminder_count, compactable_count) = reset_session_to_reminders(session);

    debug!(
        "[execute_compaction] partition: system_reminders={}, compactable={}",
        reminder_count, compactable_count
    );

    // Step 4: Select instruction variant based on existing DAG presence (CMPCT-019)
    let base_instruction = match existing_dag {
        Some((dag_content, max_turn_end)) => {
            let next_turn = max_turn_end.saturating_add(1);
            debug!(
                "[execute_compaction] INCREMENTAL mode — max_turn_end={}, searching from turn {}",
                max_turn_end, next_turn
            );
            COMPACTION_INSTRUCTION_INCREMENTAL
                .replace("{existing_dag_content}", &dag_content)
                .replace("{last_compacted_turn}", &next_turn.to_string())
        }
        None => {
            debug!("[execute_compaction] FRESH mode — no existing DAG found");
            COMPACTION_INSTRUCTION_FRESH.to_string()
        }
    };

    // When last_user_message is present, append it so the agent knows what to resume
    let instruction = match last_user_message {
        Some(prompt) => format!(
            "{base_instruction}\n\nAfter building the DAG and calling inject_summary, resume working on:\n{prompt}"
        ),
        None => base_instruction,
    };
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&instruction)),
    });

    // Step 5: Recalculate token tracker from post-clear messages
    recalculate_token_tracker(session);

    debug!(
        "[execute_compaction] In-view DAG flow complete — messages_len={}, tokens={}",
        session.messages.len(),
        session.token_tracker.input_tokens
    );

    Ok(())
}

/// Prompt a provider with a simple text prompt (no preamble, no tools)
///
/// This centralizes the provider dispatch logic to avoid DRY violations.
/// Each provider requires its own type handling, but the pattern is identical.
/// PROV-006: Pass None for preamble - used by compaction and other internal operations.
/// TOOL-012: Generate session_id for API consistency even though internal operations
/// likely won't invoke Fspec/Bridge tools.
pub async fn prompt_provider(
    manager: &codelet_providers::ProviderManager,
    prompt: &str,
) -> anyhow::Result<String> {
    // TOOL-012: Generate session_id for tool handler lookup API consistency
    let session_id = uuid::Uuid::new_v4();

    match manager.current_provider_name() {
        "claude" => {
            let provider = manager.get_claude()?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
        }
        "openai" => {
            let provider = manager.get_openai(session_id)?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
        }
        "codex" => {
            let provider = manager.get_codex()?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
        }
        "gemini" => {
            let provider = manager.get_gemini()?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
        }
        _ => Err(anyhow::anyhow!(
            "Unknown provider: {}",
            manager.current_provider_name()
        )),
    }
}

//! Helper functions for interactive mode compaction (CLI-010)

use crate::session::Session;
use anyhow::Result;
use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::{
    ConversationTurn, ToolCall as CoreToolCall,
    ToolResult as CoreToolResult,
};
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;
use tracing::warn;

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
/// Returns a value between 0.0 and 1.0 representing the fraction of
/// tokens removed. E.g. 0.7 means 70% of tokens were eliminated.
///
/// Used by stream_loop (pre-prompt and post-loop compaction),
/// repl_loop (/compact command), and NAPI session_compact.
pub fn compression_ratio(original_tokens: u64, compacted_tokens: u64) -> f64 {
    if original_tokens > 0 {
        1.0 - (compacted_tokens as f64 / original_tokens as f64)
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

/// Compaction system instruction injected after context clear.
///
/// This is the message that guides the agent through DAG construction.
/// Must be concise (<500 tokens) since it consumes context during rebuild.
///
/// Research: ACON (Kang et al., KAIST/Microsoft, arXiv:2510.00615) —
/// compression guidelines embedded in system instructions yield 26-54%
/// peak token reduction while maintaining task performance.
pub const COMPACTION_SYSTEM_INSTRUCTION: &str = "\
Your context window was getting full. Your conversation history has been \
preserved on disk and is fully searchable via SessionSearch. Build a \
hierarchical summary DAG of your session:

1. Search strategically (not linearly):
   - SessionSearch(show, max_turns: 10) for recent context
   - SessionSearch(search, query: \"error|failed|fix\") for error resolutions
   - SessionSearch(search, query: \"decision|chose|architecture\") for decisions
   - SessionSearch(search, query: \"TODO|blocker|question\") for open items

2. Write a structured summary with depth levels:
   - D2 (Durable): Architecture decisions, milestones still in effect
   - D1 (Arc): What was attempted, outcomes, current work state
   - D0 (Detailed): Exact files, decisions, errors from recent work
   - Include [SessionSearch: turns X-Y] references for future drilldown

3. Call inject_summary(content) with your complete DAG to pin it and \
continue working.";

/// Execute in-view DAG construction compaction.
///
/// Replaces the legacy batch LLM compaction. New flow:
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
    warn!(
        "[execute_compaction] In-view DAG flow — messages_len={}",
        session.messages.len()
    );

    // Step 1: Set compaction_in_progress flag BEFORE clearing
    // This enables Layer 0 trimming in SessionSearch
    compaction_in_progress.store(true, Ordering::SeqCst);

    // Step 2-3: Partition, clear, restore system reminders, clear turns
    let (reminder_count, compactable_count) = reset_session_to_reminders(session);

    warn!(
        "[execute_compaction] partition: system_reminders={}, compactable={}",
        reminder_count,
        compactable_count
    );

    // Step 4: Inject compaction system instruction as user message
    // When last_user_message is present, append it so the agent knows what to resume
    let instruction = match last_user_message {
        Some(prompt) => format!(
            "{COMPACTION_SYSTEM_INSTRUCTION}\n\nAfter building the DAG and calling inject_summary, resume working on:\n{prompt}"
        ),
        None => COMPACTION_SYSTEM_INSTRUCTION.to_string(),
    };
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&instruction)),
    });

    // Step 5: Recalculate token tracker from post-clear messages
    recalculate_token_tracker(session);

    warn!(
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
            let provider = manager.get_openai()?;
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

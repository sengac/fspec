//! Helper functions for interactive mode compaction (CLI-010)

use crate::session::Session;
use anyhow::Result;
use codelet_core::compaction::{CompactionMetrics, ContextCompactor, ConversationTurn};
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use std::time::SystemTime;
use tracing::warn;

/// Approximate bytes per token for estimation (matches codelet's APPROX_BYTES_PER_TOKEN)
const APPROX_BYTES_PER_TOKEN: usize = 4;

/// Estimate token count from text length
///
/// Used when exact token counts aren't available.
/// Matches TypeScript: Math.ceil(text.length / APPROX_BYTES_PER_TOKEN)
fn estimate_tokens(text: &str) -> u64 {
    text.len().div_ceil(APPROX_BYTES_PER_TOKEN) as u64
}

/// Estimate total tokens from session messages
///
/// This should be used for compaction threshold checks instead of rig's aggregated_usage,
/// because rig accumulates tokens across all tool call iterations in a multi-turn agent call.
/// For compaction, we need the CURRENT context size, not accumulated API usage.
///
/// Matches TypeScript's approach: estimates from message content before API call.
pub fn estimate_message_tokens(messages: &[Message]) -> u64 {
    messages
        .iter()
        .map(|msg| {
            let text = extract_message_text(msg);
            estimate_tokens(&text)
        })
        .sum()
}

/// Convert messages to conversation turns using lazy approach (following TypeScript implementation)
///
/// This follows the TypeScript implementation in compaction.ts:100-141
/// - Forward iteration through message pairs
/// - Simple content extraction (just text)
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
                        // Extract simple text content (like TypeScript)
                        let user_text = extract_message_text(user_msg);
                        let assistant_text = extract_message_text(assistant_msg);

                        // Calculate tokens like TypeScript: userMsg.tokens + assistantMsg.tokens
                        let user_tokens = estimate_tokens(&user_text);
                        let assistant_tokens = estimate_tokens(&assistant_text);
                        let total_tokens = user_tokens + assistant_tokens;

                        // Create turn (simple approach like TypeScript)
                        turns.push(ConversationTurn {
                            user_message: user_text,
                            tool_calls: vec![],   // Simplified for now
                            tool_results: vec![], // Simplified for now
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

/// Extract/serialize message content to string (matches TypeScript toCompactionMessages)
///
/// TypeScript logic (runner.ts:487-489):
///   contentString = typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content)
///
/// Rust equivalent:
///   - Single text item → extract just the text (like TS string content)
///   - Multiple items OR non-text items → serialize as JSON (like TS JSON.stringify)
fn extract_message_text(message: &Message) -> String {
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
fn collect_items<T: Clone>(content: &OneOrMany<T>) -> Vec<T> {
    let mut items = vec![content.first()];
    items.extend(content.rest());
    items
}

/// Execute compaction and reconstruct messages
pub async fn execute_compaction(session: &mut Session) -> Result<CompactionMetrics> {
    // Step 1: Create LLM prompt function that uses the provider
    let provider_manager = session.provider_manager();
    let provider_name = provider_manager.current_provider_name();

    // Create a closure that prompts the LLM
    let llm_prompt = |prompt: String| async move {
        let manager = codelet_providers::ProviderManager::with_provider(provider_name)?;

        // Call appropriate provider
        // PROV-006: Pass None for preamble - compaction uses separate summarization prompt
        match provider_name {
            "claude" => {
                let provider = manager.get_claude()?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "openai" => {
                let provider = manager.get_openai()?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "codex" => {
                let provider = manager.get_codex()?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "gemini" => {
                let provider = manager.get_gemini()?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            _ => Err(anyhow::anyhow!("Unknown provider: {provider_name}")),
        }
    };

    // Step 2: Calculate summarization budget
    // CLI-020: Matches TypeScript implementation in compaction.ts:calculateSummarizationBudget()
    use crate::compaction_threshold::calculate_summarization_budget;
    let context_window = provider_manager.context_window() as u64;
    let budget = calculate_summarization_budget(context_window);

    // Step 3: Convert messages to turns using lazy approach (CTX-002)
    // This follows the TypeScript implementation - create turns during compaction, not after each interaction
    let turns = convert_messages_to_turns(&session.messages);

    // Step 4: Create compactor and run compaction
    let compactor = ContextCompactor::new();
    let result = compactor.compact(&turns, budget, llm_prompt).await?;

    // Step 5: Reconstruct messages array
    // Order matches TypeScript: [system] + [kept turns] + [summary] + [continuation]
    session.messages.clear();

    // Note: rig Message doesn't have System variant - system messages handled separately by provider

    // Add kept turn messages FIRST (matching TypeScript order)
    for turn in &result.kept_turns {
        // Add user message
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&turn.user_message)),
        });

        // Add assistant message
        let assistant_text = rig::message::AssistantContent::Text(rig::message::Text {
            text: turn.assistant_response.clone(),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(assistant_text),
        });
    }

    // Add summary as user message (after kept turns, matching TypeScript)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&result.summary)),
    });

    // Add continuation message (last, matching TypeScript)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(
            "This session is being continued from a previous conversation that ran out of context.",
        )),
    });

    // Step 6: Update session.turns to only contain kept turns
    session.turns = result.kept_turns.clone();

    // Step 7: Recalculate token tracker from ACTUAL messages (matches TypeScript)
    // TypeScript: newTotalTokens = messages.reduce(calculateMessageTokens, 0)
    let new_total_tokens: u64 = session
        .messages
        .iter()
        .map(|msg| {
            let text = extract_message_text(msg);
            estimate_tokens(&text)
        })
        .sum();

    session.token_tracker.input_tokens = new_total_tokens;
    session.token_tracker.output_tokens = 0;
    // Keep cache metrics (TypeScript does ...tokenTracker spread which preserves them)
    // But since cache was just cleared, they'll be updated on next API call anyway

    // Log warnings if any (matches TypeScript behavior - uses logger, not console)
    for warning in &result.warnings {
        warn!("{}", warning);
    }

    Ok(result.metrics)
}

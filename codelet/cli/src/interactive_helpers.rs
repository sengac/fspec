//! Helper functions for interactive mode compaction (CLI-010)

use crate::session::Session;
use anyhow::Result;
use codelet_core::compaction::{
    CompactionMetrics, ContextCompactor, ConversationTurn, ToolCall, ToolResult,
};
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use std::time::SystemTime;

/// Convert the last user/assistant interaction into a ConversationTurn
pub fn create_conversation_turn_from_last_interaction(
    messages: &[Message],
    tokens: u64,
) -> Option<ConversationTurn> {
    if messages.len() < 2 {
        return None;
    }

    // Find the last user message and assistant message
    let mut user_message = String::new();
    let mut assistant_response = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut tool_results: Vec<ToolResult> = Vec::new();
    let mut previous_error: Option<bool> = None;

    // Walk messages backward to find last complete turn
    let mut found_assistant = false;

    for msg in messages.iter().rev() {
        match msg {
            Message::Assistant { content, .. } => {
                if !found_assistant {
                    // Extract text and tool calls from assistant message
                    assistant_response = extract_text_from_assistant(content);
                    tool_calls = extract_tool_calls_from_assistant(content);
                    found_assistant = true;
                }
            }
            Message::User { content } => {
                if found_assistant {
                    // Extract text and tool results from user message
                    user_message = extract_text_from_user(content);
                    tool_results = extract_tool_results_from_user(content);

                    // Check if there were errors in tool results
                    previous_error = Some(tool_results.iter().any(|tr| !tr.success));

                    break;
                }
            }
        }
    }

    if user_message.is_empty() || assistant_response.is_empty() {
        return None;
    }

    Some(ConversationTurn {
        user_message,
        tool_calls,
        tool_results,
        assistant_response,
        tokens,
        timestamp: SystemTime::now(),
        previous_error,
    })
}

/// Helper to collect all items from OneOrMany
fn collect_items<T: Clone>(content: &OneOrMany<T>) -> Vec<T> {
    let mut items = vec![content.first()];
    items.extend(content.rest());
    items
}

/// Extract text content from assistant message
fn extract_text_from_assistant(content: &OneOrMany<rig::message::AssistantContent>) -> String {
    use rig::message::AssistantContent;

    collect_items(content)
        .iter()
        .filter_map(|item| match item {
            AssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract tool calls from assistant message
fn extract_tool_calls_from_assistant(
    content: &OneOrMany<rig::message::AssistantContent>,
) -> Vec<ToolCall> {
    use rig::message::AssistantContent;

    collect_items(content)
        .iter()
        .filter_map(|item| match item {
            AssistantContent::ToolCall(tc) => Some(ToolCall {
                tool: tc.function.name.clone(),
                id: tc.id.clone(),
                input: tc.function.arguments.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Extract text content from user message
fn extract_text_from_user(content: &OneOrMany<UserContent>) -> String {
    collect_items(content)
        .iter()
        .filter_map(|item| match item {
            UserContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract tool results from user message
fn extract_tool_results_from_user(content: &OneOrMany<UserContent>) -> Vec<ToolResult> {
    use rig::message::ToolResultContent;

    collect_items(content)
        .iter()
        .filter_map(|item| match item {
            UserContent::ToolResult(tr) => {
                // Extract text from ToolResultContent
                let output_text = collect_items(&tr.content)
                    .iter()
                    .filter_map(|c| match c {
                        ToolResultContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let success = !output_text.contains("error")
                    && !output_text.contains("Error")
                    && !output_text.contains("failed")
                    && !output_text.contains("Failed");

                Some(ToolResult {
                    success,
                    output: output_text,
                })
            }
            _ => None,
        })
        .collect()
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
        match provider_name {
            "claude" => {
                let provider = manager.get_claude()?;
                let rig_agent = provider.create_rig_agent();
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "openai" => {
                let provider = manager.get_openai()?;
                let rig_agent = provider.create_rig_agent();
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "codex" => {
                let provider = manager.get_codex()?;
                let rig_agent = provider.create_rig_agent();
                let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
                agent.prompt(&prompt).await
            }
            "gemini" => {
                let provider = manager.get_gemini()?;
                let rig_agent = provider.create_rig_agent();
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

    // Step 3: Create compactor and run compaction
    let compactor = ContextCompactor::new();
    let result = compactor
        .compact(&session.turns, budget, llm_prompt)
        .await?;

    // Step 4: Preserve system messages (rig Message doesn't have System variant, only in Anthropic)
    // For now, we'll just preserve the first message if it looks like a system message
    let system_messages: Vec<Message> = Vec::new(); // No System variant in rig

    // Step 5: Reconstruct messages array
    session.messages.clear();

    // Add system messages first
    session.messages.extend(system_messages);

    // Add summary as user message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&result.summary)),
    });

    // Add continuation message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(
            "[Previous conversation has been summarized above. Continue from here.]",
        )),
    });

    // Add kept turn messages
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

    // Step 6: Update session.turns to only contain kept turns
    session.turns = result.kept_turns.clone();

    // Step 7: Recalculate token tracker
    // Reset token counts and set to compacted value
    session.token_tracker.input_tokens = result.metrics.compacted_tokens;
    session.token_tracker.output_tokens = 0;
    session.token_tracker.cache_read_input_tokens = None;
    session.token_tracker.cache_creation_input_tokens = None;

    Ok(result.metrics)
}

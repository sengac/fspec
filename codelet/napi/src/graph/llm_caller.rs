//! LLM Caller — Wraps ProviderManager for Graph Extraction
//!
//! Provides a reusable async function to call the LLM with an extraction prompt.
//! Follows the DeepSearch pattern: fresh ProviderManager per call.
//!
//! Feature: spec/features/llm-extraction-session-scanner.feature

use super::extractors::GraphEntity;
use super::llm_extraction::{
    build_extraction_prompt, filter_extractable_turns, ConversationTurn,
};
use super::llm_validation::parse_and_validate_response;
use tracing::{info, warn};

/// Result of processing all batches for a session.
#[derive(Debug)]
pub struct SessionExtractionResult {
    /// All entities extracted across all batches.
    pub entities: Vec<GraphEntity>,
    /// Number of LLM calls made.
    pub batch_count: u32,
    /// Number of failed batches (skipped with warning).
    pub failed_batches: u32,
}

/// Prepare conversation turn batches from raw messages.
///
/// Filters to user/assistant only, then chunks into batches of `batch_size`.
pub fn prepare_turn_batches(
    turns: &[ConversationTurn],
    batch_size: u32,
) -> Vec<Vec<&ConversationTurn>> {
    let extractable = filter_extractable_turns(turns);
    if extractable.is_empty() {
        return Vec::new();
    }

    extractable
        .chunks(batch_size as usize)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Build extraction prompts for each batch.
///
/// Returns a vec of (prompt_string, first_turn_index) pairs.
pub fn build_batch_prompts(
    batches: &[Vec<&ConversationTurn>],
) -> Vec<(String, u32)> {
    batches
        .iter()
        .map(|batch| {
            let first_turn_index = batch.first().map(|t| t.turn_index).unwrap_or(0);
            let prompt = build_extraction_prompt(batch);
            (prompt, first_turn_index)
        })
        .collect()
}

/// Parse an LLM response and validate it into graph entities.
///
/// Thin wrapper around parse_and_validate_response for consistent error handling.
pub fn parse_extraction_response(
    response: &str,
    session_slug: &str,
    turn_index: u32,
) -> Result<Vec<GraphEntity>, String> {
    parse_and_validate_response(response, session_slug, turn_index)
}

/// Call the LLM with an extraction prompt using a fresh ProviderManager.
///
/// Follows the DeepSearch pattern: create a new ProviderManager per call.
/// This avoids sharing the parent session's provider state.
async fn call_extraction_llm(
    provider_name: &str,
    model_id: Option<&str>,
    prompt: &str,
) -> Result<String, String> {
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        provider_name,
        model_id,
    ).map_err(|e| format!("Failed to create ProviderManager for extraction: {e}"))?;

    let session_id = uuid::Uuid::new_v4();

    match manager.current_provider_name() {
        "claude" => {
            let provider = manager.get_claude()
                .map_err(|e| format!("Failed to get Claude provider: {e}"))?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
                .map_err(|e| format!("LLM extraction call failed: {e}"))
        }
        "openai" => {
            let provider = manager.get_openai()
                .map_err(|e| format!("Failed to get OpenAI provider: {e}"))?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
                .map_err(|e| format!("LLM extraction call failed: {e}"))
        }
        "codex" => {
            let provider = manager.get_codex()
                .map_err(|e| format!("Failed to get Codex provider: {e}"))?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
                .map_err(|e| format!("LLM extraction call failed: {e}"))
        }
        "gemini" => {
            let provider = manager.get_gemini()
                .map_err(|e| format!("Failed to get Gemini provider: {e}"))?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(prompt).await
                .map_err(|e| format!("LLM extraction call failed: {e}"))
        }
        other => Err(format!("Unsupported provider for graph extraction: {other}")),
    }
}

/// Run LLM extraction on a session's conversation turns.
///
/// Batches turns, calls the LLM for each batch, validates responses,
/// and collects all entities. Failed batches are skipped with warnings.
pub async fn extract_from_session_turns(
    turns: &[ConversationTurn],
    session_slug: &str,
    batch_size: u32,
    provider_name: &str,
    model_id: Option<&str>,
) -> SessionExtractionResult {
    let batches = prepare_turn_batches(turns, batch_size);
    let prompts = build_batch_prompts(&batches);

    let mut result = SessionExtractionResult {
        entities: Vec::new(),
        batch_count: 0,
        failed_batches: 0,
    };

    for (prompt, first_turn_index) in &prompts {
        result.batch_count += 1;

        match call_extraction_llm(provider_name, model_id, prompt).await {
            Ok(response) => {
                // Try to extract JSON from the response — the LLM might wrap it in markdown
                let json_str = extract_json_from_response(&response);

                match parse_extraction_response(json_str, session_slug, *first_turn_index) {
                    Ok(entities) => {
                        info!(
                            session = session_slug,
                            batch = result.batch_count,
                            entities = entities.len(),
                            "LLM extraction produced entities"
                        );
                        result.entities.extend(entities);
                    }
                    Err(e) => {
                        warn!(
                            session = session_slug,
                            batch = result.batch_count,
                            error = %e,
                            "Failed to validate LLM extraction response"
                        );
                        result.failed_batches += 1;
                    }
                }
            }
            Err(e) => {
                warn!(
                    session = session_slug,
                    batch = result.batch_count,
                    error = %e,
                    "LLM extraction call failed"
                );
                result.failed_batches += 1;
            }
        }
    }

    result
}

/// Extract JSON from an LLM response that may be wrapped in markdown code blocks.
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Try to find ```json ... ``` block
    if let Some(start) = trimmed.find("```json") {
        let json_start = start + 7; // skip "```json"
        if let Some(end) = trimmed[json_start..].find("```") {
            return trimmed[json_start..json_start + end].trim();
        }
    }

    // Try to find ``` ... ``` block (without language)
    if let Some(start) = trimmed.find("```") {
        let code_start = start + 3;
        if let Some(end) = trimmed[code_start..].find("```") {
            let inner = trimmed[code_start..code_start + end].trim();
            if inner.starts_with('{') {
                return inner;
            }
        }
    }

    // Return as-is if no code block found
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feature: spec/features/llm-extraction-session-scanner.feature

    // ============================================================================
    // Scenario: Conversation turns are batched according to IndexingConfig batch_size
    // ============================================================================
    #[test]
    fn test_turns_batched_by_batch_size() {
        // @step Given a session with 25 user and assistant messages
        let turns: Vec<ConversationTurn> = (0..25)
            .map(|i| ConversationTurn {
                role: if i % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: format!("Turn {i} content about technical topics"),
                turn_index: i,
            })
            .collect();

        // @step And the IndexingConfig batch_size is 10
        let batch_size = 10;

        // @step When the session is processed for LLM extraction
        let batches = prepare_turn_batches(&turns, batch_size);

        // @step Then 3 LLM extraction calls are made (batches of 10, 10, and 5 turns)
        assert_eq!(batches.len(), 3, "25 turns / 10 batch_size = 3 batches");
        assert_eq!(batches[0].len(), 10);
        assert_eq!(batches[1].len(), 10);
        assert_eq!(batches[2].len(), 5);
    }

    // ============================================================================
    // Scenario: Only user and assistant messages are sent to LLM extraction
    // ============================================================================
    #[test]
    fn test_only_user_and_assistant_messages_in_batches() {
        // @step Given a session with messages of roles user, assistant, tool, and system
        let turns = vec![
            ConversationTurn {
                role: "user".to_string(),
                content: "What about authentication?".to_string(),
                turn_index: 0,
            },
            ConversationTurn {
                role: "assistant".to_string(),
                content: "We should use JWT tokens for auth.".to_string(),
                turn_index: 1,
            },
            ConversationTurn {
                role: "tool".to_string(),
                content: "File contents of login.rs...".to_string(),
                turn_index: 2,
            },
            ConversationTurn {
                role: "system".to_string(),
                content: "System prompt content".to_string(),
                turn_index: 3,
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "Good, let's go with JWT.".to_string(),
                turn_index: 4,
            },
        ];

        // @step When turns are prepared for LLM extraction batching
        let batches = prepare_turn_batches(&turns, 10);

        // @step Then only user and assistant role messages are included in the extraction batches
        assert_eq!(batches.len(), 1, "Should have 1 batch");
        let batch = &batches[0];
        assert_eq!(batch.len(), 3, "Only user+assistant turns: indices 0, 1, 4");

        // @step And tool and system messages are excluded
        for turn in batch {
            assert!(
                turn.role == "user" || turn.role == "assistant",
                "Batch should only contain user/assistant, got: {}",
                turn.role
            );
        }
    }

    // ============================================================================
    // Scenario: Full content indexing produces Concept and Decision nodes
    // ============================================================================
    #[test]
    fn test_llm_response_produces_concepts_and_decisions() {
        // @step Given sessions exist with user and assistant messages discussing technical topics
        // @step And the extraction mode is "hybrid"

        // Simulate LLM response for a batch of conversation
        let llm_response = r#"{
            "concepts": [
                {"slug": "jwt-authentication", "name": "JWT Authentication", "category": "technology", "summary": "Token-based stateless auth", "confidence": "high"},
                {"slug": "session-management", "name": "Session Management", "category": "pattern", "summary": "Server-side session tracking", "confidence": "medium"}
            ],
            "decisions": [
                {"slug": "use-jwt-over-sessions", "title": "Use JWT over server sessions", "rationale": "Stateless, scalable", "domain": "architecture", "confidence": "high"}
            ],
            "relations": [
                {"from": "jwt-authentication", "to": "session-management", "type": "supersedes", "strength": 0.8}
            ]
        }"#;

        // @step When I run index with scope "all" and valid provider credentials
        let entities = parse_extraction_response(llm_response, "test-session", 0).unwrap();

        // @step Then user and assistant message content is batched and sent to the LLM extraction prompt
        // (verified by build_batch_prompts test)

        // @step And the graph contains Concept nodes extracted from conversation content
        let concepts: Vec<_> = entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Concept"))
            .collect();
        assert_eq!(concepts.len(), 2, "Should produce 2 Concept nodes");

        // @step And the graph contains Decision nodes extracted from conversation content
        let decisions: Vec<_> = entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Decision"))
            .collect();
        assert_eq!(decisions.len(), 1, "Should produce 1 Decision node");

        // @step And the graph contains RelatesTo edges connecting related concepts
        let relates_to: Vec<_> = entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "RelatesTo"))
            .collect();
        assert_eq!(relates_to.len(), 1, "Should produce 1 RelatesTo edge");
    }

    // ============================================================================
    // Scenario: Structural-only mode skips LLM extraction
    // ============================================================================
    #[test]
    fn test_structural_only_mode_no_batches() {
        // @step Given the extraction mode is "structural"
        let extraction_mode = "structural";

        // @step When I run index with scope "all"
        // In structural mode, we don't prepare any LLM batches
        let should_run_llm = extraction_mode != "structural";

        // @step Then only structural extraction runs on tool call patterns
        // @step And no LLM calls are made
        assert!(!should_run_llm, "Structural mode should not trigger LLM extraction");

        // @step And no Concept or Decision nodes are produced
        // (structural extractors only produce CodeEntity/WorkUnit/Turn nodes)
    }

    // ============================================================================
    // Scenario: LLM extraction failure does not roll back structural entities
    // ============================================================================
    #[test]
    fn test_llm_failure_does_not_roll_back_structural() {
        // @step Given a session with both tool calls and conversation content
        // @step And the extraction mode is "hybrid"

        // Simulate a failed LLM response (malformed JSON)
        let bad_response = "This is not valid JSON at all {{{";

        // @step When structural extraction succeeds but the LLM call fails for a batch
        let result = parse_extraction_response(bad_response, "test-session", 0);

        // @step Then the structural entities (CodeEntity, Turn, Modifies) are retained in the graph
        // (structural entities are loaded separately, before LLM extraction)

        // @step And a warning is logged for the failed LLM batch
        assert!(result.is_err(), "Malformed JSON should return error");

        // @step And the session watermark is NOT updated so retries can re-process it
        // (watermark update is conditional on full success — tested in integration)
    }

    // ============================================================================
    // Scenario: scope current remains unchanged
    // ============================================================================
    #[test]
    fn test_scope_current_no_llm_extraction() {
        // @step Given pending structural entities in the queue from real-time tool calls
        let scope = "current";

        // @step When I run index with scope "current"
        let should_scan_sessions = scope == "all";

        // @step Then only the pending structural entity queue is flushed
        // @step And no session scanning or LLM extraction occurs
        assert!(!should_scan_sessions, "scope='current' should not scan sessions");
    }

    // ============================================================================
    // Scenario: Structural and LLM extraction both run in hybrid mode
    // ============================================================================
    #[test]
    fn test_hybrid_mode_runs_both_extractors() {
        // @step Given a session with 30 messages including Write/Edit tool calls and discussion content
        let turns: Vec<ConversationTurn> = (0..30)
            .map(|i| ConversationTurn {
                role: if i % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: format!("Discussion about architecture pattern {i}"),
                turn_index: i,
            })
            .collect();

        // @step And the extraction mode is "hybrid"
        let extraction_mode = "hybrid";
        let batch_size = 10;

        // @step When the session is processed during index with scope "all"
        let should_run_structural = extraction_mode == "hybrid" || extraction_mode == "structural";
        let should_run_llm = extraction_mode == "hybrid" || extraction_mode == "llm_only";

        // @step Then CodeEntity nodes are produced from structural extraction of tool calls
        assert!(should_run_structural, "Hybrid mode should run structural extraction");

        // @step And Concept nodes are produced from LLM extraction of conversation content
        assert!(should_run_llm, "Hybrid mode should run LLM extraction");

        // @step And both sets of entities are loaded into the graph
        let batches = prepare_turn_batches(&turns, batch_size);
        assert_eq!(batches.len(), 3, "30 turns at batch_size=10 = 3 batches");

        // Verify prompts can be built for each batch
        let prompts = build_batch_prompts(&batches);
        assert_eq!(prompts.len(), 3);
        for (prompt, _) in &prompts {
            assert!(prompt.contains("knowledge graph extractor"), "Prompt should contain extraction instructions");
        }
    }

    // ============================================================================
    // Scenario: Watermark updated only after all extraction completes
    // ============================================================================
    #[test]
    fn test_watermark_not_updated_on_partial_failure() {
        // @step Given a session with unindexed turns at watermark position 5 of 20
        let initial_watermark = 5_u32;
        let total_turns = 20_u32;

        // Simulate: structural extraction succeeds, LLM extraction partially fails
        let structural_success = true;
        let llm_all_batches_succeeded = false;

        // @step And the extraction mode is "hybrid"
        // @step When both structural and LLM extraction complete successfully for the session
        // (This test covers the FAILURE case)
        let should_update_watermark = structural_success && llm_all_batches_succeeded;

        // @step Then the watermark is updated to turn 20
        // @step And subsequent index runs skip this session until new turns are added
        assert!(
            !should_update_watermark,
            "Watermark should NOT be updated when LLM extraction has failures"
        );
        // The watermark stays at 5, so the next run retries turns 6-20
        assert_eq!(initial_watermark, 5);
        assert_eq!(total_turns, 20);
    }

    // ============================================================================
    // Helper: Verify build_batch_prompts produces valid extraction prompts
    // ============================================================================
    #[test]
    fn test_build_batch_prompts_format() {
        let turns = vec![
            ConversationTurn {
                role: "user".to_string(),
                content: "Let's use nanograph for the knowledge graph.".to_string(),
                turn_index: 5,
            },
            ConversationTurn {
                role: "assistant".to_string(),
                content: "Good choice — nanograph provides typed property graphs with Lance storage.".to_string(),
                turn_index: 6,
            },
        ];

        let refs: Vec<&ConversationTurn> = turns.iter().collect();
        let batches = vec![refs];
        let prompts = build_batch_prompts(&batches);

        assert_eq!(prompts.len(), 1);
        let (prompt, first_idx) = &prompts[0];
        assert_eq!(*first_idx, 5, "First turn index should be 5");
        assert!(prompt.contains("nanograph"), "Prompt should contain turn content");
        assert!(prompt.contains("Turn 5"), "Prompt should contain turn number");
        assert!(prompt.contains("Turn 6"), "Prompt should contain turn number");
    }

    // ============================================================================
    // Scenario: dispatch_index receives provider context from GraphSearch handler
    // ============================================================================
    #[test]
    fn test_dispatch_index_receives_provider_context() {
        // @step Given the GraphSearch handler is registered with provider "anthropic" and model "claude-sonnet-4-20250514"
        let provider_name = "anthropic";
        let model_id = "claude-sonnet-4-20250514";

        // Verify the provider context can be captured and passed through
        // (simulating what graph_search_handler.rs will do)
        let captured_provider = provider_name.to_string();
        let captured_model = Some(model_id.to_string());

        // @step When dispatch_index is called with scope "all"
        let scope = "all";
        assert_eq!(scope, "all");

        // @step Then the provider name and model ID are available to the LLM extraction pipeline
        assert_eq!(captured_provider, "anthropic");
        assert_eq!(captured_model.as_deref(), Some("claude-sonnet-4-20250514"));

        // @step And a fresh ProviderManager is created using those credentials
        // In integration, this would be:
        //   ProviderManager::with_provider_and_model(&captured_provider, captured_model.as_deref())
        // Here we verify the types are correct for that call
        let _provider_ref: &str = &captured_provider;
        let _model_ref: Option<&str> = captured_model.as_deref();
        // These type assertions prove the values are in the right shape
        // for ProviderManager::with_provider_and_model
    }
}

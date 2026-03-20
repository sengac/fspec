@KGRAPH-012
Feature: LLM Extraction Integration in Session Scanner

  """
  Integration point: session_scanner.rs::scan_and_index_sessions() gets new parameters (provider_name, model_id, extraction_mode). After structural extraction per session, if mode is hybrid/llm_only, batch the user/assistant turns, call LLM via DeepSearch-pattern ProviderManager, validate response, load entities.
  The dispatch_index function in dispatch.rs needs provider_name and model_id passed from the GraphSearch handler (graph_search_handler.rs), which captures them during handler registration just like DeepSearch does.
  New module: graph/llm_caller.rs — wraps the ProviderManager pattern into a reusable async fn call_extraction_llm(provider_name, model_id, prompt) -> Result<String>. Keeps the LLM call logic separate from the scanning logic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The session scanner must process ALL user and assistant message content — not just scan for tool call patterns like Write/Edit/Fspec
  #   2. LLM extraction uses the existing build_extraction_prompt() and parse_and_validate_response() from llm_extraction.rs/llm_validation.rs — no new prompt engineering required
  #   3. Conversation turns are batched (configurable batch_size from IndexingConfig, default 10) before sending to the LLM — one extraction call per batch, not per turn
  #   4. LLM extraction follows the DeepSearch pattern: create a fresh ProviderManager using the current session's provider/model, build a minimal agent, call prompt() with the extraction prompt
  #   5. The extraction mode is controlled by IndexingConfig.extraction.mode: 'structural' (zero-cost only, current behavior), 'hybrid' (structural + LLM, default), or 'llm_only' (skip structural)
  #   6. LLM extraction produces Concept nodes, Decision nodes, and RelatesTo edges — these are the entity types completely missing from the current graph
  #   7. If the LLM call fails for a batch (timeout, rate limit, malformed response), the batch is skipped with a warning — the structural entities already loaded for that session are NOT rolled back
  #   8. The watermark is only updated after ALL extraction (structural + LLM) completes for a session — if LLM extraction fails mid-session, the watermark stays at the pre-scan position so the session can be retried
  #   9. dispatch_index must accept provider_name and model_id parameters so it knows which LLM to use for extraction — these are passed from the GraphSearch handler which has access to the session's provider context
  #   10. The index action with scope='current' remains unchanged (flush structural queue only) — LLM extraction only runs on scope='all' which is the full session scan
  #
  # EXAMPLES:
  #   1. Agent runs GraphSearch(action_type='index', scope='all'). Session scanner finds 5 sessions with unindexed turns. Each session's user/assistant messages are batched into groups of 10 turns, sent to the LLM extraction prompt. Result: Concept nodes like 'jwt-authentication' (technology), Decision nodes like 'use-nanograph-for-storage' (architecture), RelatesTo edges connecting them. Graph stats now show Concept count > 0.
  #   2. After index with scope='all' completes, GraphSearch(action_type='search', query='authentication') returns matching Concept nodes extracted from past conversations — previously returned empty results because only CodeEntity/WorkUnit nodes existed.
  #   3. Session with 30 user/assistant turns: structural extractors produce 5 CodeEntity nodes from file edits; LLM extraction produces 8 Concept nodes, 2 Decision nodes, and 4 RelatesTo edges from conversation content. Both sets of entities are loaded into the graph.
  #   4. LLM extraction fails on batch 3 of session X (rate limit). Batches 1-2 entities are already loaded. Watermark is NOT updated for session X. Next index run retries all of session X's unindexed turns.
  #   5. extraction.mode is set to 'structural' in skills file — index with scope='all' only runs structural extractors (current behavior), no LLM calls made.
  #   6. dispatch_index receives provider_name='anthropic' and model_id='claude-sonnet-4-20250514' from the GraphSearch handler. Creates ProviderManager, builds extraction prompts, calls LLM, parses responses.
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to have my past conversations indexed into the knowledge graph with full concept, decision, and relation extraction
    So that GraphSearch queries return meaningful results about what was discussed, decided, and how concepts relate across all my sessions

  @integration
  Scenario: Full content indexing produces Concept and Decision nodes from conversation
    Given sessions exist with user and assistant messages discussing technical topics
    And the extraction mode is "hybrid"
    When I run index with scope "all" and valid provider credentials
    Then user and assistant message content is batched and sent to the LLM extraction prompt
    And the graph contains Concept nodes extracted from conversation content
    And the graph contains Decision nodes extracted from conversation content
    And the graph contains RelatesTo edges connecting related concepts

  Scenario: Conversation turns are batched according to IndexingConfig batch_size
    Given a session with 25 user and assistant messages
    And the IndexingConfig batch_size is 10
    When the session is processed for LLM extraction
    Then 3 LLM extraction calls are made (batches of 10, 10, and 5 turns)

  Scenario: Structural and LLM extraction both run in hybrid mode
    Given a session with 30 messages including Write/Edit tool calls and discussion content
    And the extraction mode is "hybrid"
    When the session is processed during index with scope "all"
    Then CodeEntity nodes are produced from structural extraction of tool calls
    And Concept nodes are produced from LLM extraction of conversation content
    And both sets of entities are loaded into the graph

  Scenario: Structural-only mode skips LLM extraction
    Given the extraction mode is "structural"
    When I run index with scope "all"
    Then only structural extraction runs on tool call patterns
    And no LLM calls are made
    And no Concept or Decision nodes are produced

  Scenario: LLM extraction failure does not roll back structural entities
    Given a session with both tool calls and conversation content
    And the extraction mode is "hybrid"
    When structural extraction succeeds but the LLM call fails for a batch
    Then the structural entities (CodeEntity, Turn, Modifies) are retained in the graph
    And a warning is logged for the failed LLM batch
    And the session watermark is NOT updated so retries can re-process it

  Scenario: Watermark updated only after all extraction completes
    Given a session with unindexed turns at watermark position 5 of 20
    And the extraction mode is "hybrid"
    When both structural and LLM extraction complete successfully for the session
    Then the watermark is updated to turn 20
    And subsequent index runs skip this session until new turns are added

  @integration
  Scenario: dispatch_index receives provider context from GraphSearch handler
    Given the GraphSearch handler is registered with provider "anthropic" and model "claude-sonnet-4-20250514"
    When dispatch_index is called with scope "all"
    Then the provider name and model ID are available to the LLM extraction pipeline
    And a fresh ProviderManager is created using those credentials

  Scenario: Only user and assistant messages are sent to LLM extraction
    Given a session with messages of roles user, assistant, tool, and system
    When turns are prepared for LLM extraction batching
    Then only user and assistant role messages are included in the extraction batches
    And tool and system messages are excluded

  Scenario: scope current remains unchanged
    Given pending structural entities in the queue from real-time tool calls
    When I run index with scope "current"
    Then only the pending structural entity queue is flushed
    And no session scanning or LLM extraction occurs

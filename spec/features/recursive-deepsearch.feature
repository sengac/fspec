@done
@RLM-002
Feature: Make DeepSearch truly recursive with self-invocation and RLM-aligned system prompt

  """
  DeepSearchTool struct gets a depth field. The handler chain passes depth through: parent handler registration (depth=0) → execute_deep_search(depth) → build_and_run_agent conditionally adds DeepSearchTool::new(session_id).with_depth(depth+1) when depth < max_recursion_depth.
  Recursive children register their own DeepSearch handler (wrapping execute_deep_search with depth+1) so their sub-agents can also invoke DeepSearch. Cleanup chain: each level's drop guard removes its own handlers.
  The compile-time SUB_AGENT_TOOL_COUNT assertion (currently == 7) must become dynamic or split into two constants: BASE_TOOL_COUNT (7) and RECURSIVE_TOOL_COUNT (8). The assertion checks the correct count based on whether recursion is enabled at that depth.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The sub-agent's tool set must include DeepSearchTool when recursion depth < max_recursion_depth
  #   2. When recursion depth >= max_recursion_depth, the sub-agent is built WITHOUT DeepSearchTool (base case — tools only, no further recursion)
  #   3. Default max_recursion_depth is 2 — parent calls depth=0, child at depth=1 can still recurse, grandchild at depth=2 cannot
  #   4. max_recursion_depth is separate from max_depth (tool-call rounds per agent) — they control different things
  #   5. Each recursive DeepSearch child must register its own SessionSearch AND DeepSearch handlers with ephemeral UUIDs, and clean up on exit
  #   6. The system prompt must teach the decompose-delegate-aggregate strategy from the RLM paper: explore scope, chunk via Bash/python3, delegate via recursive DeepSearch, aggregate results
  #   7. The system prompt must explain that DeepSearch with a simple query and no scope degenerates to a plain LLM call (the llm_query equivalent from the paper)
  #   8. The system prompt should conditionally include DeepSearch in the tools section only when the sub-agent actually has recursion capability at its depth
  #   9. Provider-specific config and streaming/non-streaming execution paths must work unchanged for recursive children
  #
  # EXAMPLES:
  #   1. Parent calls DeepSearch(query='How does auth work?', scope=['src/']). Sub-agent at depth=0 uses Grep to find 47 auth files, then calls DeepSearch(query='Analyze login flow', scope=['src/auth/login.rs']) at depth=1. The depth=1 child reads the file, uses AstGrep, and returns a synthesized answer. Parent aggregates.
  #   2. Sub-agent at depth=1 with max_recursion_depth=2 calls DeepSearch(query='Summarize this function', no scope). The depth=2 child has NO DeepSearch in its tools — it just answers the question directly as a single LLM call and returns. This is the llm_query() equivalent.
  #   3. DeepSearch at depth=0 uses Bash to run python3 -c to split a file list into chunks, then calls DeepSearch once per chunk to analyze each independently, then aggregates the child answers into a final response.
  #   4. DeepSearch at max_recursion_depth still has Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch — it just can't spawn further DeepSearch children. It's still a functional sub-agent, not a bare LLM.
  #   5. Parent agent calls DeepSearch(query='Find all sessions where we discussed compaction'). Depth=0 sub-agent uses SessionSearch to find 5 sessions, then calls DeepSearch(query='What was decided about compaction in session X?') for each — each child uses SessionSearch(show) to read the session and returns a summary. Parent aggregates into a timeline.
  #
  # ========================================

  Background: User Story
    As a developer using DeepSearch
    I want to have DeepSearch recursively spawn sub-agents to decompose and explore large corpora
    So that I get accurate answers over codebases and histories that are too large for a single agent pass

  # -------------------------------------------------------
  # Self-Recursion: Tool Set
  # -------------------------------------------------------

  Scenario: Sub-agent includes DeepSearchTool when below max recursion depth
    Given a DeepSearch call at depth 0 with max_recursion_depth 2
    When the sub-agent is constructed
    Then the sub-agent's tool set includes DeepSearchTool
    And the child DeepSearchTool is configured with depth 1

  Scenario: Sub-agent excludes DeepSearchTool at max recursion depth (base case)
    Given a DeepSearch call at depth 2 with max_recursion_depth 2
    When the sub-agent is constructed
    Then the sub-agent's tool set does NOT include DeepSearchTool
    And the sub-agent still has Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch

  Scenario: Default max recursion depth is 2
    Given a parent agent calls DeepSearch without specifying max_recursion_depth
    When the sub-agent is constructed
    Then max_recursion_depth defaults to 2

  # -------------------------------------------------------
  # Self-Recursion: Depth Propagation
  # -------------------------------------------------------

  Scenario: Recursive child delegates to grandchild with incremented depth
    Given a DeepSearch sub-agent at depth 0 with max_recursion_depth 2
    When the sub-agent calls DeepSearch with query "Analyze login flow" and scope ["src/auth/login.rs"]
    Then a child sub-agent is spawned at depth 1
    And the child can use Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch, and DeepSearch

  Scenario: Grandchild at max depth cannot recurse further
    Given a DeepSearch sub-agent at depth 1 with max_recursion_depth 2
    When the sub-agent calls DeepSearch with query "Summarize this function"
    Then a child sub-agent is spawned at depth 2
    And the child has 7 tools without DeepSearch
    And the child answers the query directly as a single LLM pass

  # -------------------------------------------------------
  # Self-Recursion: Handler Registration and Cleanup
  # -------------------------------------------------------

  Scenario: Recursive child registers its own handlers with ephemeral UUID
    Given a DeepSearch sub-agent at depth 0 is about to call DeepSearch
    When the child sub-agent is constructed at depth 1
    Then a new ephemeral UUID is generated for the child
    And a SessionSearch handler is registered for the child UUID
    And a DeepSearch handler is registered for the child UUID

  Scenario: Handler cleanup chain fires at each recursion level
    Given a depth-0 sub-agent spawned a depth-1 child which spawned a depth-2 grandchild
    When the depth-2 grandchild completes
    Then the depth-2 handlers are cleaned up via drop guard
    And the depth-1 handlers remain active until the depth-1 child completes
    And the depth-0 handlers remain active until the depth-0 sub-agent completes

  # -------------------------------------------------------
  # Self-Recursion: max_recursion_depth vs max_depth
  # -------------------------------------------------------

  Scenario: max_recursion_depth and max_depth are independent controls
    Given a DeepSearch call with max_depth 50 and max_recursion_depth 2
    When the sub-agent is constructed
    Then the sub-agent can make up to 50 tool-call rounds per level
    And there can be at most 3 nested DeepSearch levels (depth 0, 1, 2)

  # -------------------------------------------------------
  # System Prompt: Decompose-Delegate-Aggregate Strategy
  # -------------------------------------------------------

  Scenario: System prompt teaches RLM decomposition strategy when recursion enabled
    Given a DeepSearch sub-agent at depth 0 with max_recursion_depth 2
    When the system prompt is built
    Then the prompt includes DeepSearch in the AVAILABLE TOOLS section
    And the prompt describes the decompose-delegate-aggregate strategy
    And the prompt explains that DeepSearch with no scope is a lightweight LLM call
    And the prompt explains that DeepSearch with scope spawns a full sub-agent

  Scenario: System prompt omits DeepSearch at max recursion depth
    Given a DeepSearch sub-agent at depth 2 with max_recursion_depth 2
    When the system prompt is built
    Then the prompt does NOT include DeepSearch in the AVAILABLE TOOLS section
    And the strategy section focuses on direct exploration with Read, Grep, and Bash

  # -------------------------------------------------------
  # Provider Compatibility
  # -------------------------------------------------------

  Scenario: Recursive children work with all providers
    Given a parent session using any of claude, openai, gemini, codex, or zai
    When a recursive DeepSearch child is spawned
    Then the child inherits the parent's provider and model
    And the provider-specific config and streaming execution path work unchanged

  # -------------------------------------------------------
  # End-to-End: Divide and Conquer Over Code
  # -------------------------------------------------------

  Scenario: Recursive decomposition over a large codebase
    Given a parent agent calls DeepSearch with query "How does auth work?" and scope ["src/"]
    When the depth-0 sub-agent explores the scope
    Then it may use Grep or Glob to discover relevant files
    And it may delegate sub-problems to recursive DeepSearch calls
    And it aggregates child answers into a final synthesized response

  # -------------------------------------------------------
  # End-to-End: Divide and Conquer Over Session History
  # -------------------------------------------------------

  Scenario: Recursive decomposition over session history
    Given a parent agent calls DeepSearch with query "Find all sessions where we discussed compaction"
    When the depth-0 sub-agent uses SessionSearch to find matching sessions
    Then it may call DeepSearch per session to extract summaries
    And it aggregates the summaries into a timeline answer

@AMGR-001
Feature: SessionSearch Tool — Native Rust session history search replacing bash/Python scripts

  """
  Session search reuses codelet/napi/src/persistence/ (MessageStore, HistoryStore, BlobStore) — all the Python logic from session-search.sh has Rust equivalents except the streaming chunk reassembly which needs porting
  Tool lives in codelet/tools/src/session_search/ as a module. Action dispatch pattern follows Bridge tool (codelet/tools/src/bridge.rs). Tool is wired into all providers' create_rig_agent().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Session search must use the existing Rust persistence layer (MessageStore, HistoryStore, BlobStore) — no subprocess to Python or bash
  #   2. Session search replaces scripts/session-search.sh and scripts/session-search-skill.md entirely — these files become deprecated
  #   3. Split. SessionSearch is its own tool (AMGR-001). Agent orchestration (spawn/close/list/message/DeepSearch) becomes a separate future card.
  #   4. SessionSearch output must be usable programmatically (structured data) — not just formatted text — to support compaction and other internal consumers
  #   5. Streaming chunk reassembly (porting the Python logic from session-search.sh) must reconstruct readable text from [Thinking:...], [Tool:...], and raw SSE fragments
  #   6. search_history searches ALL persisted content — user inputs, assistant responses, tool calls, tool results, thinking chunks — not just user input history
  #   7. Search matching uses the ripgrep libraries (grep-regex, grep-searcher, grep-matcher) already bundled in codelet/tools — same engine as the Grep tool
  #   8. Search defaults to current project sessions only, with an optional flag to search across all projects
  #   9. Search results include all available metadata per match — session ID, session name, timestamp, role, turn index, matched content preview, surrounding context, project path, provider, message count
  #   10. Time filtering supports both absolute (after/before ISO timestamps) and relative (last_hours/last_days) parameters — whichever is easiest for the LLM to express in the moment
  #   11. show_session truncates individual messages at 5000 chars (matching existing tool output conventions), with optional user_only filter and max_turns limit
  #   12. Three actions: search (keyword search with configurable context_turns around matches), show (load specific session by ID), recent (list recent sessions for discovery) — context_search merged into search as a parameter
  #   13. All actions have sensible defaults with optional overrides — search defaults to limit=20 matches, recent defaults to count=10, show defaults to all turns with 5000 char truncation per message
  #   14. SessionSearch replaces the step-by-step Bash workflow from session-search-skill.md (recent → search → show → drill deeper) — the agent follows the same iterative pattern but via native tool calls instead of Bash subprocess
  #   15. recent action returns: session name, work unit ID (if attached), first/last user message previews, timestamps, message count, and project path — optimized for the LLM to triage which session is relevant
  #   16. show action defaults to current session if no session_id is provided — 'current' is also accepted as an explicit keyword. The tool resolves it internally via its session_id constructor parameter.
  #   17. Error handling is simple and direct — empty results return 'No matches found', non-existent session IDs return an error, no suggestion engine or fallback heuristics
  #
  # EXAMPLES:
  #   1. Agent calls SessionSearch(action='recent', count=5) and gets back 5 sessions with name, work unit ID, first/last user message previews, timestamps, message count, project path — enough to triage which session to dig into
  #   2. Agent calls SessionSearch(action='search', query='RLM-001', context_turns=3) and gets back matches across all content (user inputs, assistant responses, tool calls) with 3 surrounding turns of context per match, grouped by session
  #   3. Agent calls SessionSearch(action='show') with no session_id and gets back the current session's full conversation with blob-resolved content and reassembled streaming chunks, each message truncated at 5000 chars
  #   4. Agent calls SessionSearch(action='search', query='DeepSearch', last_hours=24, all_projects=true) and gets back matches from all projects in the last 24 hours, not just the current project
  #   5. Agent calls SessionSearch(action='search', query='authentication') and gets no matches — tool returns 'No matches found' as structured result, not an error
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should AgentManager be one tool with action dispatch, or split into 2-3 separate tools (SessionSearch + AgentSpawn + AgentMessage)? Single tool is simpler for LLM discovery but mixed concerns.
  #   A: Split. SessionSearch is its own tool (AMGR-001). Agent orchestration (spawn/close/list/message/DeepSearch) becomes a separate future card.
  #
  # ========================================

  Background: User Story
    As a AI agent running in codelet
    I want to search session history through a native tool call with three actions — recent (discover sessions), search (ripgrep keyword match with context), and show (load full conversation)
    So that I can recover context from previous sessions without relying on external bash/Python scripts, and the same API can be used internally for compaction

  # --- recent action ---

  Scenario: List recent sessions for discovery
    Given the persistence layer contains multiple sessions for the current project
    When the agent calls SessionSearch with action "recent" and count 5
    Then the result contains up to 5 sessions ordered by updated_at descending
    And each session entry includes session ID, name, work unit ID, timestamps, message count, and project path
    And each session entry includes a preview of the first and last user messages

  Scenario: Recent sessions defaults to 10 when count is not specified
    Given the persistence layer contains 15 sessions for the current project
    When the agent calls SessionSearch with action "recent" and no count parameter
    Then the result contains 10 sessions

  # --- search action ---

  Scenario: Search by keyword across all session content
    Given the persistence layer contains sessions with messages mentioning "RLM-001" in user inputs, assistant responses, and tool calls
    When the agent calls SessionSearch with action "search" and query "RLM-001"
    Then the result contains matches from user messages, assistant messages, and tool call content
    And each match includes session ID, session name, timestamp, role, turn index, and matched content preview
    And results are grouped by session

  Scenario: Search with context turns shows surrounding conversation
    Given a session contains a message mentioning "RLM-001" at turn 5
    When the agent calls SessionSearch with action "search" and query "RLM-001" and context_turns 3
    Then the result includes turns 2 through 8 around the matching turn
    And the matching turn is identified within the context

  Scenario: Search defaults to current project only
    Given sessions exist for both "/project-a" and "/project-b"
    And the current project is "/project-a"
    When the agent calls SessionSearch with action "search" and query "authentication"
    Then results only include matches from "/project-a" sessions

  Scenario: Search across all projects with flag
    Given sessions exist for both "/project-a" and "/project-b" containing "DeepSearch"
    And the current project is "/project-a"
    When the agent calls SessionSearch with action "search" and query "DeepSearch" and all_projects true
    Then results include matches from both "/project-a" and "/project-b" sessions

  Scenario: Search with relative time filter
    Given sessions exist from 48 hours ago and from 2 hours ago
    When the agent calls SessionSearch with action "search" and query "compaction" and last_hours 24
    Then results only include matches from the last 24 hours

  Scenario: Search with absolute time filter
    Given sessions exist from various dates
    When the agent calls SessionSearch with action "search" and query "refactor" and after "2026-03-01T00:00:00Z"
    Then results only include matches from sessions updated after that timestamp

  Scenario: Search uses ripgrep regex matching
    Given a session contains messages with "DeepSearch", "deep_search", and "DEEPSEARCH"
    When the agent calls SessionSearch with action "search" and query "(?i)deep.?search"
    Then all three variations are matched

  Scenario: Search defaults to limit of 20 matches
    Given sessions contain 50 messages matching "TODO"
    When the agent calls SessionSearch with action "search" and query "TODO" and no limit parameter
    Then the result contains at most 20 matches

  Scenario: Search with no matches returns empty result
    Given no sessions contain the text "nonexistent-query-xyz"
    When the agent calls SessionSearch with action "search" and query "nonexistent-query-xyz"
    Then the result indicates no matches found
    And the result is a valid structured response, not an error

  # --- show action ---

  Scenario: Show current session by default
    Given the agent is running in a session
    When the agent calls SessionSearch with action "show" and no session_id
    Then the result contains the current session's full conversation
    And messages are in chronological order

  Scenario: Show session with explicit current keyword
    Given the agent is running in a session
    When the agent calls SessionSearch with action "show" and session_id "current"
    Then the result is identical to calling show with no session_id

  Scenario: Show specific session by UUID
    Given a session exists with ID "7e0358a4-3395-4ee3-9a4b-62575d625b8c"
    When the agent calls SessionSearch with action "show" and session_id "7e0358a4-3395-4ee3-9a4b-62575d625b8c"
    Then the result contains that session's full conversation with messages in order

  Scenario: Show session resolves blob references
    Given a session contains messages with blob references to large content
    When the agent calls SessionSearch with action "show" for that session
    Then blob references are resolved to their actual content in the output

  Scenario: Show session reassembles streaming chunks
    Given a session contains an assistant message stored as raw streaming chunks with "[Thinking: partial text...]" markers and "[Tool: Read]" markers and text fragments split mid-word
    When the agent calls SessionSearch with action "show" for that session
    Then thinking chunks are merged into coherent thinking sections
    And tool invocations are preserved as structured markers
    And text fragments are concatenated into readable prose

  Scenario: Show session truncates long messages
    Given a session contains a message with 10000 characters of content
    When the agent calls SessionSearch with action "show" for that session
    Then that message is truncated to 5000 characters in the output

  Scenario: Show session with user_only filter
    Given a session contains both user and assistant messages
    When the agent calls SessionSearch with action "show" with user_only true
    Then only user messages are included in the result

  Scenario: Show session with max_turns limit
    Given a session contains 100 messages
    When the agent calls SessionSearch with action "show" with max_turns 10
    Then only the last 10 messages are included in the result

  Scenario: Show non-existent session returns error
    Given no session exists with ID "00000000-0000-0000-0000-000000000000"
    When the agent calls SessionSearch with action "show" and session_id "00000000-0000-0000-0000-000000000000"
    Then the tool returns an error indicating the session was not found

  # --- structural ---

  Scenario: SessionSearch output is structured JSON
    Given the persistence layer contains sessions
    When any SessionSearch action is called
    Then the result is valid structured JSON that can be parsed programmatically

  Scenario: SessionSearch uses persistence layer directly
    Given the SessionSearch tool is compiled as native Rust
    When any SessionSearch action is invoked
    Then data is read from MessageStore, HistoryStore, and BlobStore directly
    And no Python or bash subprocess is spawned

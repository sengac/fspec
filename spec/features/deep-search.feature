@done
@rig
@rust
@tools
@RLM-001
Feature: Deep Search Tool — Ephemeral Sub-Agent for Scoped Corpus Analysis
  """
  Tool is wired into all 5 providers' create_rig_agent() — same pattern as SessionSearch: import DeepSearchTool from codelet_tools, add .tool(DeepSearchTool::new(session_id)) to each provider's AgentBuilder chain. Verified locations: claude.rs:530, openai.rs:338, gemini.rs:172, codex/mod.rs:364, zai.rs:247.
  AMGR-001 (SessionSearch) provides the foundation for session history access. The implementation pattern: (1) Tool struct in rust/tools/src/session_search/ with types.rs (SessionSearchAction enum, args, results), handler.rs (per-session handler storage via static RwLock<HashMap<Uuid, Handler>>), reassembly.rs (streaming chunk reconstruction). (2) Handler registered in session_manager.rs via set_session_search_handler(session_id, handler) before agent run, cleaned up after. (3) The handler closure captures project_path AND compaction_trimming: Arc<AtomicBool> — the compaction_trimming flag controls whether Layer 0 trimming (CMPCT-010) is applied to SessionSearch results during agent-controlled compaction. (4) inject_summary (CMPCT-008/009) follows the exact same handler pattern — InjectSummaryTool in rust/tools/src/inject_summary.rs with set_inject_summary_handler/has_inject_summary_handler. DeepSearch should follow this same module pattern for its tool structure but does NOT need inject_summary (read-only sub-agent).
  DeepSearch's SessionSearch handler is created via rust/napi/src/session_search_handler.rs::create_handler(project_path, compaction_trimming). The project_path comes from the parent session's project context (same as the parent agent's handler). compaction_trimming must be Arc::new(AtomicBool::new(false)) since the ephemeral sub-agent never does compaction. This avoids the NAPI boundary problem since the handler is pure Rust — no ThreadsafeFunction needed.
  """

  Background: User Story
    As a developer using codelet
    I want to invoke a DeepSearch tool that spawns an ephemeral sub-agent to explore a scoped corpus
    So that I can get answers about large codebases without manually reading hundreds of files

  @tool-trait
  Scenario: DeepSearch implements rig::tool::Tool trait
    Given the DeepSearch tool struct exists in the rust/tools/src/deep_search module
    When the rig agent builder includes DeepSearchTool::new(session_id)
    Then DeepSearch has NAME = "DeepSearch"
    And DeepSearch has Args type = DeepSearchArgs with query (required) and scope (optional Vec<String>) and max_depth (optional usize)
    And DeepSearch has Output type = String
    And DeepSearch has Error type = ToolError
    And the definition() returns a JSON schema describing query, scope, and max_depth parameters

  @code-search
  Scenario: Search code files within a scoped directory
    Given a project with source files in "src/auth/"
    And the parent agent has a running session
    When the parent agent calls DeepSearch with query "How is authentication handled?" and scope ["src/auth/"]
    Then the sub-agent is spawned as an ephemeral instance with a fresh Uuid
    And the sub-agent receives a system prompt describing the code scope as "src/auth/"
    And the sub-agent uses Read, Grep, or AstGrep to explore the scoped files
    And the sub-agent returns a text answer synthesized from reading the auth source files
    And the parent agent receives the answer as the DeepSearch tool result

  @narrow-scope
  Scenario: Narrow scope restricts sub-agent to specific file
    Given a project with many source files
    When the parent agent calls DeepSearch with scope ["rust/tools/src/read.rs"]
    Then the sub-agent's system prompt describes only "rust/tools/src/read.rs" as in scope
    And the sub-agent explores only within the declared scope
    And the sub-agent does not read files outside the scope

  @depth-limit
  Scenario: Max depth limits tool call rounds
    Given a project with source files in "src/"
    When the parent agent calls DeepSearch with scope ["src/"] and max_depth 5
    Then the sub-agent's RigAgent is constructed with max_depth = 5
    And the sub-agent stops after 5 rounds of tool calls
    And the result includes the answer the sub-agent was able to produce within the depth limit

  @depth-limit
  @default
  Scenario: Default max depth is 50
    Given no max_depth is specified in the DeepSearch call
    When the sub-agent is constructed
    Then the RigAgent max_depth is set to 50
    And the sub-agent will not exceed 50 rounds of tool calls

  @session-search
  Scenario: Search session history combined with code scope
    Given a project with source files in "src/auth/"
    And session history exists for the current project
    When the parent agent calls DeepSearch with query "What changed in session 7e0358a4?" and scope ["src/auth/"]
    Then the sub-agent has both code exploration tools and SessionSearch as available tools
    And the sub-agent can use SessionSearch to load session "7e0358a4"
    And the sub-agent can use Read and Grep on "src/auth/" to correlate code with session discussion
    And the sub-agent returns an answer combining information from both sources

  @session-search
  Scenario: Search across multiple sessions without code scope
    Given session history exists for the current project
    When the parent agent calls DeepSearch with query "Find all sessions where we discussed compaction strategy" and no scope
    Then the sub-agent has SessionSearch as an available tool
    And the sub-agent uses SessionSearch with action "search" and query "compaction" to find relevant sessions
    And the sub-agent uses SessionSearch with action "show" to drill into the most relevant sessions
    And the sub-agent returns an answer synthesized across multiple sessions

  @system-prompt
  Scenario: System prompt describes available tools and code scope
    Given a DeepSearch call with scope ["src/"]
    When the sub-agent is constructed
    Then the system prompt describes SessionSearch for session history exploration
    And the system prompt describes Read, Grep, AstGrep, Glob, Ls, and Bash for code exploration
    And the system prompt specifies the code scope as "src/"
    And the system prompt instructs the sub-agent to explore strategically rather than reading all files

  @error-handling
  Scenario: Missing query returns error
    When the parent agent calls DeepSearch with no query
    Then the tool returns an error indicating that a query is required

  @optional-scope
  Scenario: Scope is optional for session-only queries
    Given session history exists for the current project
    When the parent agent calls DeepSearch with query "What did we discuss yesterday?" and no scope
    Then the sub-agent is constructed with SessionSearch only for data access
    And the system prompt does not describe any code paths
    And the sub-agent can answer using session history alone

  @ephemeral
  Scenario: Sub-agent is ephemeral with no persistence
    When a DeepSearch tool call completes
    Then no session record is persisted for the sub-agent
    And no worktree is created for the sub-agent
    And no NAPI boundary is crossed during sub-agent execution
    And the sub-agent's session_id is a fresh Uuid::new_v4() not shared with the parent

  @session-handler-lifecycle
  Scenario: SessionSearch handler registered and cleaned up
    Given the parent session has a project_path
    When the DeepSearch tool creates the ephemeral sub-agent
    Then a SessionSearch handler is created via create_handler(project_path, Arc::new(AtomicBool::new(false)))
    And compaction_trimming is always false for the ephemeral sub-agent
    And the handler is registered via set_session_search_handler(ephemeral_session_id, handler)
    And after RigAgent::prompt() completes, the handler is cleaned up via set_session_search_handler(ephemeral_session_id, None)

  @read-only-tools
  Scenario: Sub-agent has only read-only search tools
    When the sub-agent is constructed for a DeepSearch call
    Then the sub-agent has these tools: Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch
    And the sub-agent does NOT have Write tool
    And the sub-agent does NOT have Edit tool
    And the sub-agent does NOT have Bridge tool
    And the sub-agent does NOT have Fspec tool
    And the sub-agent does NOT have WebSearch tool
    And the sub-agent does NOT have ConnectMcp tool
    And the sub-agent does NOT have AstGrepRefactor tool
    And the sub-agent does NOT have inject_summary tool

  @non-streaming
  Scenario: Sub-agent uses non-streaming RigAgent::prompt()
    When the DeepSearch tool executes
    Then the sub-agent calls RigAgent::prompt(query) not the streaming variant
    And the call blocks until the sub-agent finishes all tool calls and produces a final answer
    And the final answer string is returned as the DeepSearch tool result

  @credentials
  Scenario: Sub-agent reuses parent session credentials
    Given the parent session has environment variables set for the Claude API
    When the DeepSearch tool creates a ProviderManager
    Then the ProviderManager reads credentials from the existing environment variables
    And no additional credential configuration is needed

  @provider-wiring
  Scenario: DeepSearch tool wired into provider agent builders
    Given all 5 provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI)
    When each provider's create_rig_agent() method builds an agent
    Then DeepSearchTool::new(session_id) is included in the tool chain
    And the parent agent can invoke DeepSearch like any other tool

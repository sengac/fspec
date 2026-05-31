@done
@RPC-018
@rust
@source-shape
@rpc
@tui
Feature: RPC-018 source-shape regression for the AgentView chrome port + shared types + new RPC methods
  """
  RPC-018 introduces several new source artefacts that downstream cards
  (RPC-019..022) rely on. This feature pins the file layout + symbol
  surface so that future refactors cannot silently regress the
  integration shape:

  1. `codelet/rpc-types/src/lib.rs` — three new types
  `ModelInfo`, `ThinkingLevel`, `WorkspaceInfo` (cfg-gated for napi).
  2. `codelet/rpc/src/lib.rs` — `FspecService` trait gains three new
  methods: `get_model_info`, `get_thinking_level`,
  `get_workspace_info`.
  3. `codelet/core/src/session_manager_handle.rs` — trait gains
  `get_model_info` + `get_thinking_level` with default impls
  returning safe defaults.
  4. `codelet/fspec-tui/src/transport/mod.rs` — `FspecBackend` trait
  declares the same three methods (one-line tarpc delegates).
  5. `codelet/fspec-tui/src/transport/embedded.rs` +
  `codelet/fspec-tui/src/transport/websocket.rs` — both implement
  the three new trait methods.
  6. `codelet/fspec-tui/src/views/agent/header.rs` +
  `codelet/fspec-tui/src/views/agent/footer.rs` — new widget
  modules under 300 LoC each.
  7. `codelet/fspec-tui/src/views/agent.rs` OR
  `codelet/fspec-tui/src/views/agent/mod.rs` — orchestrator stays
  under 300 LoC.
  8. `codelet/napi/src/git.rs` (or sibling) — additive
  `napi::get_workspace_info(cwd)` export delegates to
  `codelet_git::status::get_current_branch`.
  9. `codelet/napi/src/session_manager.rs` (or sibling) — additive
  `napi::get_model_info(session_id)` export delegates through the
  SessionManagerHandle path.

  Existing TS code paths
  (src/tui/components/SessionHeader.tsx,
  src/tui/components/SessionFooter.tsx,
  src/tui/utils/tokenStateUtils.ts,
  src/tui/store/modelStore.ts) are NOT touched.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the RPC-018 source layout to be locked in by a regression test
    So that future cards inheriting the FspecBackend / SharedFspecService surface continue to find ModelInfo / ThinkingLevel / WorkspaceInfo where they expect them

  Scenario: New shared types live in rpc-types
    Given codelet/rpc-types/src/lib.rs after RPC-018 lands
    Then the file contains the substring "pub struct ModelInfo"
    And the file contains the substring "pub display_name: String"
    And the file contains the substring "pub supports_reasoning: bool"
    And the file contains the substring "pub supports_vision: bool"
    And the file contains the substring "pub context_window: u32"
    And the file contains the substring "pub enum ThinkingLevel"
    And the file contains the substring "pub struct WorkspaceInfo"
    And the file contains the substring "pub cwd: String"
    And the file contains the substring "pub git_branch: Option<String>"

  Scenario: FspecService trait gains three new RPC methods
    Given codelet/rpc/src/lib.rs after RPC-018 lands
    Then the file contains the substring "async fn get_model_info(session_id: SessionId) -> ModelInfo"
    And the file contains the substring "async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel"
    And the file contains the substring "async fn get_workspace_info() -> WorkspaceInfo"
    And the FspecServiceImpl body contains the substring "codelet_git::status::get_current_branch"

  Scenario: SessionManagerHandle trait gains get_model_info / get_thinking_level with default impls
    Given codelet/core/src/session_manager_handle.rs after RPC-018 lands
    Then the file contains the substring "fn get_model_info(&self, session_id: &SessionId) -> ModelInfo"
    And the file contains the substring "fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel"
    And both methods have default implementations returning the safe defaults

  Scenario: FspecBackend trait declares the three new methods
    Given codelet/fspec-tui/src/transport/mod.rs after RPC-018 lands
    Then the file contains the substring "async fn get_model_info"
    And the file contains the substring "async fn get_thinking_level"
    And the file contains the substring "async fn get_workspace_info"

  Scenario: Both transports implement the three new FspecBackend methods
    Given the codelet/fspec-tui crate after RPC-018 lands
    Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_model_info"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_thinking_level"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_workspace_info"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_model_info"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_thinking_level"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_workspace_info"

  Scenario: New agent widget modules exist as separate files
    Given the codelet/fspec-tui crate after RPC-018 lands
    Then the file codelet/fspec-tui/src/views/agent/header.rs exists
    And the file codelet/fspec-tui/src/views/agent/footer.rs exists

  Scenario: New and modified agent modules stay under 300 lines
    Given the directory codelet/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator (or views/agent/mod.rs)
    When a test counts the line-count of every .rs file
    Then every file in views/agent/ has fewer than 300 lines
    And the orchestrator file (views/agent.rs OR views/agent/mod.rs) has fewer than 300 lines

  Scenario: Action enum gains three new variants
    Given codelet/fspec-tui/src/components/mod.rs after RPC-018 lands
    Then the file contains the substring "ModelInfoLoaded"
    And the file contains the substring "ThinkingLevelLoaded"
    And the file contains the substring "WorkspaceInfoLoaded"

  Scenario: NAPI surface exposes additive get_workspace_info export
    Given codelet/napi/src/git.rs after RPC-018 lands
    Then the file contains the substring "pub fn get_workspace_info"
    And the file contains the substring "codelet_git::status::get_current_branch"

  Scenario: NAPI surface exposes additive get_model_info export
    Given codelet/napi/src/session_manager.rs (or a sibling file) after RPC-018 lands
    Then the codelet/napi/src tree contains the substring "pub fn get_model_info"

  Scenario: Existing TS AgentView chrome files are untouched
    Given the project root after RPC-018 lands
    Then the file src/tui/components/SessionHeader.tsx exists
    And the file src/tui/components/SessionFooter.tsx exists
    And the file src/tui/utils/tokenStateUtils.ts exists
    And the file src/tui/store/modelStore.ts exists

  Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
    Given the directory codelet/fspec-tui/src/views/ (including views/agent/) after RPC-018 lands
    When a test scans every *.rs file
    Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
    And no file constructs `tokio::runtime::Builder` or `Runtime::new()`

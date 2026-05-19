@done
@RPC-022
@rust
@source-shape
@rpc
@tui
Feature: RPC-022 source-shape regression for the modal dialogs port + shared types + new RPC methods

  """
  RPC-022 introduces several new source artefacts. This feature pins
  the file layout + symbol surface so future refactors cannot silently
  regress the integration shape:

    1. `codelet/rpc-types/src/lib.rs` — two new types
       `ProviderInfo` and `ModelEntry` (cfg-gated for napi).
    2. `codelet/rpc/src/lib.rs` — `FspecService` trait gains five new
       methods: `list_providers`, `set_session_model`,
       `set_thinking_level`, `get_session_role`, `set_session_role`.
    3. `codelet/core/src/session_manager_handle.rs` — trait gains
       `set_model`, `set_thinking_level`, `get_role`, `set_role`,
       `list_providers` with default impls returning safe defaults.
    4. `codelet/fspec-tui/src/transport/mod.rs` — `FspecBackend` trait
       declares the same five methods.
    5. `codelet/fspec-tui/src/transport/embedded.rs` +
       `codelet/fspec-tui/src/transport/websocket.rs` — both implement
       the five new trait methods.
    6. `codelet/fspec-tui/src/components/model_selector_dialog.rs` +
       `codelet/fspec-tui/src/components/thinking_level_dialog.rs` —
       new modal dialog modules.
    7. `codelet/fspec-tui/src/views/agent/role_banner.rs` — new inline
       widget module under views/agent/.
    8. `codelet/fspec-tui/src/app/dispatch_rpc022.rs` — new dispatch
       helper module.
    9. `codelet/fspec-tui/src/components/mod.rs::Priority` — gains a
       new `Foreground = 900` variant.

  Existing TS code paths
  (src/tui/components/ModelSelectorScreen.tsx,
  src/tui/components/ModelSelectorView.tsx,
  src/tui/components/ThinkingLevelDialog.tsx,
  src/tui/components/RoleBanner.tsx,
  src/tui/store/modelStore.ts) are NOT touched.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the RPC-022 source layout locked in by a regression test
    So that future cards inheriting the FspecBackend / SharedFspecService surface continue to find ProviderInfo / ModelEntry / the five new RPC methods where they expect them

  @shared-types
  Scenario: New shared types live in rpc-types
    Given codelet/rpc-types/src/lib.rs after RPC-022 lands
    Then the file contains the substring "pub struct ProviderInfo"
    And the file contains the substring "pub key: String"
    And the file contains the substring "pub display_name: String"
    And the file contains the substring "pub models: Vec<ModelEntry>"
    And the file contains the substring "pub struct ModelEntry"
    And the file contains the substring "pub id: String"
    And the file contains the substring "pub context_window: u32"
    And the file contains the substring "pub supports_reasoning: bool"
    And the file contains the substring "pub supports_vision: bool"
    And the file contains the substring "pub is_custom: bool"

  @rpc-trait
  Scenario: FspecService trait gains five new RPC methods
    Given codelet/rpc/src/lib.rs after RPC-022 lands
    Then the file contains the substring "async fn list_providers() -> Vec<ProviderInfo>"
    And the file contains the substring "async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> Result<(), String>"
    And the file contains the substring "async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>"
    And the file contains the substring "async fn get_session_role(session_id: SessionId) -> Option<String>"
    And the file contains the substring "async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>"

  @session-manager-handle
  Scenario: SessionManagerHandle trait gains the new methods with default impls
    Given codelet/core/src/session_manager_handle.rs after RPC-022 lands
    Then the file contains the substring "fn list_providers(&self) -> Vec<ProviderInfo>"
    And the file contains the substring "fn set_model(&self, session_id: &SessionId, provider_id: &str, model_id: &str) -> Result<(), String>"
    And the file contains the substring "fn set_thinking_level(&self, session_id: &SessionId, level: ThinkingLevel) -> Result<(), String>"
    And the file contains the substring "fn get_role(&self, session_id: &SessionId) -> Option<String>"
    And the file contains the substring "fn set_role(&self, session_id: &SessionId, role: Option<String>) -> Result<(), String>"
    And each of the five methods has a default implementation returning the safe default (empty Vec / None / Ok(()))

  @fspec-backend
  Scenario: FspecBackend trait declares the five new methods
    Given codelet/fspec-tui/src/transport/mod.rs after RPC-022 lands
    Then the file contains the substring "async fn list_providers"
    And the file contains the substring "async fn set_session_model"
    And the file contains the substring "async fn set_thinking_level"
    And the file contains the substring "async fn get_session_role"
    And the file contains the substring "async fn set_session_role"

  @transport-impl
  Scenario: Both transports implement the five new FspecBackend methods
    Given the codelet/fspec-tui crate after RPC-022 lands
    Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn list_providers"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_session_model"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_thinking_level"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_session_role"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_session_role"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn list_providers"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_session_model"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_thinking_level"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_session_role"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_session_role"

  @new-modules
  Scenario: New modal dialog modules and dispatch helper exist
    Given the codelet/fspec-tui crate after RPC-022 lands
    Then the file codelet/fspec-tui/src/components/model_selector_dialog.rs exists
    And the file codelet/fspec-tui/src/components/thinking_level_dialog.rs exists
    And the file codelet/fspec-tui/src/views/agent/role_banner.rs exists
    And the file codelet/fspec-tui/src/app/dispatch_rpc022.rs exists

  @line-budget
  Scenario: New RPC-022 modules stay under 300 lines
    Given the new files introduced by RPC-022
    When a test counts the line-count of every .rs file
    Then codelet/fspec-tui/src/components/model_selector_dialog.rs has fewer than 300 lines
    And codelet/fspec-tui/src/components/thinking_level_dialog.rs has fewer than 300 lines
    And codelet/fspec-tui/src/views/agent/role_banner.rs has fewer than 300 lines
    And codelet/fspec-tui/src/app/dispatch_rpc022.rs has fewer than 300 lines

  @priority-enum
  Scenario: Priority enum gains a Foreground variant numbered 900
    Given codelet/fspec-tui/src/components/mod.rs after RPC-022 lands
    Then the Priority enum contains the variant "Foreground = 900"
    And Priority::Foreground sorts strictly between Priority::High (800) and Priority::Critical (1000)

  @action-enum
  Scenario: Action enum gains the new RPC-022 variants
    Given codelet/fspec-tui/src/components/mod.rs after RPC-022 lands
    Then the file contains the substring "ModelSelected"
    And the file contains the substring "ThinkingLevelSelected"
    And the file contains the substring "SetSessionRole"
    And the file contains the substring "SessionRoleLoaded"
    And the file contains the substring "ListProvidersLoaded"
    And the file contains the substring "OpenModelDialog"
    And the file contains the substring "OpenThinkingDialog"

  @ts-untouched
  Scenario: Existing TS modal dialog files are untouched
    Given the project root after RPC-022 lands
    Then the file src/tui/components/ModelSelectorScreen.tsx exists
    And the file src/tui/components/ModelSelectorView.tsx exists
    And the file src/tui/components/ThinkingLevelDialog.tsx exists
    And the file src/tui/components/RoleBanner.tsx exists
    And the file src/tui/store/modelStore.ts exists

  @architecture-invariants
  Scenario: New view + component files do not directly import codelet_core / napi / tarpc / tokio_tungstenite
    Given the new RPC-022 files (model_selector_dialog.rs, thinking_level_dialog.rs, role_banner.rs)
    When a test scans each *.rs file
    Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
    And no file constructs `tokio::runtime::Builder` or `Runtime::new()`

@done
@session-management
@codelet
@rust
@infrastructure
@napi
@RPC-044
Feature: Wire codelet-sessions::SessionManager into codelet-fspec build_service

  # SharedFspecService::with_session_manager already delegates chunks_rx/logs_rx/status_changes_rx
  # to the SessionManagerHandle when one is attached (see codelet/rpc/src/lib.rs lines 526-580).
  # NO additional fan-out task is needed in this card — the bridging is already a runtime property
  # of the SharedFspecService constructor.
  #
  # codelet_sessions::SessionManager::new() takes no arguments. Persistence is initialised via the
  # global codelet_common::set_data_directory(~/.fspec) call already present at the top of
  # build_service (RPC-025). The SessionManager picks up that data dir lazily on its first
  # persistence-touching call.
  #
  # The default NoopSessionManagerHooks is left in place for the fspec binary. The full
  # agent-loop / scheduler / footer-poller / IsolationStateChange hooks are wired in RPC-045+
  # when the AgentView is connected to the new RPC surface. In the meantime, sessions created
  # from the fspec binary will be tracked but will not spawn a backing agent loop until the
  # hooks are populated.
  Background: User Story
    As a Rust developer wiring codelet-sessions into the codelet-fspec binary
    I want codelet/fspec/src/common.rs::build_service to construct a real codelet_sessions::SessionManager and pass it via SharedFspecService::with_session_manager, with codelet-sessions declared as a Cargo dependency
    So that the fspec binary in all three modes runs real agent sessions through the NAPI-free codelet-sessions crate

  @rule:build_service_constructs_session_manager
  @wiring
  Scenario: build_service constructs a real SessionManager and passes it via with_session_manager
    Given the RPC-044 changes are applied to the codelet workspace
    When I open `codelet/fspec/src/common.rs`
    Then the file contains the literal substring `use codelet_sessions::SessionManager`
    And the file contains the literal substring `use codelet_core::SessionManagerHandle`
    And the `build_service` function constructs `let session_manager: Arc<dyn SessionManagerHandle> = Arc::new(SessionManager::new());`
    And the `build_service` function calls `SharedFspecService::with_session_manager(watcher, session_manager)` instead of `SharedFspecService::new(watcher)`
    And the `set_data_directory` call still appears before the SessionManager construction

  @rule:cargo_toml_session_dep
  @cargo
  @manifest
  Scenario: codelet/fspec/Cargo.toml adds codelet-sessions and does not add codelet-napi
    Given the RPC-044 changes are applied to the codelet workspace
    When I open `codelet/fspec/Cargo.toml`
    Then the `[dependencies]` table contains `codelet-sessions.workspace = true` or `codelet-sessions = { workspace = true }`
    And the file contains zero occurrences of the literal substring `codelet-napi`

  @rule:build_service_returns_handle_attached_service
  @unit
  Scenario: build_service returns a SharedFspecService whose session-manager handle has been attached
    Given the RPC-044 changes are applied to the codelet workspace
    When `build_service` is invoked against a temp workspace
    Then `service.cwd()` returns `Some(temp_workspace_path)` as before
    And the literal substring `SharedFspecService::with_session_manager(watcher, session_manager)` is present in codelet/fspec/src/common.rs
    And a chunk sent through `service.chunks_tx()` is received via `service.chunks_rx()` proving the SessionManager broadcast is live

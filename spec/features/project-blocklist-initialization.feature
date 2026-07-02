@done
@rust
@tools
@security
@RPC-407
Feature: Project blocklist initialization at service startup
  """
  Fix seam: codelet/fspec/src/common.rs::build_service (chokepoint for daemon.rs:38 and combined.rs:35; client mode does not call it and inherits the daemon). Call codelet_tools::blocklist::init_blocklist(Some(workspace)) alongside the existing set_data_directory bootstrap.
  Tests: BLOCKLIST_PROJECT_ROOT is process-global, so integration tests use serial_test::serial and restore state via init_blocklist(None) + clear_session_allowances() on exit. codelet-fspec needs serial_test added as a dev-dependency (already a workspace dep used by codelet-tools).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. build_service must call codelet_tools::blocklist::init_blocklist(Some(workspace)) during startup so both daemon and combined modes get the project blocklist root
  #   2. init_blocklist is idempotent/re-init safe; calling it again at startup must not change napi path behavior
  #   3. The project root used for the blocklist is the same workspace path build_service already receives (explicit --workspace or current dir fallback)
  #   4. Setting the root once at startup is sufficient because check_bash_command/check_file_path hot-reload the config file on every check
  #
  # EXAMPLES:
  #   1. Workspace contains .fspec/blocklist.json with a block rule pattern sentinel-rpc407; after build_service(workspace), check_bash_command("sentinel-rpc407", uuid) returns Err(BlockedError) with the project rule id
  #   2. Negative control: after re-initializing the blocklist with a different root (no project config), the same sentinel command is allowed — proving the block came from the project config
  #   3. Source-shape guard: codelet/fspec/src/common.rs build_service contains a literal init_blocklist call so future entry points cannot silently drop it (covers both daemon and combined modes which both call build_service)
  #
  # ========================================
  Background: User Story
    As a developer using the standalone Rust fspec binary
    I want to have my project's .fspec/blocklist.json rules loaded at service startup
    So that project-level block and prompt rules protect the agent session just like they do in the legacy napi shell

  Scenario: Project blocklist rules are enforced after service startup
    Given a workspace containing ".fspec/blocklist.json" with a block rule for pattern "sentinel-rpc407"
    When the fspec binary builds its service via build_service against that workspace
    Then running a command matching "sentinel-rpc407" is blocked with the project rule id
    And the blocked error carries the reason from the project blocklist rule

  Scenario: Blocking comes from the project config and not any other source
    Given the project blocklist was initialized against a workspace with a "sentinel-rpc407" block rule
    When the blocklist is re-initialized against a different workspace without a project blocklist
    Then running a command matching "sentinel-rpc407" is allowed

  Scenario: Startup seam covers both daemon and combined modes
    Given the codelet-fspec binary crate after RPC-407 lands
    When I open codelet/fspec/src/common.rs
    Then the build_service function contains a literal init_blocklist call
    And both daemon.rs and combined.rs reach build_service so neither mode can skip blocklist initialization

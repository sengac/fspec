@done
@RPC-017
@rust
@rpc
@tui
@work-units
Feature: RPC-017 production fspec binary build_service cwd attachment
  """
  RPC-017 (split from rpc017-cross-transport-parity): the inline unit
  test that asserts `codelet-fspec`'s `common::build_service` chains
  `.with_cwd(workspace)` lives inside `src/common.rs` because
  `build_service` is private to the binary crate (no `[lib]` target).

  Splitting this scenario into its own feature satisfies the 1:1
  feature-to-test-file rule (VAL-005): this feature maps to the inline
  test in `codelet/fspec/src/common.rs`, while
  rpc017-cross-transport-parity maps to `codelet/fspec-tui/tests/move_work_unit_rpc017.rs`.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want common::build_service(workspace) to attach the workspace cwd to the SharedFspecService
    So that move_work_unit_up/_down and checkpoint_counts succeed at runtime in both combined and daemon modes

  Scenario: Production fspec binary's build_service attaches workspace cwd to SharedFspecService
    Given the codelet-fspec binary crate after RPC-017 lands
    When common::build_service(workspace) is invoked against a temp workspace path
    Then the returned Arc<SharedFspecService>::cwd() returns Some equal to that workspace path
    And codelet/fspec/src/common.rs contains the substring ".with_cwd(workspace.to_path_buf())"

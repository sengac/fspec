@done
@tui
@session-management
@source-shape
@rust
@rpc
@RPC-050
Feature: /detach and work-unit binding source-shape invariants
  """
  Source-shape regression pins for the RPC-050 wiring:
  * No file under `codelet/fspec-tui/src/` matches "codelet_napi" (post-RPC-002 invariant).
  * Every file under `codelet/fspec-tui/src/app/`, `codelet/fspec-tui/src/views/agent/`, and `codelet/fspec-tui/src/store/agent_view/` is strictly less than 300 lines of code.
  * `codelet/fspec-tui/src/app/dispatch.rs` is strictly less than 300 lines of code.
  * `codelet/fspec-tui/src/app/dispatch_rpc020.rs` is strictly less than 300 lines of code.
  * The new dispatch_rpc050.rs file exists and declares the three RPC-050 helpers.
  * The components::Action enum declares the three new variants.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. No new file may push the directory ceiling above 300 LoC
  #   2. The new dispatch helpers must live in app/dispatch_rpc050.rs (not in dispatch.rs or dispatch_rpc020.rs)
  #   3. The Action enum must declare AttachWorkUnitToSession, WorkUnitAttached, WorkUnitDetached
  #
  # ========================================
  Background: User Story
    As a fspec developer enforcing the RPC-002 architectural invariants
    I want the RPC-050 wiring to land without breaking the 300-LoC ceiling or the no-codelet-napi rule
    So that the dependency-rule regression tests continue to pass through the rest of the RPC-030 roadmap

  Scenario: No codelet_napi reference and the 300-LoC ceiling holds
    Given the codelet/fspec-tui/src/ tree after the RPC-050 changes
    Then no file under codelet/fspec-tui/src/ matches "codelet_napi"
    And every file under codelet/fspec-tui/src/app/, codelet/fspec-tui/src/views/agent/, and codelet/fspec-tui/src/store/agent_view/ is strictly less than 300 lines of code
    And codelet/fspec-tui/src/app/dispatch.rs is strictly less than 300 lines of code
    And codelet/fspec-tui/src/app/dispatch_rpc020.rs is strictly less than 300 lines of code

  Scenario: components::Action declares the new RPC-050 variants
    Given codelet/fspec-tui/src/components/mod.rs after RPC-050 lands
    Then the file declares "AttachWorkUnitToSession(" as an Action variant
    And the file declares "WorkUnitAttached(" as an Action variant
    And the file declares "WorkUnitDetached(" as an Action variant

  Scenario: dispatch_rpc050.rs declares the new RPC-050 helpers
    Given codelet/fspec-tui/src/app/dispatch_rpc050.rs after RPC-050 lands
    Then the file declares "handle_attach_work_unit_to_session"
    And the file declares "handle_work_unit_attached"
    And the file declares "handle_work_unit_detached"
    And the file declares "handle_slash_detach"

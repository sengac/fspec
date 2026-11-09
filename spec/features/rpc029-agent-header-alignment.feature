@BUG-582
@done
@RPC-029
@REFAC-051
@rust
@tui
@ui
@agent-view
@header
@ui-enhancement
Feature: RPC-029 AgentView session header alignment with TS Ink original
  """
  Split from rpc029-agent-structure-alignment.feature by REFAC-051 so each feature's one test file stays under 300
  lines. Covers the header half of RPC-029: the work-unit prefix between the session number and the model
  ('#N (ID: status): model'), the dark-grey (RGB 0x33, 0x33, 0x33) header and footer rows, and the multi-span header
  colours (cyan-bold prefix + work unit + model, red-bold [DEBUG], green [ISOLATED]).

  Pair: render tests live in rust/fspec-tui/tests/rpc029_agent_header_alignment.rs; shared render helpers live in
  rust/fspec-tui/tests/rpc029_support/mod.rs.
  """

  Background: User Story
    As a Rust TUI developer
    I want to have AgentView render structurally identical to the canonical TS Ink original
    So that users see consistent layout, theming, and header semantics across Rust and TS frontends

  Scenario: Header inserts work-unit prefix between session number and model
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And the store's current_work_unit_id is "RPC-029"
    And the store's current_work_unit_status is "implementing"
    When the App renders AgentView against a 120x20 TestBackend
    Then the rendered buffer's top row contains the substring "#1 (RPC-029: implementing): claude-sonnet-4"

  Scenario: Header omits work-unit prefix when no work unit is set
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And the store has no current_work_unit_id
    When the App renders AgentView against a 120x20 TestBackend
    Then the rendered buffer's top row contains the substring "#1: claude-sonnet-4"
    And the rendered buffer's top row does NOT contain the substring "(RPC"

  Scenario: Header and footer rows paint dark grey #333333 background on every cell
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    When the App renders AgentView against an 80x20 TestBackend
    Then every cell of the header row has background color RGB(0x33, 0x33, 0x33)
    And every cell of the footer row has background color RGB(0x33, 0x33, 0x33)

  Scenario: Header [DEBUG] badge paints red-bold when debug enabled
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And the SessionHeader's is_debug_enabled field is true
    When the SessionHeader renders against an 80x1 buffer
    Then the rendered buffer's row 0 contains the substring "[DEBUG]"
    And the cell containing the "D" of "[DEBUG]" has foreground color Red and Bold modifier

  Scenario: Header [ISOLATED] badge paints green when session is isolated
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And the SessionHeader's is_isolated field is true
    When the SessionHeader renders against an 80x1 buffer
    Then the rendered buffer's row 0 contains the substring "[ISOLATED]"
    And the cell containing the "I" of "[ISOLATED]" has foreground color Green

  Scenario: Header prefix + work unit + model run paints cyan and bold
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And the store's current_work_unit_id is "RPC-029"
    And the store's current_work_unit_status is "implementing"
    When the App renders AgentView against a 120x20 TestBackend
    Then the cell containing the "c" of "claude-sonnet-4" in the header row has foreground color Cyan and Bold modifier

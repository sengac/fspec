@done
@RPC-029
@rust
@tui
@ui
@rpc
@agent-view
@header
@footer
@ui-enhancement
Feature: RPC-029 AgentView structure alignment with TS Ink original

  """
  RPC-029 — make the Rust ratatui AgentView render structurally identical
  to the canonical TS Ink AgentView (src/tui/components/AgentView.tsx).

  Structural changes from the previous RPC-018/RPC-019 layout:
    - The scrollback area no longer has a Block border or "Agent — <sid>" title.
    - The input area no longer has any border at all — only horizontal padding
      of 1 column on the left and right.
    - The footer row is now rendered ABOVE the input row (was below).
    - The header and footer rows each paint a dark grey background
      (RGB 0x33, 0x33, 0x33) on every cell, with paddingX=1.

  Header semantic changes:
    - The left text inserts a work-unit prefix between the session number
      and the model name: '#N (ID: status): model'.
    - The left text paints multi-span colours: cyan-bold prefix+work-unit
      +model, magenta [R], blue [V], dark-grey [Nk], red-bold [DEBUG],
      cyan [SELECT], yellow [T:<level>], green [ISOLATED].
    - The right text paints 'tokens: in↓ out↑' in dark-grey and the
      percent bracket in a context-fill colour (green<50, yellow<70,
      magenta<85, red>=85).

  Footer semantic changes:
    - The left side is now empty (the old hints 'Enter=send  Ctrl+C=interrupt
      ESC=back' are removed).
    - The branch glyph reverts from ⌥ (U+2325 OPTION KEY) to ⎇ (U+2387
      ALTERNATIVE KEY SYMBOL) to match TS canonical output.
    - The right side splits into two spans: dim/dark-grey cwd then cyan
      '[⎇ branch]' suffix.

  Out of scope (deferred): InputTransition character-by-character
  animation, inline pause/HITL/compaction indicators, tokens-per-second
  and reasoning-tokens 🧠 wiring (the badge code paths exist but default
  to None/0).

  Pair: render tests live in
  codelet/fspec-tui/tests/view_agent_unit_rpc029.rs.
  """

  Background: User Story
    As a Rust TUI developer
    I want to have AgentView render structurally identical to the canonical TS Ink original
    So that users see consistent layout, theming, and header semantics across Rust and TS frontends

  Scenario: Scrollback area has no border and no Agent title
    Given an AgentViewStore with current_session "s-1"
    And the AgentView has pushed one scrollback line "user> hi"
    When the App renders AgentView against an 80x20 TestBackend
    Then the rendered buffer does NOT contain the substring "┌"
    And the rendered buffer does NOT contain the substring "└"
    And the rendered buffer does NOT contain the substring "│"
    And the rendered buffer does NOT contain the substring " Agent — "

  Scenario: Input area has no border and prompt sits at padded column
    Given an empty AgentViewStore with no current_session
    When the App renders AgentView against an 80x20 TestBackend
    Then the input row contains the substring "> "
    And the input row does NOT contain the substring "│"
    And the cell at column 1 of the input row contains the character ">"

  Scenario: Footer row appears strictly above the input row
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    When the App renders AgentView against an 80x20 TestBackend
    Then the row containing the substring "/tmp/scratch" appears strictly above the row containing the green ">" prompt

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

  Scenario: Header and footer have horizontal padding of one column on both edges
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    And workspace is WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    When the App renders AgentView against an 80x20 TestBackend
    Then the first column of the header row contains no glyph
    And the last column of the header row contains no glyph
    And the first column of the footer row contains no glyph
    And the last column of the footer row contains no glyph

  Scenario: Footer left side is empty - no Enter=send / Ctrl+C / ESC=back hints
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    When the App renders AgentView against an 80x20 TestBackend
    Then the footer row does NOT contain the substring "Enter=send"
    And the footer row does NOT contain the substring "Ctrl+C"
    And the footer row does NOT contain the substring "ESC=back"

  Scenario: Footer branch glyph uses ⎇ U+2387 not ⌥ U+2325
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    When the App renders AgentView against an 80x20 TestBackend
    Then the footer row contains the substring "[⎇ main]"
    And the footer row does NOT contain the substring "[⌥"

  Scenario: Footer cwd span is dark-grey and bracketed branch span is cyan
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    When the App renders AgentView against an 80x20 TestBackend
    Then the cell at the cwd position of the footer row has foreground color DarkGray
    And the cell at the branch suffix position of the footer row has foreground color Cyan

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

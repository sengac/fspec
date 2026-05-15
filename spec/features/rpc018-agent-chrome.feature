@done
@RPC-018
@rust
@tui
@ui
@rpc
@ui-enhancement
@agent-view
@header
@footer
Feature: RPC-018 AgentView chrome — SessionHeader + SessionFooter widgets

  """
  RPC-018 (slice 1 of 4) — AgentView gains a 1-row SessionHeader at the
  top and a 1-row SessionFooter at the bottom, sandwiching the existing
  Scrollback (flex) and single-line Input (3 rows) from RPC-009/RPC-012.

  SessionHeader layout (mirrors src/tui/components/SessionHeader.tsx):
    Left:  `#N: <model display name> [R] [V] [Nk] [T:<level>]`
           - `#N:` only when current_session is populated (1-based index)
           - `[R]` only when supports_reasoning
           - `[V]` only when supports_vision
           - `[Nk]` only when context_window > 0 (compact form, e.g. `192k`)
           - `[T:<level>]` only when thinking_level != Off (Low/Med/High)
    Right: `tokens: <in>↓ <out>↑ [<fill>%]`

  SessionFooter layout (mirrors src/tui/components/SessionFooter.tsx):
    Left:  `Enter=send  Ctrl+C=interrupt  ESC=back` (kept from RPC-013)
    Right: `<cwd> [⌥ <branch>]`
           - cwd shortened with `~` when inside $HOME
           - `[⌥ <branch>]` segment omitted when not a git repo

  Token state for the current session is derived live from
  `StreamChunk::TokenUpdate { tokens: TokenTracker }` and
  `StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo }`
  events arriving via `Action::ChunkReceived`. All other chunk variants
  leave token state unchanged.

  No TypeScript code is modified. The TS Ink AgentView keeps its
  existing SessionHeader.tsx / SessionFooter.tsx / tokenStateUtils.ts /
  modelStore.ts implementations.

  Pair: render tests live in codelet/fspec-tui/tests/view_agent_unit_rpc018.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a 1-row SessionHeader + 1-row SessionFooter sandwiching the AgentView scrollback and input
    So that the Rust ratatui AgentView matches the Ink TS AgentView chrome with model badges, token deltas, cwd, and git branch

  Scenario: Empty AgentViewStore paints placeholder header and bare-cwd footer
    Given an empty AgentViewStore with no current_session, no model_info, no thinking_level, and no workspace snapshot
    When the App renders AgentView against an 80x20 TestBackend
    Then the rendered buffer's top row contains the substring "Agent"
    And the rendered buffer's top row contains the substring "tokens: 0↓ 0↑ [0%]"
    And the rendered buffer's bottom row contains the substring "Enter=send"
    And the rendered buffer's bottom row contains the substring "ESC=back"
    And the rendered buffer does NOT contain the substring "[R]"
    And the rendered buffer does NOT contain the substring "[V]"
    And the rendered buffer does NOT contain the substring "[T:"

  Scenario: Header paints model badges and thinking level when session has model info
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And model_info_by_session["s-1"] is ModelInfo { display_name: "Claude Opus 4.7", supports_reasoning: true, supports_vision: true, context_window: 192000 }
    And thinking_level_by_session["s-1"] is ThinkingLevel::High
    When the App renders AgentView against an 100x20 TestBackend
    Then the rendered buffer's top row contains the substring "#1:"
    And the rendered buffer's top row contains the substring "Claude Opus 4.7"
    And the rendered buffer's top row contains the substring "[R]"
    And the rendered buffer's top row contains the substring "[V]"
    And the rendered buffer's top row contains the substring "[192k]"
    And the rendered buffer's top row contains the substring "[T:High]"

  Scenario: Header right-side reflects TokenUpdate followed by ContextFillUpdate
    Given an AgentViewStore with current_session "s-1"
    And token_state_by_session["s-1"] is TokenState { input_tokens: 1234, output_tokens: 567, context_fill_pct: 45 }
    When the App renders AgentView against an 100x20 TestBackend
    Then the rendered buffer's top row contains the substring "tokens: 1234↓ 567↑ [45%]"

  Scenario: Footer abbreviates cwd to ~ inside $HOME and appends [⌥ branch] in a git repo
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/Users/rquast/projects/fspec", git_branch: Some("codelet-integration") }
    And the env var HOME is "/Users/rquast"
    When the App renders AgentView against a 100x20 TestBackend
    Then the rendered buffer's bottom row contains the substring "~/projects/fspec"
    And the rendered buffer's bottom row contains the substring "[⌥ codelet-integration]"
    And the rendered buffer's bottom row does NOT contain the substring "/Users/rquast/projects/fspec"

  Scenario: Footer omits the [⌥ ...] segment when the workspace is not a git repo
    Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    When the App renders AgentView against a 100x20 TestBackend
    Then the rendered buffer's bottom row contains the substring "/tmp/scratch"
    And the rendered buffer's bottom row does NOT contain the substring "[⌥"

  Scenario: AgentView layout splits area into Header / Scrollback / Input / Footer
    Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    And the AgentView has pushed two scrollback lines "user> hi" and "assistant> hello"
    When the App renders AgentView against an 80x10 TestBackend
    Then the rendered buffer's row 0 contains the substring "#1:"
    And the rendered buffer's rows 1 through 5 contain the substring "user> hi"
    And the rendered buffer's rows 1 through 5 contain the substring "assistant> hello"
    And the rendered buffer's row 9 contains the substring "Enter=send"

  Scenario: StreamChunk::TokenUpdate updates AgentViewStore.token_state_by_session for the current session
    Given an App with current_session "s-1"
    And token_state_by_session["s-1"] starts at TokenState::default()
    When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::TokenUpdate with tokens { input_tokens: 1234, output_tokens: 567 })
    Then AgentViewStore.token_state_by_session["s-1"] has input_tokens = 1234
    And AgentViewStore.token_state_by_session["s-1"] has output_tokens = 567

  Scenario: StreamChunk::ContextFillUpdate updates context_fill_pct
    Given an App with current_session "s-1"
    And token_state_by_session["s-1"] starts at TokenState { input_tokens: 100, output_tokens: 50, context_fill_pct: 0 }
    When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with context_fill { fill_percentage: 45, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 })
    Then AgentViewStore.token_state_by_session["s-1"] has context_fill_pct = 45
    And input_tokens and output_tokens are unchanged (100 and 50)

  Scenario: Non-token StreamChunk variants leave token_state unchanged
    Given an App with current_session "s-1"
    And token_state_by_session["s-1"] is TokenState { input_tokens: 1234, output_tokens: 567, context_fill_pct: 45 }
    When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::Text { text: "hi", correlation_id: None, observed_correlation_ids: None })
    Then AgentViewStore.token_state_by_session["s-1"] still has input_tokens = 1234
    And output_tokens still equals 567
    And context_fill_pct still equals 45

@done
@session
@agent-view
@RPC-074
@e2e
@rust
@tui
@slash-command
Feature: RPC-074 — /clear E2E (real fspec binary) emits no TS-divergent notice

  """
  E2E regression net for RPC-074 — the real `fspec` binary (built with
  --features test-stub-provider) is launched under @microsoft/tui-test
  and the rendered scrollback after a `/clear` keystroke is asserted to
  contain none of the TS-divergent `[notice] /clear: history cleared`,
  `[error] /clear failed: ...`, or `history cleared` strings. Matches
  the TS reference at src/tui/components/AgentView.tsx:1554-1564
  (handleClearCommand) which only blanks the input and calls
  sessionClearHistory.

  Pattern mirrors `e2e/rpc-072-work-agent-roundtrip.test.ts`.
  """

  Background: User Story
    As a Rust TUI user
    I want the real `/clear` slash command to behave byte-identically to the TS reference
    So that the Rust port does not invent UI text that the TS implementation never produces

  Scenario: Real fspec binary /clear keystroke leaves no divergent notice in rendered scrollback
    Given the real fspec binary built with --features test-stub-provider is launched under tui-test against the project workspace
    And the user has opened a Work Agent on a DONE work unit and sent at least one message so the scrollback contains output
    When the user types "/clear" and presses Enter
    Then within 5 seconds the rendered scrollback does NOT contain the substring "[notice] /clear"
    And within 5 seconds the rendered scrollback does NOT contain the substring "history cleared"
    And within 5 seconds the rendered scrollback does NOT contain the substring "[error] /clear failed"

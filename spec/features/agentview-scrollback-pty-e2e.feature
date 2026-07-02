@done
@rust
@scrollback
@agent-view
@bug
@tui
@e2e
@RPC-078
Feature: AgentView scrollback end-to-end through a real PTY
  """
  Highest-fidelity regression for RPC-078: spawn the real `fspec` binary in a real PTY, drive it with typed keystrokes, parse the master-side byte stream through a real vt100 terminal emulator, and assert what the user actually sees on screen. This is the only test layer that catches integration bugs (duplicate sync push + broadcast UserInput at the binary boundary) that in-process App::dispatch tests cannot reach.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # ARCHITECTURE NOTES:
  #   - Uses portable_pty::native_pty_system().openpty() with rows=40, cols=220
  #   - Spawns env!("CARGO_BIN_EXE_fspec") with --features test-stub-provider so the agent reply is the canned "hi back"
  #   - Drains master-side bytes through vt100::Parser into a render grid
  #   - Asserts substring counts on the captured screen — banned literals must be zero, "You: ..." and "● ..." each exactly one
  #
  # ========================================
  Background: User Story
    As a developer who needs end-to-end proof that RPC-078 is fixed in the shipped binary
    I want a PTY-driven test that types into the real fspec terminal and reads the rendered screen back
    So that integration bugs invisible to in-process tests (e.g. dual sync-push + broadcast) are caught by CI

  Scenario: End-to-end via tui-test: user typing produces correct prefixes, no duplicates, no truncation
    Given the real fspec binary running against ~/projects/fspec in a 220-column terminal with a stub LlmProvider that replies "hi back"
    When the user opens a Work Agent and types "is this card done?" and presses Enter
    Then the rendered terminal contains the substring "You: is this card done?" exactly once
    Then the rendered terminal contains the substring "● hi back" exactly once
    Then the rendered terminal contains none of the substrings "user>", "assistant>", "[done]", "[error]", "[interrupted]", "[notice]", "supervisor>", "(thinking)"

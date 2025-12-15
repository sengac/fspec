@high
@cli
@interactive
@tui
@CLI-002
Feature: Interactive TUI Agent Mode
  """
  Input queue: tokio::sync::mpsc::unbounded_channel for async-native queuing. ProviderManager extended with has_any_provider() for startup card. Based on OpenAI codex TUI architecture (see attachments).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System must enable raw mode after agent started
  #   2. System must start status updates after prompt received
  #   3. System must pause agent after agent interrupted
  #   4. System must dequeue all inputs after agent interrupted
  #   5. System must disable raw mode after agent terminated
  #   6. System must refresh system prompt after provider switched
  #   7. System must display startup card after agent started
  #   8. System must enable raw mode when agent starts to capture ESC key immediately
  #   9. System must display startup card showing available providers on agent start
  #   10. System must update status display every 1 second during agent execution
  #   11. System must stop agent streaming and display queued inputs when ESC is pressed
  #   12. System must restore terminal state (disable raw mode) when agent terminates
  #   13. System must preserve conversation history in terminal scrollback (inline mode, not alternate screen)
  #   14. System must queue user input received while agent is processing
  #   15. Use crossterm with panic hook. Set panic::set_hook to call crossterm::disable_raw_mode() before panicking. Copy codex pattern.
  #   16. Use ratatui + crossterm stack (same as codex). crossterm::event::EventStream with keyboard enhancement flags for ESC detection. No rustyline needed.
  #   17. Yes, tokio::select! handles both. Use async_stream::stream! to create unified event stream from crossterm::event::EventStream + agent stream. Standard pattern from codex.
  #   18. Use tokio::sync::mpsc::unbounded_channel. Async-native, integrates with tokio::select!, survives interruption cycle. No need for crossbeam.
  #   19. Use tokio::time::interval(Duration::from_secs(1)) as another arm in tokio::select!. Non-blocking, coordinates perfectly with streaming and input.
  #   20. Use Arc<AtomicBool> interrupt flag, break stream loop immediately. Rig MultiTurnStreamItem yields discrete events - breaking between events is safe. Partial results preserved.
  #   21. Extend ProviderManager with has_any_provider() and list_available_providers() methods. Check all provider credentials at startup.
  #
  # EXAMPLES:
  #   1. User runs 'codelet' without arguments, sees startup card with 'Available models: Claude (/claude), OpenAI (/openai)', enters prompt
  #   2. User sends prompt 'list all rust files', agent streams response with file operations, user sees output in real-time
  #   3. User presses ESC during agent execution, sees '⚠️ Agent interrupted. Queued inputs: (none)', can type next message
  #   4. User types additional input while agent is processing, sees '⏳ Input queued', presses ESC, sees queued input displayed
  #   5. Status display updates: '🔄 Processing request (1s • ESC to interrupt)' → '🔄 Processing request (2s • ESC to interrupt)'
  #   6. User types 'exit' or '/quit', agent terminates cleanly with terminal state restored
  #   7. User switches provider with '/openai', agent responds 'Switched to OpenAI provider', next prompts use OpenAI
  #   8. Terminal history preserved: User scrolls up to see previous agent responses and outputs
  #
  # QUESTIONS (ANSWERED):
  #   Q: Rust doesn't have direct equivalent to Node.js process.stdin.setRawMode(). Need to use termios/crossterm crates. How to properly restore terminal state on crash?
  #   A: Use crossterm with panic hook. Set panic::set_hook to call crossterm::disable_raw_mode() before panicking. Copy codex pattern.
  #
  #   Q: TypeScript uses readline + raw mode hybrid. Rust options: rustyline (readline-like) vs crossterm (low-level). Which provides better ESC key handling while maintaining normal input?
  #   A: Use ratatui + crossterm stack (same as codex). crossterm::event::EventStream with keyboard enhancement flags for ESC detection. No rustyline needed.
  #
  #   Q: Need to coordinate async agent streaming (futures::Stream) with terminal input events. Can tokio::select! handle both? Or need separate thread for input?
  #   A: Yes, tokio::select! handles both. Use async_stream::stream! to create unified event stream from crossterm::event::EventStream + agent stream. Standard pattern from codex.
  #
  #   Q: TypeScript uses EventEmitter pattern. Rust equivalent: tokio::sync::mpsc channels or crossbeam channels? Need to ensure queued inputs survive interruption cycle.?
  #   A: Use tokio::sync::mpsc::unbounded_channel. Async-native, integrates with tokio::select!, survives interruption cycle. No need for crossbeam.
  #
  #   Q: TypeScript uses setInterval(1000ms) for status updates. Rust: tokio::time::interval? How to coordinate with streaming without blocking?
  #   A: Use tokio::time::interval(Duration::from_secs(1)) as another arm in tokio::select!. Non-blocking, coordinates perfectly with streaming and input.
  #
  #   Q: When ESC pressed mid-tool-execution, TypeScript continues with partial results. Rig multi-turn handles tool calls automatically - can we interrupt cleanly without corrupting state?
  #   A: Use Arc<AtomicBool> interrupt flag, break stream loop immediately. Rig MultiTurnStreamItem yields discrete events - breaking between events is safe. Partial results preserved.
  #
  #   Q: Need to display provider availability (like TypeScript). ProviderManager::has_any_provider() doesn't exist yet in Rust. How to detect available providers at startup?
  #   A: Extend ProviderManager with has_any_provider() and list_available_providers() methods. Check all provider credentials at startup.
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to interact with the AI agent in a REPL-style interface
    So that I can iteratively work with the agent, interrupt when needed, and maintain conversation context

  Scenario: Start interactive mode and see startup card
    Given I have Claude and OpenAI API keys configured
    When I run 'codelet' without any arguments
    Then I should see a startup card with 'Codelet v'
    And I should see 'Available models: Claude (/claude), OpenAI (/openai)'
    And the terminal should be in raw mode for input capture

  Scenario: Stream agent response in real-time
    Given I am in interactive mode
    When I send the prompt 'list all rust files'
    Then I should see agent response streaming in real-time
    And I should see tool execution output as it happens
    And the conversation history should be preserved in terminal scrollback

  Scenario: Interrupt agent with ESC key
    Given the agent is currently processing a request
    When I press the ESC key
    Then the agent should stop streaming immediately
    And I should see '⚠️ Agent interrupted'
    And I should be able to type a new message
    And partial results should be preserved in conversation history

  Scenario: Queue input while agent is processing
    Given the agent is currently processing a request
    When I type additional input
    Then I should see '⏳ Input queued'
    And when I press ESC I should see the queued input displayed
    And the queued input should be available for the next prompt

  Scenario: Display status with elapsed time
    Given the agent is processing a request
    When 1 second passes
    Then the status display should show '🔄 Processing request (1s • ESC to interrupt)'
    And when another second passes it should show '🔄 Processing request (2s • ESC to interrupt)'

  Scenario: Exit agent cleanly
    Given I am in interactive mode
    When I type 'exit' or '/quit'
    Then the agent should terminate
    And the terminal state should be restored (raw mode disabled)
    And I should return to my normal shell

  Scenario: Switch provider during session
    Given I am in interactive mode using Claude
    When I type '/openai'
    Then I should see 'Switched to OpenAI provider'
    And subsequent prompts should use the OpenAI provider

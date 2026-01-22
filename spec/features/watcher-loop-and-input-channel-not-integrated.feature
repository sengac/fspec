@done
@watcher-management
@session
@WATCH-019
Feature: Watcher Loop and Input Channel Not Integrated

  """
  session_create_watcher must: 1) Create BackgroundSession 2) Subscribe to parent's watcher_broadcast via subscribe_to_stream() 3) Spawn watcher_agent_loop (not agent_loop) passing the broadcast receiver
  Parent agent_loop uses tokio::select! to read from both input_rx AND watcher_input_rx, with user input taking priority
  Watcher agent_loop uses run_watcher_loop from WATCH-005 with a callback that runs evaluation prompts through the agent and parses for [INTERJECT]/[CONTINUE] blocks
  WatcherInput messages are formatted with structured prefix and processed as User messages by the parent AI
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Watcher sessions must spawn run_watcher_loop instead of regular agent_loop to enable dual-input handling
  #   2. Parent session agent_loop must read from watcher_input_rx to process watcher injections
  #   3. run_watcher_loop uses tokio::select! biased towards user input over parent broadcast observations
  #
  # EXAMPLES:
  #   1. session_create_watcher creates watcher session → spawns run_watcher_loop with parent broadcast subscription → watcher receives parent chunks via broadcast_rx
  #   2. Watcher observes parent writing code → parent sends TurnComplete chunk → watcher evaluates accumulated observations → watcher AI decides to interject warning
  #   3. Watcher injects message → watcher_inject calls receive_watcher_input → parent agent_loop reads from watcher_input_rx → parent AI receives and responds to watcher message
  #
  # ========================================

  Background: User Story
    As a watcher session
    I want to observe parent session and inject messages
    So that the parent AI can receive real-time feedback from watcher AI agents

  Scenario: Parent session processes watcher injections
    Given a parent session exists with a watcher attached
    When the watcher injects a message via watcher_inject
    Then the parent agent_loop should read the message from watcher_input_rx and process it


  Scenario: Watcher session subscribes to parent broadcast on creation
    Given a parent session exists with an active broadcast channel
    When session_create_watcher is called with the parent session ID
    Then the watcher should have a broadcast receiver subscribed to the parent's stream


  Scenario: Watcher loop processes parent observations at breakpoints
    Given a watcher session is running with parent broadcast subscription
    When the parent session emits Text chunks followed by a Done chunk
    Then the watcher should accumulate observations and trigger evaluation at the Done breakpoint


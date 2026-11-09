@infrastructure
@rpc
@done
@BUG-572
@LOG-004
Feature: Tracing event recursion via BroadcastLogLayer ↔ bootstrap logs_task feedback loop
  """
  BroadcastLogLayer is the process-global tracing layer that fans tracing events out to every registered SharedFspecService.logs_tx broadcast. The TUI bootstrap subscriber pulls from logs_rx and re-emits each record as a debug! event, which BroadcastLogLayer would otherwise re-capture and re-broadcast — a recursive cycle. Filter: events whose metadata target equals 'codelet_fspec_tui::app::bootstrap' are dropped at on_event time. No other layers (file, stderr) are affected — they continue to receive the original AND the bootstrap re-emission. This is the minimum filter that breaks the cycle without losing diagnostic data.
  BUG-572: the broadcast is carried by tungstenite, tokio_tungstenite and tarpc, which log every frame they move (tungstenite through the log crate, bridged into tracing by the registry's try_init). Forwarding one LogRecord over a WebSocket logged more frames, which were broadcast and forwarded again: a flood that lagged subscribers. TRACE, DEBUG and INFO events whose target is one of those crates (the crate name, or the crate name followed by '::') are not broadcast; their WARN and ERROR events still are. A record bridged from the log crate is judged and labelled by its own target (tracing-log's normalized metadata), not 'log'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BroadcastLogLayer must drop events whose target is the TUI bootstrap subscriber that re-emits broadcast records, preventing infinite recursion
  #
  # EXAMPLES:
  #   1. When BroadcastLogLayer sees an event with target 'codelet_fspec_tui::app::bootstrap', it skips broadcasting and the cycle is broken
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have the tracing log broadcast not feed back into itself
    So that log files do not grow geometrically and diagnostics remain readable

  Scenario: Events targeted at codelet_fspec_tui::app::bootstrap are not broadcast
    Given a BroadcastLogLayer is installed with a registered broadcast sender
    When a tracing event with target "codelet_fspec_tui::app::bootstrap" is emitted
    Then the registered broadcast sender receives zero LogRecord values for that event

  Scenario: Events from non-TUI-bootstrap targets are still broadcast
    Given a BroadcastLogLayer is installed with a registered broadcast sender
    When a tracing event with target "codelet_agent_loop::hooks" is emitted
    Then the registered broadcast sender receives exactly one LogRecord value for that event

  Scenario: Below-warning events from the WebSocket transport crates are not broadcast
    Given a BroadcastLogLayer is installed with a registered broadcast sender
    When tungstenite, tokio_tungstenite and tarpc emit TRACE, DEBUG and INFO events through tracing and through the log crate
    Then the registered broadcast sender receives zero LogRecord values

  Scenario: Transport warnings and errors and look-alike targets are still broadcast
    Given a BroadcastLogLayer is installed with a registered broadcast sender
    When tungstenite emits a WARN event through the log crate, tarpc emits an ERROR event and a target named "tungstenite_ext" emits a TRACE event
    Then the registered broadcast sender receives exactly those three LogRecord values in order, each with its own target and level

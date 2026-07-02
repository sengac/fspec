@infrastructure
@rpc
@done
@LOG-004
Feature: Tracing event recursion via BroadcastLogLayer ↔ bootstrap logs_task feedback loop
  """
  BroadcastLogLayer is the process-global tracing layer that fans tracing events out to every registered SharedFspecService.logs_tx broadcast. The TUI bootstrap subscriber pulls from logs_rx and re-emits each record as a debug! event, which BroadcastLogLayer would otherwise re-capture and re-broadcast — a recursive cycle. Filter: events whose metadata target equals 'codelet_fspec_tui::app::bootstrap' are dropped at on_event time. No other layers (file, stderr) are affected — they continue to receive the original AND the bootstrap re-emission. This is the minimum filter that breaks the cycle without losing diagnostic data.
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

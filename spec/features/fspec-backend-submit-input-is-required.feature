@done
@agent-interaction
@BUG-331
@tui
@rpc
@regression
Feature: Every TUI backend states how it delivers each input mode
  """
  FspecBackend::submit_input (rust/fspec-tui/src/transport/backend/subscriptions.rs)
  is a required trait method with no default body: a default that dropped the
  InputMode and called send_input silently turned a steer or interrupt into a plain
  turn for any backend that did not override it. The embedded and websocket transports
  implement it with the submit_input RPC; every test backend implements it explicitly —
  forwarding backends (ModelBackend, ContinueBackend) pass it to the recording
  MockBackend, and HistoryMockBackend and DefaultsOnlyBackend record the
  (session, text, mode) they receive.
  """

  Background: User Story
    As a TUI user steering or interrupting a running session
    I want every backend to deliver my input with the mode I chose
    So that a steer or interrupt is never silently sent as a plain turn

  Scenario: The FspecBackend trait has no default submit_input
    Given the FspecBackend trait declaration
    When its submit_input method is read
    Then it is a required method without a default body

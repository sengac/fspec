@done
@agent-core
@agent-manager
@rust
@BUG-217
Feature: Independent subordinate relay progress and cancellation
  """
  Both front doors use the shared forwarding owner. Each connected relay has
  an independent disk-journal cursor and bounded channel, with at most one
  delivery task and one in-flight chunk per relay, never a task per chunk.
  Cancelling the forwarding owner cancels its delivery workers and releases
  their subscriptions. Disconnecting a relay releases only its own backlog.
  A destroyed session drains pending events to still-connected relays before
  delivery closes. New relays start with newly forwarded output, not history.
  """

  Background: User Story
    As an operator with multiple relay connections
    I want each connection to progress independently
    So that a stalled observer does not hide subordinate completion

  Scenario: A stalled relay does not block a healthy relay or lose its own backlog
    Given a subordinate has a stalled relay and a healthy relay on its root parent
    When more events than a relay channel can hold are emitted including completion
    Then the healthy relay receives every event exactly once in order
    And disconnecting the healthy relay releases its subscription without new output
    And the stalled relay later receives the same ordered events without gaps

  Scenario: Cancelling the forwarding owner releases blocked relay subscriptions
    Given an independently owned forwarder has a relay with a full bounded queue
    When the forwarding owner is cancelled
    Then all of its relay subscriptions are released while the relay stays connected

  Scenario: Late relays receive new output without replaying already forwarded history
    Given one relay has received subordinate output
    When a second relay connects before new output is emitted
    Then both relays receive the new output without replaying earlier output
    And destroying the subordinate drains completion to both relays and closes forwarding

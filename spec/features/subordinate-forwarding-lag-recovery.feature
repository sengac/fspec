@done
@agent-core
@agent-manager
@rust
@BUG-217
Feature: Subordinate forwarding survives bounded receiver lag
  """
  Subordinate output is accepted into a cursor-readable temporary disk journal before
  bounded broadcast notification. Forwarders use the broadcast only as a wakeup and
  replay one journal record at a time, so dropped notifications cannot drop output.
  Both front doors share the delivery implementation and retain root-parent routing.
  Relay channels apply bounded backpressure rather than accumulating unbounded queues.
  Journal cursors preserve all chunk types, including completion and history-clear
  events; new cursor subscriptions start at current output without replaying history.
  The journal is transient, not crash-recovery storage, and is released with its last
  subscription or session. Journal I/O errors are explicit failures, never silent gaps.
  Each relay progresses independently with a bounded queue and its own journal cursor.
  Common consumed journal prefixes are reclaimed before publication even during
  continuous lag; each cursor retains its unread records independently.
  """

  Background: User Story
    As an operator watching subordinate agents through a relay
    I want complete ordered output when forwarding falls behind
    So that text and terminal events do not silently disappear

  Scenario: A stalled subordinate forwarder replays evicted terminal events in order
    Given a subordinate forwarding to its root parent's relay
    When a stalled bounded broadcast receiver is overrun with text and terminal events
    Then the relay receives every subordinate event exactly once in original order
    And a subsequent burst is delivered without replaying earlier events

  Scenario: Relay output applies bounded backpressure and survives disconnection
    Given a subordinate forwarding to a relay that is not draining output
    When more events are emitted than the relay channel can hold
    Then the relay queue remains bounded
    When the relay disconnects and a replacement relay subscribes
    Then new subordinate output reaches the replacement relay

  Scenario: Destroying a subordinate drains terminal output and closes its subscription
    Given a subordinate with a stalled forwarding subscription
    When terminal output is emitted and the subordinate is destroyed
    Then the relay receives the pending terminal output
    And the forwarding subscription closes after draining

  Scenario: Cancelling a subscription releases its bounded receiver
    Given a subordinate with an active forwarding subscription
    When the forwarding subscription is cancelled
    Then its broadcast receiver is released without keeping the session alive

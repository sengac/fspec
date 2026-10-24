@done
@BUG-216
@rust
@agent-loop
@session
@high
@streaming-loop-detection
Feature: Loop escalation requires independent post-warning evidence
  """
  Architecture: BackgroundOutput rearms the warned channel's detector after
  consuming its signal. The shared policy can only escalate from fresh complete
  word evidence or a trigger from the other independently maintained channel.
  The existing interruption protocol and corrective-note machinery are unchanged.
  """

  Scenario: A warning cannot escalate from latched evidence alone
    Given a background output whose text channel has just warned on a genuine loop
    When empty deltas and fresh technical prose arrive
    Then the session is not interrupted and no corrective note is staged
    When a fresh complete evidence window repeats in the text channel
    Then the session is interrupted and a corrective note is staged

  Scenario: Channel evidence is independent and new turns clear partial words
    Given a background output whose text channel has just warned on a genuine loop
    When the thinking channel receives a genuine complete word loop
    Then the session is interrupted and a corrective note is staged
    When a new turn resets both channels and receives fresh prose
    Then no new interruption or corrective note occurs

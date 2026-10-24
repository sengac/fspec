@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Bound recovery admission without losing accepted originals
  """
  Uses the single shared recovery owner. Capacity is 128 envelopes and 16 MiB
  encoded bytes per session. Original ownership survives a journal failure in
  memory, but cannot promise crash durability until actual storage recovers.
  Three bounded in-process ownership slots retain a blocked journal event and
  its assistant/result originals beyond admission capacity, up to 16 MiB total.
  They require an explicit shutdown-unsafe error and block further dispatch.
  """

  Scenario: An unavailable recovery journal retains original ownership
    Given a real session whose archive and recovery journal are unavailable
    When assistant text settles and its output owner is dropped
    Then a scoped error warns that shutdown is unsafe without exposing original content
    And repairing storage and explicit recovery archives the original text once

  Scenario: Oversized input is refused before recovery admission
    Given a real session with an empty recovery journal
    When one envelope exceeds the recovery byte capacity
    Then admission fails without accepting or truncating the envelope

@done
@CMPCT-082
@testing
@providers
@context-management
@compaction
Feature: Invalid provider admission state blocks physical inference
  """
  Explicit usable capacity and output reservation are mandatory at the physical
  boundary. Inaccessible admission state cancels inference before sending and
  cannot claim successful completion, compaction or alternate recovery.
  """

  Scenario Outline: Invalid explicit admission limits block physical inference
    Given a Codex session with <limits> and an unchanged committed checkpoint
    When I request a provider completion
    Then a typed admission failure is returned before a physical send
    And no completion is emitted and the committed checkpoint remains unchanged

    Examples:
      | limits                        |
      | missing context capacity      |
      | missing output reservation    |
      | zero context capacity         |
      | zero output reservation       |
      | reservation equal to capacity |
      | reservation above capacity    |

  Scenario: Inaccessible admission state blocks the streaming agent
    Given an agent whose admission state lock is inaccessible
    When its streaming completion hook checks a real provider request
    Then inference is cancelled before any physical request
    And no successful completion or substitute compaction is emitted

@done
@session
@rpc
@tui
@persistence
@agent-view
@BUG-245
Feature: Archival transcript history access
  """
  The rendered scrollback is a bounded projection. Its trim marker identifies
  the archive inspector command. The inspector uses the same authoritative
  transcript cursor as hydration, without changing model history or tokens.
  """

  Scenario: Original events remain inspectable outside the resident scrollback
    Given a real session containing original text and complete tool content
    When the TUI inspects an original event using its archive command through either transport
    Then the complete original payload is accessible in the turn inspector
    And no input is sent to the model

  Scenario: Archive inspection rejects replacement source generations
    Given a hydrated tab whose backend transcript has been replaced without a reset notification
    When the user inspects an event from the displayed archive
    Then a visible archive generation failure preserves the displayed history without presenting replacement content

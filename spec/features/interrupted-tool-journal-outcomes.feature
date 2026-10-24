@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Keep interrupted tool outcomes truthful
  """
  Synthetic dialogue pairing is not authentic tool result capture. The private
  journal preserves unknown side effects separately without changing originals.
  """

  Scenario: Interrupted running tools retain unknown outcomes
    Given a running tool that has written an external sentinel
    When a genuine user interruption stops streaming before authentic result capture
    Then the durable tool outcome remains unknown rather than completed or cancelled

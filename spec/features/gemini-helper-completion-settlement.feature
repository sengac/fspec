@done
@BUG-223
@cli
@providers
@agent-interaction
@streaming
@bug-fix
@phase-1
Feature: Gemini helper completion settlement
  """
  Architecture: Gemini empty-after-tool continuation returns structured completion
  evidence to the primary stream loop. The existing common settlement owner alone
  decides nudges, refunds, exhausted budgets, accepted summaries and goal teardown.
  Local scripted Gemini SSE tests drive real tools and the public stream runner.
  Preserve thought signatures, token accounting and compaction recovery policies.
  """

  Scenario: Clean helper completion consumes a counted nudge
    Given an armed Gemini session enters empty-after-tool continuation
    When the helper completes with text and a clean stop without accepted done
    Then the common owner sends one counted nudge and eventually emits exactly one terminal event
    And helper text history and measured usage are preserved

  Scenario: Helper tool activity refunds a pending nudge
    Given an armed Gemini session has consumed a nudge before entering the helper
    When the helper executes a tool and completes with a clean text response
    Then the common owner refunds the pending nudge before deciding the next nudge
    And eventual budget exhaustion emits exactly one terminal event

  Scenario: Exhausted helper completion emits the normal budget warning
    Given an armed Gemini session enters the helper with no remaining nudge budget
    When the helper completes with a clean text response
    Then the common owner warns about exhausted budget without another inference
    And exactly one terminal event is emitted

  Scenario: Exhausted goal helper completion escalates for human review
    Given a Gemini helper session has an active goal and an exhausted goal budget
    When the helper completes without accepted done
    Then the common owner raises goal escalation and retains the active goal without another inference
    And exactly one terminal event is emitted

  Scenario: Nonclean helper stops never consume a nudge
    Given armed Gemini helper sessions return token safety or unknown stop reasons
    When each helper completes with useful text
    Then no nudge is consumed and exactly one terminal event retains the actual helper stop reason

  Scenario: Helper errors do not settle a clean completion
    Given an armed Gemini session enters the helper
    When the helper receives a terminal provider error
    Then the error is surfaced without a nudge or a successful terminal event

  Scenario: Helper interruption does not settle a clean completion
    Given an armed Gemini session enters the helper
    When streamed helper text triggers user interruption
    Then interruption preserves partial history without a nudge and emits exactly one terminal event

  Scenario: Helper compaction hands off without a completion nudge
    Given an armed Gemini helper executes a tool whose result exceeds the context threshold
    When the next helper request is cancelled for compaction
    Then the compaction owner receives the handoff without a nudge or premature terminal success

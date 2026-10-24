@done
@BUG-222
@cli
@providers
@provider-abstraction
@thinking-exhaustion
Feature: Truthful bounded prompt-only thinking exhaustion recovery
  """
  Immediate recovery reuses the active agent with a concise-reasoning prompt.
  It does not rebuild provider reasoning controls or enlarge output limits.
  Capture outgoing requests to verify unchanged effective settings and caps.
  Cross-turn degradation only updates the preference used by future agent builds.
  Keep the two-retry bound, history, reasoning context and cancellation priority.
  BUG-220 owns teardown and BUG-223 owns Gemini continuation settlement.
  """

  Scenario: Prompt-only retries preserve effective reasoning settings and output caps
    Given exhaustion responses under high low minimal or absent reasoning controls and a fixed output cap
    When an exhaustion response is followed by a useful answer
    Then the first and retry requests have identical reasoning controls and output limits
    And the retry prompt and status explicitly describe prompt-only recovery with unchanged settings

  Scenario: Exhausted retries stop without falsely claiming thinking was disabled
    Given every response exhausts thinking and the future-turn preference is already Off
    When the bounded recovery budget is exhausted
    Then exactly two retry requests preserve the original settings and output cap
    And the final warning says no further recovery requests without claiming thinking was disabled or reduced

  Scenario: Cross-turn downgrade affects the future preference rather than the active retry
    Given a High preference one exhaustion away from the cross-turn downgrade threshold
    When another exhaustion response triggers a retry
    Then the future preference becomes Medium while both captured requests retain high reasoning effort
    And the notification distinguishes subsequent turns from unchanged current retry settings

  Scenario: Prompt-only recovery retains conversation history reasoning and completed tool pairs
    Given a conversation with completed tool progress and an earlier answer
    When the next user turn exhausts thinking and recovers with a prompt-only retry
    Then the retry retains the original history and matched tool pairs without duplicate recovery prompts
    And captured reasoning is present in the retry prompt while request caps remain unchanged

  Scenario: Cancellation during recovery prevents an unused prompt and extra request
    Given user cancellation on a thinking event or recovery notification
    When the stream loop handles thinking exhaustion
    Then only the initial request is sent and no unused recovery prompt is appended

  Scenario: A terminal retry error stops prompt-only recovery immediately
    Given thinking exhaustion followed by a terminal provider rejection on retry
    When the retry fails
    Then exactly two requests occur and the terminal error is surfaced without another recovery attempt

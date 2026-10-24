@done
@BUG-223
@cli
@providers
@agent-interaction
@streaming
@bug-fix
@phase-1
Feature: Gemini helper accepted completion
  """
  Architecture: Accepted Done in Gemini empty-after-tool continuation uses the
  shared immediate completion owner before another stream poll. It announces the
  summary, clears goal state and resets counters exactly as ordinary completion.
  Local scripted Gemini SSE tests drive real tools and the public stream runner.
  """

  Scenario: Accepted helper done settles summary and goal teardown once
    Given Gemini helper sessions have an armed contract with and without an active goal
    When the helper executes an accepted done tool call
    Then the common owner announces the accepted summary and resets the nudge counter once
    And an active goal and its reminder and registry state are cleared without another inference
    And exactly one terminal event records done acceptance

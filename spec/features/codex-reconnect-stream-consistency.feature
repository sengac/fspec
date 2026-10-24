@phase-1
@provider-abstraction
@providers
@PROV-160
Feature: Codex reconnect stream consistency
  """
  Each physical response attempt has a streaming snapshot boundary shared by SSE and WebSocket. Transport failure retracts only abandoned text and reasoning before recovery, including exhausted recovery. Successful continuation output remains visible. Terminal output fallback belongs to the same attempt as live deltas. Tool dispatch and checkpoint publication remain terminal-only.
  """

  Background: User Story
    As a Codex session owner
    I want reconnection to replace abandoned streaming output
    So that the next turn replays exactly the successful answer

  Scenario: Replacement output agrees with next-turn lineage on both transports
    Given an attempt streams divergent text and reasoning before disconnecting
    When its replacement completes on the selected transport
    Then abandoned output is retracted and the successful answer appears once
    And the next turn replays the successful answer without lineage failure

  Scenario: Streaming remains live before the attempt terminates
    Given an accepted WebSocket response whose terminal event is withheld
    When partial text and reasoning arrive
    Then both are delivered before the server releases the terminal event

  Scenario: Continuation retries preserve earlier successful physical output
    Given a successful nonterminal response followed by an interrupted continuation
    When the continuation replacement completes
    Then only the interrupted continuation output is retracted

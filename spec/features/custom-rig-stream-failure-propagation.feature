@done
@provider-abstraction
@BUG-227
@providers
@streaming
@bug-fix
@phase-1
Feature: Custom rig stream failure propagation
  """
  Architecture: the real RhaiCustomProviderModel streams local HTTP responses
  through open_stream and the rig streaming adapter. A bridge error must reach
  rig consumers without a successful FinalResponse or default EndTurn. Clean
  DONE and explicit stops retain usage and tool behavior. Dropping a pending
  response cancels the underlying body rather than manufacturing completion.
  These scenarios were split unchanged from the bridge feature to preserve
  one feature per test module and keep each file below 300 lines.
  """

  Scenario: Rig rejects interrupted and malformed HTTP streams
    Given local HTTP streams with body failures decoding failures or missing terminal evidence
    When the real custom rig model consumes each stream
    Then each stream reports one error and no successful FinalResponse or EndTurn
    And buffered tools are not completed by failure finalization

  Scenario: Rig preserves successful clean terminal responses
    Given local HTTP streams with text reasoning usage tools and clean terminal evidence
    When the real custom rig model consumes each stream
    Then each stream preserves its deltas tools usage and final stop reason

  Scenario: Rig preserves HTTP and Rhai error behavior
    Given local HTTP streams with an unauthorized response or a Rhai runtime failure
    When the real custom rig model consumes each stream
    Then each stream reports its original error without a successful final response

  Scenario: Cancel a pending rig stream
    Given a local HTTP stream pending after its first text delta
    When the consumer drops the rig stream after that delta
    Then the HTTP connection closes without a successful final response

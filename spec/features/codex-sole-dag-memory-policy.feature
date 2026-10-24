@providers
@CMPCT-077
@provider-abstraction
@codex
@compaction
@phase-1
Feature: Codex sole DAG memory policy
  Codex uses fspec's committed DAG rather than an alternate native memory algorithm.

  """
  CMPCT-077 supersedes the opt-in native compaction behavior of PROV-158.
  Remove the compaction capability and trigger generation, not merely its default.
  Reject native memory activation and compaction items before checkpoint mutation,
  tool dispatch or completion, on streaming and direct completion routes alike.
  Existing incompatible native checkpoints must not become replayable by changing
  the model or replacing the incoming prefix. Encrypted reasoning and explicitly
  negotiated hosted search remain separate supported provider contracts.
  All tool dispatch, including read-only calls, waits for successful terminal
  validation so later unsupported native items cannot follow local execution.
  Validate native items in every response snapshot before interpreting lifecycle
  fields. Preparing a replacement request must not discard committed state before
  the response passes policy validation.
  This supersedes PROV-151's early read-only dispatch contract.
  This slice does not claim to fix failed DAG recovery owned by CMPCT-079.
  """

  Scenario: Native memory activation fails before sending a request
    Given a Codex model and explicit native memory configuration
    When streaming and direct completion are requested
    Then typed unsupported errors leave the checkpoint unchanged and send no request

  Scenario: Unsolicited native memory output never commits or completes
    Given a committed Codex checkpoint and responses containing native compaction items
    When streaming events and direct responses are consumed
    Then typed unsupported errors preserve the checkpoint without tools or completion

  Scenario: Native checkpoint and trigger input cannot bypass the DAG policy
    Given a committed checkpoint and attempted native trigger or native checkpoint replay
    When history preparation and commit are attempted
    Then typed unsupported errors retain the original checkpoint and input

  Scenario: Encrypted reasoning and hosted search remain independent of native memory
    Given a Codex model with hosted search enabled and encrypted reasoning output
    When a successful response is followed by another user request
    Then supported items replay without native memory activation or opaque disclosure

  Scenario: Nonterminal native memory snapshots cannot authorize local tools
    Given a pending local tool followed by a nonterminal response containing native memory
    When the stream later reports successful completion without native items
    Then a typed unsupported error preserves the checkpoint without dispatch or completion

  Scenario: Malformed native memory terminals still return typed policy errors
    Given a response containing native memory with invalid terminal lifecycle fields
    When streaming and direct completion responses are consumed
    Then native memory is rejected with a typed unsupported error before lifecycle parsing

  Scenario: Rejected replacement requests retain the committed checkpoint
    Given a committed checkpoint and a request replacing its model or conversation prefix
    When the replacement response contains unsupported native memory
    Then the original in-memory and durable checkpoint survive on every completion route
    And a subsequent supported response commits the replacement conversation

  Scenario: Malformed direct completions preserve the committed checkpoint
    Given a committed checkpoint and a replacement response with malformed completion data
    When a direct completion cannot be decoded or converted to an assistant response
    Then the in-memory and durable checkpoint remain unchanged without replay
    And a subsequent valid completion commits the replacement conversation

  Scenario: Malformed streamed completions preserve the committed checkpoint
    Given a committed checkpoint and a streamed replacement response with malformed completion data
    When a streamed completion cannot be decoded or converted to an assistant response
    Then the in-memory and durable checkpoint remain unchanged without replay
    And a subsequent valid streamed completion commits the replacement conversation

@done
@provider-abstraction
@BUG-227
@providers
@streaming
@bug-fix
@phase-1
Feature: Custom stream failure propagation
  """
  Architecture: open_stream owns fallible SSE decoding and terminal evidence.
  All eventsource errors are fatal, including transport and UTF-8/parser errors.
  Ordinary EOF requires a Rhai stop; DONE is independently sufficient.
  Errors terminate without flushing buffered tool arguments. Already emitted
  deltas remain partial evidence, never a successful FinalResponse or EndTurn.
  The real rig model and local HTTP responses verify downstream propagation.
  Dropping a stream cancels consumption without manufacturing a final response.
  Builtin provider adapters and continuation policy are outside this change.
  """

  Scenario: Fail transport before any output
    Given a custom stream with a body error before any events
    When the bridge consumes the stream
    Then it yields one typed provider error and does not read subsequent events

  Scenario: Preserve partial deltas without flushing tools after transport failure
    Given a custom stream with text reasoning usage and buffered tool arguments
    When a body error interrupts the stream
    Then the emitted deltas precede one error without tool completion or stop

  Scenario: Treat SSE decoding errors as fatal
    Given invalid UTF-8 followed by valid SSE events
    When the bridge decodes the stream
    Then it yields one decoding error without processing later events

  Scenario: Do not conceal transport failure after explicit stop
    Given a custom stream with an explicit stop followed by a body error
    When the bridge consumes the remaining body
    Then the stop is followed by one terminal error

  Scenario: Reject EOF without terminal evidence
    Given empty text-only pending-tool and truncated custom streams without a stop or DONE
    When each stream reaches ordinary EOF
    Then each stream yields one incomplete-stream error without flushing tools or stop

  Scenario: Preserve clean DONE completion
    Given text reasoning usage and buffered tool arguments followed by DONE
    When the bridge consumes the clean stream
    Then the deltas and completed tool are preserved without passing DONE to Rhai

  Scenario: Preserve explicit stop with clean EOF
    Given custom streams ending with each supported explicit stop reason
    When the bridge reaches clean EOF
    Then each stream preserves its stop reason without error

  Scenario: Cancel a pending custom stream
    Given a custom byte source pending after its first text delta
    When the consumer drops the bridge after that delta
    Then the source is dropped without further polling or finalization

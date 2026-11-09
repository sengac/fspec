@BUG-582
@streaming-loop-detection
@BUG-302
@cli
@streaming
Feature: Stream-loop source guards read the split module tree
  """
  The interactive stream loop was split from rust/cli/src/interactive/stream_loop.rs into
  stream_loop.rs plus stream_loop/*.rs (2026-10-25). Source-shape guard tests read the whole
  module tree through rust/cli/tests/support/stream_loop_source.rs, in a stable order
  (stream_loop.rs first, then module files sorted by name).
  """

  Background: User Story
    As a developer relying on stream-loop source guards
    I want the guards to inspect the code wherever the split stream loop keeps it
    So that the auto-continue, done and compaction wiring guards keep protecting real behaviour

  Scenario: Source guards see code moved into stream-loop modules
    Given the stream loop is split across stream_loop.rs and stream_loop/*.rs
    When a source guard reads the stream-loop source
    Then it sees the FinalResponse settle code from stream_loop/final_completion.rs
    And stream_loop.rs comes first followed by the module files in name order

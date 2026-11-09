@done
@BUG-467
@codelet
@infrastructure
@rust
Feature: Debug capture reports events and summaries it cannot write
  """
  rust/common/src/debug_capture/capture.rs capture() returned without a
  trace when the session file was unset, could not be opened, an event
  could not be serialised or a line could not be written; summary.rs wrote
  "Unable to read events" and ignored the summary write result; the
  capture helpers (capture_event, increment_debug_turn and the inline
  "if let Ok(manager) ... if let Ok(lock)" blocks in rust/cli) dropped
  manager and lock errors; stop_capture returned an empty path when
  capture was off.
  Now capture() counts each failed write with its error (it cannot log:
  the DebugCaptureLayer calls it while holding the manager lock, and
  logging there would recurse into the layer); stop_capture fails with
  EventsLost naming the count, last error and session file, fails when the
  summary cannot be read or written, and fails with NotCapturing when
  capture is off. debug_capture::with_enabled_capture replaces the inline
  blocks and logs manager / lock failures before any lock is held.
  Tests: rust/common/tests/bug467_debug_capture_reports_lost_events.rs.
  """

  Background: User Story
    As a developer diagnosing an LLM session with debug capture
    I want to have events and summaries that could not be written reported
    So that a debug session never looks complete while it is missing events

  Scenario: Stopping a capture that lost events reports them
    Given a started debug capture whose session file is removed and replaced by a directory
    When an event is captured and capture stops
    Then stopping fails reporting 1 lost event and the write error

  Scenario: Stopping a capture that is off is an error
    Given a debug capture manager that has not started capture
    When capture stops
    Then stopping fails saying capture is not running

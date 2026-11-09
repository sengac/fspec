@done
@COV-064
@codelet
@infrastructure
@rust
Feature: Debug capture and logging
  """
  COV-061 retired the never-built debug capture and logging features; the
  APIs remained untested. This feature specifies their current behaviour
  (rust/common/src/debug_capture, rust/common/src/logging):
  DebugCaptureManager::start_capture creates the debug directory with 0700
  permissions and an empty session-<YYYY-MM-DDTHH-MM-SS>.jsonl file;
  capture writes one JSON line per event (timestamp, sequence, eventType,
  turnId, optional requestId, data) with sensitive headers replaced by
  [REDACTED], and nothing while capture is disabled; stop_capture writes
  exactly one session.start (provider/model from set_session_metadata) and
  one session.end, then <session>.summary.md. Logging writes through
  logging::private_files::appender: a 0700 directory, daily rotation, the
  "rust" filename prefix and at most five files.
  Capture's silent write failures are tracked separately (BUG-467).
  Tests: rust/common/tests/cov064_debug_capture_and_logging.rs.
  """

  Background: User Story
    As a developer diagnosing an LLM session
    I want to have debug capture and the log files behave as specified and tested
    So that a debug session or log file can be trusted to hold what it claims

  Scenario: Starting a capture creates a private debug directory and a session file
    Given a debug capture manager whose debug directory does not exist yet
    When capture starts
    Then the debug directory exists with 0700 permissions
    And an empty session file named session-<timestamp>.jsonl exists in it

  Scenario: Captured events are numbered JSON lines with redacted sensitive headers
    Given a started debug capture in turn 2
    When an "api.request" event with an Authorization header and request id "req-1" is captured
    Then the session file holds one JSON line with sequence 0, turnId 2 and requestId "req-1"
    And the Authorization header reads "[REDACTED]"

  Scenario: Nothing is captured while capture is disabled
    Given a debug capture manager that has not started capture
    When an event is captured
    Then no session file is written

  Scenario: Stopping a capture records the session start and end and writes a summary
    Given a started debug capture with provider "anthropic" and model "claude-sonnet-4"
    When capture stops
    Then the session file holds exactly one session.start naming the provider and model and one session.end after it
    And a .summary.md file is written next to the session file

  Scenario: Rust logs go to private daily-rotated files
    Given an empty logs directory
    When the private log appender is built and a line is written
    Then the directory has 0700 permissions
    And the line is in a file whose name starts with "rust"

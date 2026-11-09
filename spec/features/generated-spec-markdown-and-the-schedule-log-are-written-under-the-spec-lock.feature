@done
@BUG-498
@cli
@infrastructure
@rust
Feature: Generated spec markdown and the schedule log are written under the spec lock
  """
  BUG-477 made every spec/*.json write hold the spec lock and replace the file
  atomically, but spec/TAGS.md (generate-tags-md, register-tag, update-tag,
  delete-tag), spec/FOUNDATION.md (generate-foundation-md and the foundation
  commands that regenerate it) were still written in place with
  std::fs::write, and spec/schedule-log.jsonl was appended (and rotated by a
  read-then-truncating write) without the lock in
  rust/core/src/scheduler/job_log.rs, so concurrent commands could interleave
  or truncate them. Now the markdown files are written through one helper
  that holds the spec lock and replaces the file atomically, and each log
  entry is appended (and the log rotated, atomically) while holding the spec
  lock.
  Tests: rust/fspec-core/tests/bug498_markdown_written_under_the_spec_lock.rs,
  rust/core/tests/bug498_schedule_log_appends_under_the_spec_lock.rs.
  """

  Background: User Story
    As a maintainer running fspec commands in parallel
    I want to have TAGS.md, FOUNDATION.md and spec/schedule-log.jsonl written under the spec lock
    So that concurrent commands never interleave or truncate them

  Scenario: Generating TAGS.md waits for the spec lock
    Given another process holds the spec lock of a project with registered tags
    When generate-tags-md runs
    Then TAGS.md is not written until the lock is released
    And afterwards TAGS.md holds the complete generated document

  Scenario: Generating FOUNDATION.md waits for the spec lock
    Given another process holds the spec lock of a project with a foundation
    When generate-foundation-md runs
    Then FOUNDATION.md is not written until the lock is released
    And afterwards FOUNDATION.md holds the complete generated document

  Scenario: Concurrent schedule log appends never interleave
    Given a project spec directory
    When eight threads each append 300 schedule log entries at the same time
    Then the schedule log holds every entry since its one rotation, each a whole JSON line

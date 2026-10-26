@done
@agent-core
@session-management
@persistence
@testing
@BUG-270
Feature: Preserve streaming admission ownership across storage failures
  """
  Admission cannot publish before journal durability. Failed admission keeps the
  exact envelope in bounded recovery ownership, including when recovery staging
  is also unavailable. Memory-only retention remains shutdown-unsafe until repaired.
  """

  Scenario: Directory and root barrier failures retain the provisional journal identity
    Given a stream with a failing journal directory or root durability barrier
    When a delta reaches the provisional journal but cannot be acknowledged
    Then publication is withheld and recovery owns that same journal record
    When persistence is retried twice after repair
    Then only the original journal identity is recovered

  Scenario: Combined journal and recovery staging failure retains caller ownership
    Given a stream whose journal record barrier and recovery staging both fail
    When a delta is emitted and its output owner is dropped
    Then no text is published and the original remains in bounded recovery ownership
    When storage is repaired and persistence is retried twice
    Then cold recovery contains the authentic delta exactly once

  Scenario: A warmed writer cannot resurrect a deleted session
    Given a stream that durably published a delta before its session was deleted
    When that same stream emits another delta
    Then the later delta is not published and the deleted journal stays absent

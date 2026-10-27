@done
@bug-fix
@session-management
@persistence
@phase-1
@BUG-273
Feature: Avoid history rescans and async worker stalls during shared message-store access

  """
  Use a derived completion certificate bound to exact data and index file generations. Only strict full scan can establish completeness; append and cleanup may carry it forward under the existing sidecar lease. Missing/stale certificates fall back to existing recovery. V1 and V2 entry layouts are identical; read both, publish V1 while retaining strict JSONL/hash validation. No live migration or daemon restart.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A verified complete index is reusable across current peers and cold reopen only while both exact data and index generations still match; any corruption, replacement, failed publication or legacy write invalidates that proof.
  #   2. Current readers accept byte-compatible V1 and V2 index layouts and publish a legacy-readable layout without trusting cached offsets as proof of message integrity.
  #   3. Missing originals, identity conflicts, malformed records, cleanup safety and durable publication remain protected after optimization.
  #   4. Blocking SessionSearch storage and serialization execute outside Tokio async workers, preserving per-session handlers and hook denials.
  #
  # EXAMPLES:
  #   1. Given two current stores sharing verified history When they alternate fresh appends Then each append publishes its index only once
  #   2. Given compatible legacy and current readers When they repeatedly reopen unchanged history Then neither reader repeatedly rebuilds the index
  #   3. Given a current index with a verified generation When an original is omitted from that index Then lookup recovers the intact original and conflicting append is rejected
  #   4. Given an index published by a legacy writer When a cached record contains an invalid content hash Then the current reader refuses the corrupt message
  #   5. Given a SessionSearch handler waiting for storage When the tool runs on a single async worker Then unrelated async work continues and the search result is preserved
  #
  # ========================================

  Background: User Story
    As a developer running concurrent Fspec clients
    I want to append and search message history without repeated full rescans or blocked async workers
    So that responsive agents with intact recoverable evidence

  Scenario: Current peers reuse complete history
    Given two current stores sharing verified history
    When they alternate fresh appends
    Then each append publishes its index only once


  Scenario: Legacy and current readers coexist
    Given compatible legacy and current readers
    When they repeatedly reopen unchanged history
    Then neither reader repeatedly rebuilds the index


  Scenario: Tampered indexes do not hide originals
    Given a current index with a verified generation
    When an original is omitted from that index
    Then lookup recovers the intact original and conflicting append is rejected


  Scenario: Legacy indexes do not bypass integrity
    Given an index published by a legacy writer
    When a cached record contains an invalid content hash
    Then the current reader refuses the corrupt message


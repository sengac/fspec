@done
@CMPCT-088
@session
@context-management
@rust
Feature: Compaction counts are never clamped or wrapped
  """
  rust/cli/src/compactor_sub_agent.rs clamped original/compacted token
  counts and turn counts with u32::try_from(..).unwrap_or(u32::MAX);
  rust/sessions/src/handle_impl/handle_impl_2.rs cast original_tokens
  'as u32' (wrapping) for the pre-compaction snapshot and the manual
  CompactionResult; rust/core/src/persistence/manifest_messages.rs cast
  the summary token count 'as u32'. A count that does not fit was reported
  as a false number.
  Now every site narrows through codelet_common::narrow_count(what, value),
  which returns the u32 or an error naming the count; the manual /compact
  checks before the session enters Compacting.
  Tests: rust/common/tests/cmpct088_compaction_counts_are_never_clamped.rs.
  """

  Background: User Story
    As a user reading compaction results
    I want to have a token or turn count that does not fit its 32-bit field reported as an error
    So that no compaction result or summary row carries a clamped or wrapped false number

  Scenario: A count that fits is narrowed exactly
    Given an original token count of 4294967295
    When it is narrowed for a compaction result
    Then the result is 4294967295

  Scenario: A count that does not fit is an error naming it
    Given an original token count of 4294967296
    When it is narrowed for a compaction result
    Then narrowing fails naming "original tokens" and 4294967296

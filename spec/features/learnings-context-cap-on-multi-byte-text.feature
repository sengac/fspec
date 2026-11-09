@done
@rust
@context-management
@parser
@BUG-626
Feature: Learnings context truncation panics on a multi-byte character boundary (learnings_context.rs:170)
  """
  learnings_context::format_context caps the formatted context at 8000 bytes. It used to cut with String::truncate(8000), which panics when byte 8000 is inside a UTF-8 character. The cut is now str::floor_char_boundary(8000): the kept text is the longest prefix of at most 8000 bytes that ends on a character boundary, followed by the truncation marker.
  Workspace audit (2026-11-07) of String::truncate and byte slicing at a computed length on user text, non-test code. Panic on a multi-byte character at the cut, not fixed by this card: rust/cli/src/interactive/output.rs:507 &tool_result.content[..500] (tool result preview); rust/cli/src/interactive/recovery_thinking.rs:104 &reasoning[..2000]; rust/agent-loop/src/agent_manager_handler.rs:630 and :711 &content[..2000] (subordinate turn context and search matches); rust/fspec-tui/src/app/dispatch_slash_loop.rs:212 &entry.prompt[..25] (/loop list); rust/providers/src/credentials.rs:75-76 &api_key[..6] and &api_key[len-4..] (masked key display, panics on a non-ASCII pasted key). Not a panic but inexact: rust/tools/src/file_type.rs:98 and :112 decode data[..256] / data[..1024] with str::from_utf8, which fails when the cut lands inside a character, so an SVG or ipynb detected by content is missed. Already safe: every other String::truncate restores a length the same string had earlier (progressive_output.rs:83, stream_loop/assistant.rs:31-32, compaction_execution.rs:343, transcript_assembler.rs:102); session_search_handler.rs, session_search/reassembly.rs, transcript_journal.rs, learnings_context.rs:238 and providers codex/diagnostic_redaction.rs snap to a character boundary first.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Learnings context longer than 8000 bytes is cut at the last character boundary at or before byte 8000 and then followed by the truncation marker, so the kept text is the longest prefix of at most 8000 bytes that ends on a character boundary and building the context never panics
  #   2. Context of 8000 bytes or fewer is returned whole, without the truncation marker
  #
  # EXAMPLES:
  #   1. A Learnings graph holds one decision whose 12000-byte rationale of 2-byte characters puts a character across byte 8000 of the context; the context built for its slug is its first 7999 bytes followed by the truncation marker
  #   2. The same decision with a title one byte longer puts byte 8000 on a character boundary; the context is its first 8000 bytes followed by the truncation marker
  #   3. A decision with a short rationale gives a context under 8000 bytes, returned whole without the truncation marker
  #
  # ========================================
  Background: User Story
    As a agent using DeepSearch with a Learnings graph
    I want to build learnings context from decisions, warnings and learnings whose text holds multi-byte UTF-8 characters
    So that the context is capped at 8000 bytes without a panic and keeps only whole characters

  Scenario: A context cut inside a multi-byte character keeps only whole characters
    Given a Learnings graph holding one decision whose 12000-byte rationale of 2-byte characters puts a character across byte 8000 of the context
    When the learnings context is built for the decision's slug
    Then the context is its first 7999 bytes followed by the truncation marker

  Scenario: A context cut on a character boundary keeps exactly 8000 bytes
    Given a Learnings graph holding the same decision with a title one byte longer, which puts byte 8000 on a character boundary
    When the learnings context is built for the decision's slug
    Then the context is its first 8000 bytes followed by the truncation marker

  Scenario: A context under the cap is returned whole
    Given a Learnings graph holding one decision with a short rationale of 2-byte characters
    When the learnings context is built for the decision's slug
    Then the context is returned whole, without the truncation marker

@wip
@error-handling
@resilience
@cli
@CMPCT-023
Feature: Unify compaction entry paths into a single helper
  """
  A single helper `begin_compaction_recovery` in `recovery_compaction.rs` centralizes the cross-cutting invariants (partial-text save, tracker flush, conditional user-pop, flag set with warn, progress-callback clear, event emit) so all compaction entry points become uniform. The existing `flush_partial_state_before_compaction` from CMPCT-024 becomes the internal save+flush step. Paths B and C call it with `pop_user_prompt=true`; Path D (Gemini continuation) calls it with `pop_user_prompt=false`. Path A (pre-prompt) remains structurally separate because it runs before streaming begins. The pre-existing compaction_started/progress double-emission in `compaction_retry.rs` is removed because the helper now emits them at the break site. The existing CMPCT-024/025/026 fixes (structural PromptCancelled detection, defense-in-depth flag, disagreement warn) are preserved — `classify_compaction_branch` still gates whether the helper is called.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Partial assistant text is saved via handle_final_response on every compaction recovery entry
  #   2. Token tracker is flushed from StreamingTokenDisplay on every compaction recovery entry
  #   3. The last user message is popped only when pop_user_prompt=true (Paths B, C — prompt at tail of messages)
  #   4. The last user message is NOT popped for Path D (Gemini continuation — prompt is mid-flight, not at tail)
  #   5. compaction_needed flag is always set on token_state; a warn! is emitted if it was already set (defense-in-depth disagreement)
  #   6. Tool progress callback is cleared via Uuid::nil on every compaction recovery entry
  #   7. compaction_started + compaction_progress('Context limit reached', 0, total_turns.max(1)) are emitted exactly once per recovery entry by the helper; call sites must not re-emit
  #   8. handle_compaction_retry in compaction_retry.rs no longer emits compaction_started/progress because the helper already did (avoids the pre-existing double-emit bug)
  #
  # EXAMPLES:
  #   1. Paths B, C, D produce identical session.messages + token_tracker end-states given identical starting conditions (modulo pop_user_prompt=true vs false)
  #   2. Path B (API prompt-too-long): buffer='partial answer', last msg=User('Hi'); helper with pop=true appends Assistant('partial answer'), pops User('Hi'), emits events once
  #   3. Path C (hook-cancel): buffer='streaming text', last msg=User('Do X'); helper with pop=true appends Assistant('streaming text'), pops User('Do X'), emits events once, sets compaction_needed
  #   4. Path D (Gemini continuation): buffer='', last msg=User (continuation prompt); helper with pop=false preserves all messages, emits events once
  #   5. Warning disagreement: compaction_needed=true already set on entry (shouldn't happen — indicates a disagreement between the cancel path and the flag). warn! emitted, flag remains true.
  #
  # ========================================
  Background: User Story
    As a developer
    I want to unify all compaction recovery entry paths through a single helper
    So that the invariants (partial-text save, tracker flush, user-pop, event emit, tool callback clear) are uniform and provably identical across entry points

  Scenario: Path B produces identical end state as Path C from identical starting conditions
    Given two identical sessions whose last message is a user prompt 'Hi' and whose streaming buffer contains 'partial answer'
    When begin_compaction_recovery is called on both sessions with pop_user_prompt=true
    Then both sessions end with identical session.messages (the Assistant message appended, the User 'Hi' popped)
    Then both sessions end with identical token_tracker state
    Then both sessions have compaction_needed=true on their shared TokenState

  Scenario: Path D preserves all messages when pop_user_prompt is false
    Given a session whose last message is a mid-flight User continuation prompt
    When begin_compaction_recovery is called with pop_user_prompt=false and an empty assistant_text buffer
    Then the last user message remains in session.messages
    Then no Assistant message is appended for an empty buffer
    Then compaction_needed is set to true on the TokenState

  Scenario: Helper emits compaction lifecycle events exactly once per entry
    Given a fake StreamOutput that records every emit_compaction_started and emit_compaction_progress call
    When begin_compaction_recovery is called once
    Then exactly one compaction_started event is recorded
    Then exactly one compaction_progress event with phase 'Context limit reached' is recorded

  Scenario: Helper emits a warning when compaction_needed flag was already true on entry
    Given a TokenState whose compaction_needed flag is already true before entering the helper
    When begin_compaction_recovery is invoked
    Then the helper completes successfully without error
    Then the compaction_needed flag remains true

  Scenario: Empty partial assistant text does not append an empty Assistant message
    Given a session with some existing messages and an empty assistant_text buffer
    When begin_compaction_recovery is invoked with pop_user_prompt=false
    Then the session.messages count is unchanged
    Then no empty Assistant message is added

@done
@agent-core
@rust
@session-management
@persistence
@high
@BUG-683
Feature: A resumed session refuses input until its saved state is restored
  """
  Root cause (spec/attachments/BUG-683/root-cause-research.md): resume_session put the session in the live map before restoring it, and input was admitted while the restore was still running. The turn ran on an empty generation-0 Codex history, and its checkpoint was refused as a rollback below the saved generation 1.
  codelet-sessions: BackgroundSession gets one restore check used by start_turn (send_input, steers, queued input, /loop firings), receive_incoming_message (supervisor input) and compact (manual compaction). It reads the existing manifest_restore flag that create_session_from_manifest sets and resume_session clears after restore_session_messages and restore_session_token_state succeed. A failed or unfinished restore keeps the flag; resume_session on the live session retries the restore.
  codelet-cli: block_terminal_request_failure renders the error chain without repeated layers: a layer whose text is already contained in the previous kept layer is skipped. For a CheckpointError it adds the close-and-resume instruction. No RPC wire change.
  Not changed: provider_state upsert still refuses a checkpoint below a saved generation; that refusal protected the saved generation-1 state of session b279075a.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A session resumed from its manifest refuses input until its saved history and provider state are restored: user input, queued input, steers, loop firings, supervisor messages and manual compaction
  #   2. The refusal says the session is still restoring and to send the input once the resume completes, or to resume again if the resume failed; nothing is echoed, queued or written to the session
  #   3. Once the resume completes, the Codex history is at the saved generation and a new turn's checkpoint is saved at that generation, never refused as a rollback
  #   4. A terminal provider-state failure names its cause once and says to close the session and resume it to reload its saved state
  #
  # EXAMPLES:
  #   1. A Codex session saved at generation 1 is resumed after a daemon restart; a supervisor message arrives while the restore is still loading; it is refused with the restoring message, and the manifest and provider state are unchanged
  #   2. The same session's resume completes; its Codex history is at generation 1 with the saved checkpoint, and the next turn's checkpoint is saved at generation 1
  #   3. A checkpoint persistence failure whose chain repeats 'RequestError: X' twice and 'X' once blocks the session with 'selected request requires resolution: RequestError: X' plus the close-and-resume instruction
  #
  # ========================================
  Background: User Story
    As a user running a long Codex session
    I want to resume the session after a daemon restart and keep working
    So that a slow restore never runs my next turn on empty provider state and leaves the session stuck

  Scenario: Input sent before a resumed session finishes restoring is refused and changes nothing
    Given a Codex session whose saved provider state is at generation 1
    And the daemon restarts and resumes the session, and the restore has not finished
    When the user sends input, a supervisor message arrives and a manual compaction is requested
    Then each is refused with a message saying the session is still restoring its saved history
    And the message says to send the input once the resume completes or to resume the session again
    And nothing is echoed or queued and the session stays idle
    And the session's messages and saved provider state are unchanged

  Scenario: A resumed Codex session continues at its saved generation once its restore completes
    Given a Codex session whose saved provider state is at generation 1
    And the daemon restarts and resumes the session, and the restore has not finished
    When the resume completes
    Then the session's Codex history is at generation 1 with the saved checkpoint
    And the next turn's checkpoint is saved at generation 1 without a rollback
    And input sent to the session is accepted

  Scenario: A provider-state failure names its cause once and says how to recover
    Given a request whose error chain repeats the same checkpoint persistence failure in three layers
    When the failure blocks the session
    Then the session's error names the failure once
    And it says to close the session and resume it to reload its saved state

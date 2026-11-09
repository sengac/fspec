@BUG-582
@done
@safety
@BUG-610
@persistence
@session
@providers
@critical
Feature: A panic in one session never stops other sessions' persistence
  """
  Architecture notes:
  - rust/core/src/persistence: SESSION_STORE (manifest_registry.rs) is one daemon-wide Mutex<Option<SessionStore>>. update_session_manifest used to hold it across the caller's update closure, the manifest read and commit_locked, so a panic in any closure poisoned it and every later persistence call in the daemon failed with "session store unavailable: poisoned lock: another task failed inside".
  - The store lock now guards only the in-memory cache, the last-session pointers and the writer lease map. A SessionWriter (sessions dir + the session's WriterLease) is taken under it and released; the closure, file I/O and promotion run under the session's own lease gate; the committed manifest is then recorded in the cache (never moving a cached revision backwards).
  - A poisoned lease gate is ManifestCommitError::WriterPoisoned naming the session (lock_state rule, BUG-369). release_session_ownership (session close/destroy, BUG-288) drops the lease, so the reopened session gets a fresh writer over the manifest on disk.
  - The real panic: TurnCompaction::apply -> commit_durable_compaction -> commit_compaction_admitted's admit closure (inside the manifest update) -> validate_current_compaction_history replays the whole saved history and converts it with CodexResponsesModel::prepare_request -> rig responses_api TryFrom<Message>, which called expect on Reasoning.id. Saved thinking blocks replay with no provider id. Reproduced on a copy of session bbada3c4 (227 messages, 115 thinking blocks): panic at patches/rig-core/src/providers/openai/responses_api/mod.rs:486, then load_session failed with "poisoned lock".
  - Panic logging: codelet_common::panic_log wraps each process's panic hooks (fspec combined/client/daemon, fspec-tui terminal) so the panic is logged at ERROR with thread, location, message and the session id from the agent loop's task-local scope before the earlier hook runs, once per panic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The shared session store lock guards only the in-memory cache and the writer lease map: a session's update closure, manifest file I/O and the commit run under that session's own writer lock, never under the shared store lock, so a panic in one session's persistence work never makes persistence unavailable for any other session
  #   2. A poisoned lock is never recovered silently (lock_state rule, BUG-369): after a panic inside session A's manifest writer, every later write of A fails with 'the manifest writer lock of session <A> is poisoned: an earlier update of this session panicked; close and reopen the session to recover' and the store and lease registry locks report 'the session store lock is poisoned' / 'the session writer lease registry lock is poisoned' when poisoned
  #   3. Recovery is explicit: releasing the session's writer ownership (closing or destroying the session, BUG-288 release_session_ownership) discards the poisoned writer, and the reopened session writes again from the committed manifest on disk, which a panicked update never changed
  #   4. Every panic in the daemon and TUI processes is logged at ERROR to the process log (the combined log in combined mode) with the thread name, source location, panic message and, when the panic happens inside a session's agent loop, the session id, before any earlier panic hook (terminal restore, default stderr printer) runs; one panic is logged once even when several hooks are chained
  #   5. The panic (reproduced from a copy of session bbada3c4's data): applying a self-compaction re-validates the Codex session's whole saved history inside the manifest update, and the OpenAI Responses request conversion (rust/patches/rig-core responses_api) called expect on the reasoning item id; saved thinking blocks are replayed without a provider id, so it panicked with 'An OpenAI-generated ID is required when using OpenAI reasoning items'. Reasoning without an OpenAI id is saved thinking text the Responses API cannot take as an input item (the Codex continuation carries the native reasoning items), so the conversion leaves it out of the input
  #   6. The same conversion never panics on a tool call or tool result without a call id: it returns a request error naming the item ('tool call <id> has no call id' / 'tool result <id> has no call id')
  #
  # EXAMPLES:
  #   1. Session A's update closure panics while sessions A and B are both open: B's next update_session_manifest commits revision+1, and A's next update fails with 'the manifest writer lock of session <A> is poisoned: ...'; A's manifest on disk keeps its pre-panic revision
  #   2. After A's writer is poisoned, release_session_ownership(A) (the session is closed) and A's next update commits from the manifest on disk
  #   3. A panic with message 'injected BUG-610 panic' inside session A's agent loop on thread 'bug610-worker' is in the log as an ERROR line naming the thread, the file:line of the panic, the message and session_id=<A>, written before the earlier panic hook runs, and appears exactly once with two logging hooks chained
  #   4. A Codex session whose saved history holds an assistant thinking block with no provider id applies a self-compaction: the compaction commits and the manifest gets the new compaction generation (before: panic at responses_api/mod.rs:486 inside the session store lock; reproduced on session bbada3c4's 227 messages with 115 thinking blocks)
  #   5. Converting a request whose history holds a tool result with no call id returns the error 'tool result <id> has no call id' instead of panicking
  #
  # ========================================
  Background: User Story
    As a user running several agent sessions in one fspec daemon
    I want to keep every other session working when one session's work panics
    So that one session's crash never stops every session's turns until the daemon restarts, and the crash itself is in the log

  Scenario: A Codex session whose saved history holds reasoning without an id compacts
    Given a Codex session whose saved history holds an assistant thinking block with no provider id
    When the session applies a self-compaction of its history
    Then the compaction commits a new compaction generation
    And the session's manifest can still be written

  Scenario: A tool call or tool result without a call id is a request error, not a panic
    Given a request whose history holds a tool result with no call id
    When the request is converted for the OpenAI Responses API
    Then the conversion fails with "tool result <id> has no call id"
    And a request whose history holds a tool call with no call id fails with "tool call <id> has no call id"

  Scenario: Another session still commits after one session's update panics
    Given two open sessions A and B in one daemon
    When session A's manifest update panics inside its update closure
    Then session B's next manifest update commits its next revision
    And session A's committed manifest keeps its revision from before the panic

  Scenario: The panicked session reports its poisoned writer until it is reopened
    Given session A's manifest update panicked inside its update closure
    When session A writes its manifest again
    Then the write fails with the poisoned writer error naming session A
    When session A is closed, releasing its writer ownership
    Then session A's next manifest update commits from the manifest on disk

  Scenario: A panic is logged with its thread, location and session before the earlier hook runs
    Given the panic logging hook is installed over an earlier panic hook
    When code running for session A panics on thread "bug610-worker"
    Then the log holds one ERROR line with the panic message, the thread name, the panic location and session A's id
    And the line was written before the earlier hook ran
    And a second chained logging hook does not log the panic again

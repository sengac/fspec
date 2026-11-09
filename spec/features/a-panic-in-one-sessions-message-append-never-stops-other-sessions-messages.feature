@done
@safety
@BUG-625
@persistence
@session
@critical
Feature: A panic in one session's message append never stops other sessions' messages
  """
  rust/core/src/persistence: MESSAGE_STORE (manifest_registry.rs) becomes Mutex<Option<Arc<MessageStore>>>; message_store() takes it only to initialize or clone the handle. MessageStore methods take &self: in-process and cross-process writers are serialized by the messages.lock file lock (a new open file description per operation, so flock also excludes threads of one process). StoreState's mutex is held only for in-memory index/cache work; index snapshots are taken under it and written after it is released. The panic is injected with messages::durability::panic_at(boundary, op), a thread-local test hook like observe(), at the message.sync boundary inside append_message.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The daemon-wide message store lock guards only the message store handle: it is held to create the store or take a shared handle to it and is released before any message is read, written, indexed or cleaned up, so a panic in message store work never poisons it
  #   2. Inside the message store, the in-memory index lock guards only in-memory reads and writes of the index and cache; message file reads and writes, index file writes and recovery scans run outside it, serialized by the message store's file lock, which a panic releases when it unwinds
  #   3. A message append inside a session's manifest update runs under that session's own writer lock (BUG-610), so a panic in session A's append poisons only A's writer: every later write of A fails with BUG-610's exact error 'the manifest writer lock of session <A> is poisoned: an earlier update of this session panicked; close and reopen the session to recover' until A is closed and reopened, while every other session keeps appending, admitting, reading and updating messages
  #   4. A poisoned message store lock is never recovered silently (BUG-369 lock_state rule): it is reported as 'the message store lock is poisoned' or 'the message store index lock is poisoned', and since neither lock is held across work that can panic, either one means a bug in that in-memory work
  #   5. The panic is not caught: it unwinds as a real panic, so the daemon's panic logging hook (codelet_common::panic_log, BUG-610) logs it with the session id before the earlier hook runs
  #
  # EXAMPLES:
  #   1. A panic injected at session A's message durability boundary (message.sync) during A's append on thread bug625-worker: B's next append_message commits and B's manifest references the new message, which get_message returns; A's committed manifest keeps its pre-panic messages
  #   2. After the injected panic, A's next append_message fails with 'the manifest writer lock of session <A> is poisoned: ...'; release_session_ownership(A) and A's next append commits
  #   3. After the injected panic, B's append_message_idempotent, store_original_idempotent followed by admit_stored_message, update_message_metadata, get_session_messages and cleanup_orphaned_messages all succeed
  #   4. The injected panic reaches the process panic hook with the message 'injected panic at message.sync'
  #
  # ========================================
  Background: User Story
    As a user running several agent sessions in one fspec daemon
    I want to keep every other session's messages persisting when one session's message append panics
    So that one session's crash in the message store never stops every session's turns until the daemon restarts

  Scenario: Another session still appends after one session's message append panics
    Given two open sessions A and B in one daemon
    When session A's message append panics inside the message store
    Then the panic reaches the process panic hook with the message "injected panic at message.sync"
    And session B's next message append commits and B's manifest references the new message
    And session A's committed manifest keeps its messages from before the panic

  Scenario: The panicked session reports its poisoned writer until it is reopened
    Given session A's message append panicked inside the message store
    When session A appends a message again
    Then the append fails with the poisoned writer error naming session A
    When session A is closed, releasing its writer ownership
    Then session A's next message append commits

  Scenario: Every message store operation still works after one session's append panics
    Given session A's message append panicked inside the message store
    When session B appends a message idempotently
    And a retained original is stored and admitted to session B
    And the metadata of B's message is updated
    Then session B's messages read back with the updated metadata
    And orphaned message cleanup runs without an error

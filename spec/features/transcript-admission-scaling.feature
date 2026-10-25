@done
@agent-core
@session-management
@persistence
@BUG-255
Feature: Avoid full transcript replay on every streamed journal append
  """
  Reuse validated admission indexes only within a journal writer. Detect changed directory or tail stamps and rebuild; cold opens and all explicit reads validate every immutable record. Use the existing writer lease and durability boundaries; mutate indexes only after durable acknowledgement, invalidate on errors. Recovery lanes retain a writer and use per-session mutexes, with global lookup held only to clone a lane handle. Deterministic admission observations count record reads and lifecycle work; use real temporary journals and controlled filesystem stalls.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After validated opening, fixed-size streamed deltas use incremental sequence, identity and lifecycle indexes instead of rereading or reprojecting accumulated immutable history; cold opens and explicit reads still validate corruption and completeness.
  #   2. Recovery ownership is serialized per session, never by a process-global mutex held across filesystem work. Admission remains durable before publication, idempotent on retry, and rejects deleted owners.
  #
  # EXAMPLES:
  #   1. Growing batches beyond 128 deltas after completed turns and a large authentic tool result perform bounded record reads and one lifecycle admission per new delta; a cold reopen reconstructs exact history and detects corruption or missing ordinals.
  #   2. While session A is blocked at a real filesystem publication boundary, session B durably admits and publishes its own event; retrying A preserves identity and deletion still retires its lane.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Which observable guarantees must the warm admission optimization retain, including external corruption and independent sessions?
  #   A: The attachment and existing integrity specs require validated cold reconstruction, corrupt/incomplete history rejection, exact retry identity, lifecycle checks, ownership and durable-before-publication. Warm private immutable history may reuse validation while directory/tail changes invalidate it; explicit reads and cold opens always validate history. Per-session ownership must isolate filesystem waits.
  #
  # ========================================
  Background: User Story
    As a session user
    I want to stream long sessions without accumulated history replay or cross-session stalls
    So that durable output remains responsive while retaining integrity and ownership guarantees

  Scenario: Long streaming sessions require bounded admission work
    Given completed turns and a large authentic tool result in a durable session
    When growing batches of more than 128 deltas are acknowledged
    Then historical record reads and lifecycle work grow linearly

  Scenario: Cold opening validates immutable history before admission
    Given a journal with acknowledged deltas and completed tools
    When a fresh writer opens intact corrupt or incomplete history
    Then intact history resumes exactly and invalid history refuses admission

  Scenario: An unrelated session progresses during a stalled journal append
    Given two sessions and an exclusively locked first transcript writer
    When both sessions attempt durable transcript admission
    Then the second session completes without waiting for the first writer

  Scenario: Retried publication retains durable identity
    Given a transcript admission whose directory acknowledgement fails
    When the same record is retried and streaming continues
    Then recovery contains each record exactly once in durable order

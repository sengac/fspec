@phase-1
@context-management
@agent-core
@persistence
@session-management
@high
@integration-test
@BUG-284
Feature: Recover durable submission receipts
  """
  Independent review refinement: terminal cancellation/failure facts after admission must be durable and model visible, not only UI transcript events. Uncertain post-crash dispatch blocks continuation until explicit recovery. Receipt and recovery-spool originals/blobs need GC roots, and fork/merge must not grant execution rights. Submissions are retained as versioned durable receipts (core persistence input_receipts) before queue acknowledgment; admission appends the stored original to conversation membership under the runtime lock and installs it before dispatch. Synchronous BackgroundSession submission now returns storage and queue refusals, but ordinary-input RPC/handle callers still discard that result and no external idempotency key exists; that external acknowledgment limitation is explicit and out of this scope. The focused CLI stream_loop decomposition was approved on 2026-10-28.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Preserve every submitted input as an audit/recovery record while keeping the provider conversation, cold restore and compaction replay on the same explicitly admitted ordered history.
  #   2. Strict checkpoint lineage rejection must remain effective for altered assistant content, tool invocation, tool outcome, missing evidence and DAG ancestry; diagnosis does not authorize a bypass or history deletion.
  #   3. Do not claim live repair until the exact loaded executable is identified and repeated realistic compaction, restore and continuation succeed without repeated effects.
  #   4. Admission is a durable monotone commitment under cancellation ownership: Pending becomes Cancelled or Admitted; cancellation after admission stops execution but cannot remove or requeue the admitted input. The exact original is installed in runtime before dispatch ownership is released.
  #   5. A submission has one immutable UUID/content/origin identity retained before queue acknowledgment. Identical text with distinct UUIDs is legitimate; same-UUID retries are idempotent and conflicting identity, unknown state versions or contradictory membership refuse inference.
  #   6. Lost acknowledgment after manifest promotion must settle the same receipt under ownership; unresolved authority blocks inference. Process recovery restores admitted originals once and never auto-executes Pending or Cancelled submissions or assumes remote completion.
  #   7. Legacy divergent sessions remain fail-closed with actionable evidence-preserving recovery diagnostics. No text-prefix filtering, guessed disposition, silent historical migration, original deletion, checkpoint relaxation or tool replay is permitted.
  #
  # EXAMPLES:
  #   1. Preserved session 4d8eedb6-fe1d-43cb-9aa9-70252fb8cd61 has six supervisor originals persisted just after cancellation but absent from its provider checkpoint. Actual CodexHistory replay rejects; a cloned diagnostic request omitting only those six accepts.
  #   2. Preserved session 7b41d57c-d7aa-47f3-b2c7-b689012703c4 has two such supervisor originals. Actual CodexHistory replay rejects; omitting those two only in an isolated diagnostic clone accepts; changed tool outcomes remain rejected.
  #   3. Six supervisor submissions queued before interruption, or two arriving during interruption, are each retained once but absent from runtime and Codex selected-history replay; genuine later user input remains admitted and compaction does not fail lineage.
  #   4. Cancellation while prompt hooks run, while the original is being retained, or while the runtime lock is held produces a recoverable canceled receipt and no actionable canceled instruction; a later fresh prompt may run once.
  #   5. Cancellation after durable admission, provider construction failure or pre-prompt compaction failure preserves the accepted instruction exactly once across runtime and cold restore without silently retrying its effects.
  #   6. After parallel tools complete once, two automatic compactions separated by cancellation, cold restore and fresh continuation retain exact tool receipts, valid provider generations and the same admitted conversation; no command runs again.
  #   7. A lost durability acknowledgment or process death never loses the original or grants inference from an unresolved receipt. Reopening restores admitted inputs once, keeps canceled or pending inputs audit-only, and preserves image payloads and duplicate-text distinct submissions.
  #   8. All four sanitized failed-history derivatives still refuse their original divergent lineage; altering assistant content, invocation, outcome, missing evidence or DAG ancestry remains rejected. Inspecting a clone never mutates failed historical evidence or makes a provider request.
  #
  # ========================================
  Background: User Story
    As a agent operator
    I want to cancel submitted work without losing its recovery evidence or corrupting the admitted conversation
    So that compaction and cold continuation preserve strict history and never repeat completed effects

  Scenario: Submission identity survives retries images and identical prompt text
    Given two intentional submissions have identical text and image payloads
    When one receipt is retried and a conflicting identity is attempted
    Then the two distinct originals remain recoverable and the retry adds no duplicate
    And the conflicting identity or unknown receipt version blocks inference
    And garbage collection preserves pending receipt and recovery spool blob evidence
    And fork or merge cannot grant execution rights from pending or canceled receipts

  Scenario: Durability acknowledgment loss never grants unresolved execution authority
    Given an input is being promoted at a manifest durability boundary
    When durability acknowledgment is lost after rename or storage remains unavailable
    Then the same receipt identity is settled or inference remains blocked
    And the original evidence is preserved without requeueing it under a new identity
    And a local attempt retry cannot be mistaken for a new externally identified submission

  Scenario: Process death preserves input disposition without replaying effects
    Given an input is pending admitted installed or beginning provider dispatch
    When the process dies and persisted state is reopened
    Then only admitted inputs restore into the conversation exactly once
    And pending and canceled inputs remain recoverable without automatic execution
    And uncertain dispatch blocks continuation until explicit recovery records a truthful model visible disposition

  Scenario: Process death after queue acknowledgment retains the pending submission
    Given a pending submission was acknowledged before the task loop dequeued it
    When the submitting process is killed and persisted state is reopened
    Then the pending original remains recoverable exactly once
    And it is absent from conversational replay and cannot execute automatically

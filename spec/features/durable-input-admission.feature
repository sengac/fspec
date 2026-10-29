@phase-1
@context-management
@agent-core
@persistence
@session-management
@high
@integration-test
@BUG-284
Feature: Keep canceled input audit receipts out of the admitted Codex conversation
  """
  Independent review refinement: terminal cancellation/failure facts after admission must be durable and model visible, not only UI transcript events. Uncertain post-crash dispatch blocks continuation until explicit recovery. Receipt and recovery-spool originals/blobs need GC roots, and fork/merge must not grant execution rights. Current synchronous submission APIs, discarded ordinary-input RPC errors and absent caller submission IDs leave the external acknowledgment/idempotency guarantee unresolved. Do not narrow that guarantee silently or edit concurrent RPC/schema ownership; report and coordinate any exact additional file scope before implementation. CLI decomposition approval is separate and remains pending.
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

  Scenario: Untouched accepted history remains eligible for selected compaction
    Given a genuine Codex conversation with a completed tool effect
    When no canceled submission has been added
    Then selected compaction publishes a valid generation without repeating the effect

  Scenario: Six supervisor inputs canceled before admission remain audit only
    Given a genuine Codex conversation with a completed tool effect
    When six queued supervisor inputs are canceled before admission
    Then all six originals remain recoverable exactly once without executing
    And a fresh instruction completes on the unchanged runtime
    And selected compaction accepts the same admitted history as runtime

  Scenario: Two supervisor inputs submitted during interruption remain audit only
    Given a genuine Codex conversation with a completed tool effect
    When two supervisor inputs arrive while the session is interrupted
    Then both originals remain recoverable exactly once without executing
    And a fresh instruction completes on the unchanged runtime
    And selected compaction accepts the same admitted history as runtime

  Scenario: Cancellation during a prompt hook preserves audit without conversation membership
    Given a genuine Codex conversation and a prompt held by a submission hook
    When the prompt is canceled before the hook releases
    Then the original remains recoverable without executing
    And a fresh instruction completes on the unchanged runtime
    And selected compaction accepts the same admitted history as runtime

  Scenario: Cancellation during original persistence preserves audit without conversation membership
    Given a genuine Codex conversation and a prompt held at durable original persistence
    When the prompt is canceled before persistence releases
    Then asynchronous work remains responsive and the original remains recoverable
    And a fresh instruction completes on the unchanged runtime
    And selected compaction accepts the same admitted history as runtime

  Scenario: Cancellation while awaiting the runtime lock does not admit the input
    Given a genuine Codex conversation and a submitted prompt waiting for runtime ownership
    When the prompt is canceled before runtime ownership becomes available
    Then the original remains recoverable without executing
    And cold conversation replay equals the admitted runtime history

  Scenario: Interrupted execution cannot revoke an already admitted input
    Given a submitted input has won durable admission and runtime installation
    When execution is interrupted before provider dispatch
    Then the admitted input remains in runtime and cold replay exactly once
    And a fresh subsequent input may continue without replaying the old input as new work
    And truthful durable cancellation evidence is model visible before subsequent inference

  Scenario: Provider setup and pre-prompt compaction failure cannot split admitted history
    Given an admitted input waiting for its first provider request
    When provider construction or pre-prompt compaction fails
    Then runtime and cold replay retain the same admitted original exactly once
    And no successful provider completion or tool effect is fabricated

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

  Scenario: Cancellation and cold continuation survive successive automatic compactions
    Given a real queued conversation has completed parallel tool effects once
    When canceled inputs and fresh admitted inputs span two automatic compactions and cold continuations
    Then each committed generation retains exact original outcomes and valid DAG ancestry
    And no canceled instruction or completed tool effect executes again
    And admitted cancellation and failure dispositions remain model visible after both compactions

  Scenario: Legacy divergent histories and altered ancestry remain fail closed
    Given four sanitized derivatives of preserved failed Codex histories
    When their divergent histories or altered assistant tool or DAG evidence are checked
    Then strict provider lineage refuses the incompatible continuation
    And no provider request runs and all historical evidence bytes remain unchanged

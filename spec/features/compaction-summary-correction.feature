@done
@agent-core
@context-management
@compaction
@BUG-614
Feature: Rejected compaction summaries are corrected or left uninstalled
  """
  TurnCompaction (agent-loop/src/turn_compaction.rs) counts rejected submissions per turn. A rejection is a typed InjectSummaryError::Rejected carrying the exact validator error, attempt and limit; InjectSummaryTool turns it into a non-error JSON result {status: rejected | rejected_final, attempt, max_attempts, error, installed: false, syntax, instruction}. Other handler failures stay ordinary tool errors.
  interactive/compaction_handoff.rs requires_handoff returns a HandoffOutcome (Queued, Continue, NotInstalled(error)) and never an error: no inject_summary or GenerateCompaction result can change durable state before commit, so none of them may block inference. stream_loop/tool_result.rs emits StreamEvent::SummaryNotInstalled for NotInstalled (background output maps it to a Warning user notification 'Summary not installed: <error>') and discards an in-view builder view so the turn continues on the uncompacted context.
  DAG_NODE_SYNTAX, DAG_RANGE_RULES and FILE_METADATA_INSTRUCTION move to codelet-tools inject_summary (the lowest crate that needs them) so the tool description can show them; cli compaction_dag and the compactor import them from there.
  Truncation (Q5): a candidate whose text ends inside an unclosed dag-node gets inject_summary::truncated_dag_feedback(complete_nodes); PROV-040's build_truncation_recovery_message uses the same text for a truncated inject_summary call.
  Compactor (Q7/Q8): compaction_chunking validates every response inside one synthesis conversation. Complete candidates are validated with the operation's DagValidationContext; truncated chunks (output-limit stop reason, or text ending inside an unclosed dag-node) have their complete-node prefix structure-checked before continuation. A rejection appends the candidate as the assistant turn and the exact error as the user turn, up to 3 candidates; build_validated_dag's restart loop and the depth-0 correction re-exploration are removed.
  Q4: agent_loop retries a failed retryable builder for every next input that is not an internal retry or a manual /compact (which restarts it itself); retry_failed_compactor_for_user_input becomes retry_failed_compactor_for_next_input.
  Q3 (threshold exhaustion while the context still fits) was not decided separately: this card keeps the existing hold after a failed threshold compaction and only widens who may retry (Q4). Durable-authority blocks (publication, identity, pending commit) are unchanged and are still cleared by restore_compaction_boundary.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rejection is non-fatal: an inject_summary candidate that fails DAG validation returns to the same model in the same turn as a 'rejected' tool result with the exact validator error, the DAG syntax and the attempt count (product owner Q1, 2026-11-07)
  #   2. Ordinary turns keep inject_summary; its tool description shows the exact DAG node, range and file syntax (product owner Q1, 2026-11-07)
  #   3. Every submission path has one limit of 3 validation attempts: inject_summary per turn, GenerateCompaction and threshold compaction per operation (derived Q6, 2026-11-07)
  #   4. When the attempts run out nothing is installed, committed memory is unchanged, the turn continues on the uncompacted context and the UI shows 'Summary not installed: <exact error>' (product owner Q2, 2026-11-07)
  #   5. A DAG cut off mid-node (or a truncated inject_summary tool call) is reported as truncated after N complete nodes and the model is asked for a whole new, shorter DAG; inject_summary has no continuation mode (product owner Q5, 2026-11-07)
  #   6. Self-targeted GenerateCompaction that exhausts its attempts returns a non-fatal 'rejected_final' result to the caller and never blocks inference (Q2 applied to the self-target path, 2026-11-07)
  #   7. Any next input, including background and supervisor messages such as AgentManager reports, starts a fresh compaction retry with a fresh budget for a failed threshold compaction; this replaces BUG-211's user-input-only rule (product owner Q4, 2026-11-07)
  #   8. A correction continues the same generating conversation: the rejected candidate and the exact error go back to the model that wrote it (derived Q7, 2026-11-07)
  #   9. Validation runs on each completed candidate, and on each chunk's complete nodes for the compactor sub-agent, before it is treated as final; a compactor candidate that ends mid-node is continued rather than restarted (derived Q8, 2026-11-07)
  #   10. A candidate rejection never sets a permanent inference block; only real durable-authority failures (publication or identity loss, pending commit) may block, and a restore still clears such a block
  #
  # EXAMPLES:
  #   1. Incident 21b391d2: in an ordinary turn the main model calls inject_summary with a markdown handoff note and no dag-node; the result is 'rejected', attempt 1/3, with 'invalid DAG candidate: missing or unclosed dag-node structure' and the syntax; the model's next request in the same turn carries it, and a later AgentManager report runs ordinary inference instead of failing before dispatch
  #   2. After one rejection the model resubmits a valid DAG in the same turn; the second submission is queued and the turn's reconciliation commits it
  #   3. The model submits three invalid DAGs in one turn; the third result is 'rejected_final' with installed false, the UI shows 'Summary not installed: invalid DAG candidate: ...', committed memory is unchanged and the model finishes its answer
  #   4. The model submits two complete dag-nodes and then an unclosed third; the result says the DAG was truncated after 2 complete nodes and asks for a whole new, shorter DAG
  #   5. An inject_summary tool call cut off by the output limit gets a recovery message saying the DAG was truncated after N complete nodes and to resubmit a shorter DAG, not the Bash heredoc advice for file writes
  #   6. The inject_summary tool description shows the dag-node attribute syntax, the inclusive range rules and the dag-files syntax
  #   7. Self-target GenerateCompaction receives three malformed candidates; the caller's turn continues with a 'rejected_final' result, nothing is installed and the next input runs ordinary inference
  #   8. Threshold compaction fails before committing; an AgentManager background report retries the builder with a fresh budget, commits the summary and then runs ordinary inference
  #   9. A publication failure blocks inference; restoring the committed session clears the block and the next input runs ordinary inference
  #   10. The compactor returns a candidate without dag-nodes; the correction request continues the same synthesis conversation with the rejected candidate as the assistant turn followed by the exact error
  #   11. A compactor chunk stops at the output limit after a malformed but closed dag-node; the correction is requested at once instead of a continuation
  #   12. A compactor candidate ends inside an unclosed dag-node with a normal stop reason; it is continued from its complete nodes rather than restarted
  #
  # ========================================
  Background: User Story
    As a user running a long fspec agent session
    I want to have a rejected compaction summary corrected by the model that wrote it, or left uninstalled without blocking the session
    So that a malformed DAG never stops the session from accepting the next input

  Scenario: A prose summary in an ordinary turn returns the exact error to the model
    Given a main model in an ordinary turn with the production summary handler
    When it calls inject_summary with a markdown handoff note that has no dag-node
    Then the tool result is rejected attempt 1 of 3 with the exact validator error and the DAG syntax
    And the model's next request in the same turn carries that result
    And nothing is installed and ordinary inference is not blocked
    And a later background AgentManager report runs ordinary inference

  Scenario: A corrected resubmission in the same turn is committed
    Given a main model whose first inject_summary candidate was rejected
    When it resubmits a valid DAG in the same turn
    Then the second submission is queued and the turn stops at the tool boundary
    And the turn's reconciliation commits the corrected DAG

  Scenario: Three rejected submissions leave the summary uninstalled and the turn continues
    Given a main model that submits an invalid DAG three times in one turn
    When the third submission is rejected
    Then the result is rejected_final with installed false
    And the UI shows Summary not installed with the exact validator error
    And committed memory is unchanged and the model finishes its answer
    And the next input runs ordinary inference

  Scenario: A DAG cut off mid-node asks for a whole shorter DAG
    Given a main model whose inject_summary content ends inside an unclosed dag-node after 2 complete nodes
    When the submission is validated
    Then the result says the DAG was truncated after 2 complete nodes
    And it asks for a whole new shorter DAG

  Scenario: A truncated inject_summary tool call gets DAG truncation feedback
    Given an inject_summary tool call cut off by the output limit after 3 complete nodes
    When the truncated tool call recovery message is built
    Then it says the DAG was truncated after 3 complete nodes and asks for a whole new shorter DAG
    And it does not give the Bash heredoc advice for file writes

  Scenario: Exhausted self-target generation leaves the caller's turn running
    Given a main model that calls GenerateCompaction for its own session
    When the compactor's three candidates are all malformed
    Then the caller receives a rejected_final result and its turn continues
    And nothing is installed and the UI shows Summary not installed
    And the next input runs ordinary inference

  Scenario: A durable-authority block stays reachable by restore
    Given ordinary inference is blocked for an unresolved publication failure
    When the committed session is restored
    Then the block is cleared
    And the next input runs ordinary inference

  Scenario: Compactor correction continues the same synthesis conversation
    Given a compactor whose synthesized candidate has no dag-node
    When the candidate is rejected
    Then the correction request carries the rejected candidate as the assistant turn followed by the exact error
    And a valid corrected candidate becomes committed memory

  Scenario: A malformed node in a truncated compactor chunk is corrected at once
    Given a compactor chunk that stops at the output limit after a closed but malformed dag-node
    When the chunk ends
    Then the next request is a correction carrying the exact error rather than a continuation

  Scenario: A compactor candidate ending mid-node is continued
    Given a compactor candidate that ends inside an unclosed dag-node with a normal stop reason
    When the response ends
    Then the next request continues from its complete nodes rather than restarting
    And the assembled DAG is validated before it is returned

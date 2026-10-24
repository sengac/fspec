@done
@BUG-242
@phase-1
@agent-core
@providers
@context-management
@compaction
Feature: Compaction settles checkpoint obligations before synthesis
  """
  Exploration cutoff drops the requester, not the independently running rig tool
  server. Cancellation is therefore not inferred from timeout. A settlement
  barrier compares genuine captured results with native provider obligations
  before synthesis admission. Unresolved execution receives an explicit runtime
  outcome-unknown handoff record, not a purported tool result. It states that
  execution may continue and must not be retried. No success, cancellation or
  non-execution is inferred. Synthesis uses only genuine gathered evidence;
  the existing full-source validation still rejects invented or incomplete DAGs.
  Invalid native lineage remains an explicit failure, never a lineage reset.
  Native item IDs, call IDs, reasoning, response attempts and generation remain
  authoritative. Completed evidence is retained exactly once. Diagnostics contain
  operation, phase and pending call identities only, never tool payloads.
  The absolute deadline, full-source DAG validation and publication gates apply
  unchanged to registered recovery and direct generation.
  """

  Scenario: A blocked first checkpointed call permits truthful unknown outcome synthesis
    Given an actual Codex compactor checkpoints a first call that starts but produces no result
    When exploration expires while that call is still executing
    Then tool-free synthesis receives explicit outcome unknown records with native call identities
    And native lineage survives without claiming success cancellation or non-execution
    And late actor completion does not rewrite the settled handoff or repeat execution

  Scenario: Partial batches preserve genuine evidence without inventing later outcomes
    Given an actual Codex compactor reads originals before a blocked call and an unstarted later call
    When exploration expires inside that checkpointed batch
    Then tool-free synthesis marks both unresolved calls outcome unknown without claiming cancellation or success
    And the completed originals result survives exactly once with authentic identities
    And admitted synthesis preserves the selected model reservation generation and absolute deadline without repeated execution

  Scenario: Completed checkpoint evidence crosses the synthesis barrier
    Given an actual Codex compactor completes its originals call before its next response stalls
    When exploration expires with no unresolved tool outcome
    Then physical tool-free synthesis retains the original result and native reasoning exactly once
    And the selected model output reservation and checkpoint generation remain unchanged

  Scenario: Both publication routes preserve the prior DAG on invalid native lineage
    Given registered recovery and direct generation have a previously committed DAG
    When actual Codex exploration expires after its checkpoint model identity changes
    Then both routes report settlement failure without resetting the absolute deadline
    And the prior durable generation remains authoritative without a completion event

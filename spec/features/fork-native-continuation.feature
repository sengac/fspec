@done
@providers
@persistence
@session-management
@context-management
@BUG-257
Feature: Preserve valid Codex continuation state when forking a session
  """
  Fork construction verifies parent sidecar (or legacy state), selects only checkpoints anchored exactly at the child prefix, recalculates child session/revision/message-source prefix anchors, durably publishes a child root before its manifest, and leaves parent bytes unchanged. Unsupported historical/committed-compaction boundaries fail closed. The existing 980-line manifest file must be mechanically split into sub-300-line type/store/facade sections before editing fork; preserve public exports and lexical visibility, no unrelated behavior refactor. Test cold restoration and captured physical requests using the existing offline loopback-only Codex fixture; no external provider calls.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A supported Codex fork reanchors verified provider continuation to the child's own session identity, revision and exact selected message prefix. The next physical request preserves native reasoning and tool identity without later parent content. Provider checkpoints remain outside originals.
  #   2. Reject missing, corrupt or unsupported checkpoint boundaries before publishing a child; do not relax lineage validation, synthesize a reset, copy a parent-bound state root or mutate the parent's manifest, state or originals. Retain legacy checkpoint and compaction-boundary behavior.
  #
  # EXAMPLES:
  #   1. Fork the final completed Codex tool turn, cold restore the child and send a captured physical next request. Its intended native prefix and tool IDs survive, later parent work is absent, and parent bytes remain unchanged.
  #   2. Fork an older boundary without a retained matching checkpoint, or with corrupted state: reject explicitly and publish no child. Legacy checkpoint forks still restore and pre-compaction boundaries remain rejected.
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should a fork handle older boundaries lacking an exact retained native checkpoint, or unsupported committed compaction lineage?
  #   A: Per the attachment, support exact retained checkpoint boundaries and reject unsupported earlier prefixes explicitly before publishing a child. Never truncate opaque native items or copy a parent-bound root. Preserve existing pre-compaction boundary rejection; committed compaction lineage requires its own valid child-bound transition and must fail closed rather than silently import later history.
  #
  # ========================================
  Background: User Story
    As a Codex session user
    I want to fork a completed turn and continue its exact selected conversation prefix
    So that the child cold restores without importing later parent history or changing the parent

  Scenario: Final completed Codex fork cold continues its exact prefix
    Given completed tool dialogue with a separate native continuation checkpoint
    When its final message is forked and the child cold sends a physical next request
    Then native reasoning and tool identities survive without changing parent evidence

  Scenario: Retained exact boundary excludes later parent work
    Given a completed checkpoint followed by later parent conversation
    When the checkpoint prefix is forked and cold continued physically
    Then the child request excludes all later parent content and originals remain separate

  Scenario: Unsupported older boundaries fail before publishing a child
    Given a Codex session whose only checkpoint is later than the selected prefix
    When that older boundary is forked
    Then an explicit unsupported boundary error leaves parent and session inventory unchanged

  Scenario: Corrupt provider state prevents fork publication
    Given a parent with a corrupt referenced native checkpoint
    When its completed prefix is forked
    Then fork fails explicitly without publishing a child or changing parent evidence

  Scenario: Legacy checkpoints and compaction boundaries remain safe
    Given a legacy checkpoint session and an existing compacted conversation
    When a final legacy checkpoint and a forbidden pre-compaction boundary are forked
    Then legacy continuation restores and the forbidden boundary remains rejected

  Scenario: Failed child checkpoint publication preserves parent ownership
    Given a completed Codex parent with durable native state
    When child checkpoint synchronization fails during fork
    Then no child manifest is acknowledged and parent continuation remains recoverable

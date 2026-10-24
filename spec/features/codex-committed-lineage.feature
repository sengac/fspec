@done
@CMPCT-078
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Bind Codex continuation checkpoints to full committed lineage
  """
  Implementation:
  - P1; CB-27. Read attached cmpct-078-codex-lineage.md and 00-shared-contract.md. Evidence classification and exact source anchors are in the specific brief. No runtime reproduction claimed. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  - Continuation binds the complete non-reminder ancestry and fixed full-current-DAG protocol. Model or DAG replacement requires an explicit durable generation transition, superseding historical implicit checkpoint replacement behavior. A transition records a generation-bearing empty checkpoint, not an absent checkpoint; the first request binds the new ancestry and model while preserving its supplied suffix.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Validate model/protocol identity, committed DAG generation and full relevant assistant/tool ancestry before reusing a provider checkpoint.
  #   3. DAG installation performs an explicit recorded durable generation transition; compatible suffix work survives and old checkpoint content cannot reappear.
  #   4. Incompatibility returns typed failure or uses the preselected full-current-DAG protocol after its recorded commit-time reset; never error-driven silent repair.
  #
  # EXAMPLES:
  #   1. Given the same user prefix but changed assistant/tool history, when checkpoint reuse is requested, then stale saved non-user content is not sent.
  #   2. Given a valid committed DAG transition plus suffix and a cold resume, when another Codex request is built, then it references only the compatible generation.
  #
  # ========================================
  Background: User Story
    As a Codex session owner
    I want to continue only the committed DAG generation
    So that changed assistant and tool ancestry cannot resurrect stale context

  Scenario Outline: Incompatible continuation cannot send stale checkpoint content
    Given a durable Codex checkpoint for a committed DAG generation
    And the current request changes the checkpoint's <identity>
    When I continue the session using the same user prefix
    Then I receive an explicit lineage failure before any provider request
    And the last committed checkpoint and generation remain unchanged
    And no completion or alternate request is emitted

    Examples:
      | identity             |
      | assistant ancestry   |
      | tool call ancestry   |
      | tool result ancestry |
      | model                |
      | protocol             |
      | DAG generation       |

  Scenario: Compatible full-current-DAG continuation preserves recorded suffix work
    Given a durable Codex checkpoint matching the full current DAG lineage
    And compatible assistant and tool outcomes extend the committed prefix
    When I continue the session in explicit full-current-DAG mode
    Then the captured provider request contains the current DAG and compatible suffix exactly once
    And opaque recorded reasoning and required reminder identities are preserved

  Scenario: Durable generation transition survives cold resume without old checkpoint content
    Given a committed Codex session with recorded suffix work
    And preparing the next generation checkpoint does not mutate memory or call its writer
    When a validated DAG installation durably records a new generation and provider transition
    And the already durable checkpoint is installed without a second persistence transaction
    And I cold resume the session and send an explicit full-current-DAG request
    Then the captured request contains only the new committed generation and compatible suffix
    And replaced assistant and tool history cannot reappear from the old checkpoint

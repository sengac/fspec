@done
@CMPCT-083
@phase-1
@integration
@compaction
@context-management
@high
Feature: Gate compaction repair on cold-resume and no-fallback end-to-end tests
  """
  Implementation:
  - P1; CB-01–28 acceptance gate. Read attached cmpct-083-end-to-end-gate.md and 00-shared-contract.md. Current runtime results are recorded in gate-supervisor-finish-2026-10-18.md; mapping presence alone is not acceptance evidence. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  - Ordered suffix pairing is checked in each selected adapter's native representation. Codex Responses requires exactly one function_call and one subsequent function_call_output with matching call_id, function arguments and complete result; a call's id and call_id fields are not duplicate calls.
  - Retained originals use the supported SessionSearch originals action with complete envelope equality and unchanged durable state. Every provider row repeats restore after a completed request and checks ordered history and billing idempotence. Lost-notification reconciliation requires a typed committed-context refresh without any fabricated completion notification.
  - The first cold request and the request after repeated restore retain separate wire captures, timestamps and manifest evidence, and both receive the complete lineage and suffix checks. Each ordinary selected HTTP send must produce exactly one captured physical request; selecting a final correct body cannot conceal earlier alternate requests.
  - Synthetic model limits reserve the actual selected adapter's output allowance. Z.AI keeps its native 8192-token builder allowance in both endpoints and restores that same reservation from the fixture registry; admission validation is never bypassed or the native request shrunk to fit an unrelated generic fixture limit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Verify live stream → submitted candidate → durable commit → next physical request → fresh-process resume → another request with matching intended lineage.
  #   3. Cover all supported built-in provider adapters plus explicit custom/stub contracts, retained source retrieval, durable suffix/tool pairing and coherent usage/UI state.
  #   4. Audit every no-fallback inventory branch with typed failure, unchanged committed state before promotion, blocked inference, zero alternate requests and no false completion. After-promotion notification failure retains/reconciles the new commit.
  #   5. Human HITL decision: durable DAG commits and lossless structured replay are both first-milestone requirements; runtime-only fixes cannot be declared complete.
  #
  # EXAMPLES:
  #   1. Given repeated DAG cycles and post-snapshot tool work, when the session commits, cold-resumes and sends again, then DAG/file awareness, suffix and full originals survive.
  #   2. Given the missing-handler, bad-candidate, timeout, stale-generation, write-failure, replay-gap, WS-upgrade and delta-mismatch matrix, when exercised, then every failure terminates boundedly without a fallback or false successful completion.
  #   3. Given cancellation during working-view preparation or notification loss after promotion, when cold resume runs, then it restores the correct authoritative generation without destroyed suffix, rollback or duplicate promotion.
  #
  # ========================================
  Background: User Story
    As a agent operator
    I want to resume committed DAG sessions and send with the same intended lineage
    So that compaction is proven across process boundaries without alternate-path success

  Scenario Outline: Repeated committed DAG cycles survive a fresh process and another physical request
    Given a live <provider> session with immutable originals and post-snapshot tool work
    When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    And the selected session sends a physical provider request
    And a fresh process restores the session and sends another physical provider request
    Then both requests carry the intended committed DAG and complete ordered tool suffix
    And no covered original is silently replayed in either physical request
    And each candidate was submitted by the live stream and applied through the runtime commit owner
    And the physical adapter is the selected provider rather than a copied generic request
    And original payload retrieval returns the retained deep sentinel without another commit
    And context events and durable generation agree
    And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage

    Examples:
      | provider  |
      | anthropic |
      | openai    |
      | gemini    |
      | codex     |
      | copilot   |
      | zai       |
      | zai-plan  |
      | custom    |
      | stub      |

  Scenario: Cancellation before promotion preserves the original generation on cold resume
    Given a committed session with an uncovered tool suffix
    When candidate preparation is cancelled before durable promotion
    And a fresh process restores the session
    Then the original generation and complete suffix survive without a completion notice

  Scenario: Notification loss after promotion reconciles the new generation without another commit
    Given durable promotion succeeds but completion notification fails
    When a fresh process restores and explicitly reconciles the session
    Then the new generation and complete suffix survive without rollback or duplicate promotion
    And ordinary inference makes no physical request before exact identity reconciliation
    And the header refresh does not claim that the lost notification was delivered

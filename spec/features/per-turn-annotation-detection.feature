@wip
@CMPCT-011
Feature: Per-Turn Structural Annotation Detection

  """
  annotation_detector module in codelet/core/src/compaction/annotation_detector.rs provides
  zero-cost inline detection of structural annotations from per-turn tool call metadata.
  Uses StructuralAnnotation from CMPCT-007. Called after each completed turn in stream_loop.rs.
  Annotations piggyback on existing data flow — no LLM calls, no external processes.
  """

  Background: User Story
    As an AI agent
    I want per-turn structural annotations detected from my tool call metadata
    So that important turns (milestones, error resolutions, file modifications) are marked for efficient DAG construction during compaction

  @annotation-detection
  Scenario: Detect FspecMilestone annotation from fspec tool call
    Given a completed turn containing a Fspec tool call with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    When the per-turn annotation detector inspects the turn
    Then a FspecMilestone annotation should be created with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    And the annotation should be attached to the persisted turn metadata

  @annotation-detection
  Scenario: Detect ErrorResolution annotation from error-to-success transition
    Given a previous turn where Bash tool failed with exit code 1
    And a current turn where Edit modifies "src/main.rs" and Bash succeeds with exit code 0
    When the per-turn annotation detector inspects the current turn
    Then an ErrorResolution annotation should be created with failed_tool "Bash" and resolved_file "src/main.rs"
    And the annotation should be attached to the persisted turn metadata

  @annotation-detection
  Scenario: Detect FileModification annotation from Write tool call
    Given a completed turn containing a Write tool call creating "src/auth/handler.rs"
    When the per-turn annotation detector inspects the turn
    Then a FileModification annotation should be created with path "src/auth/handler.rs" and operation Created
    And the annotation should be attached to the persisted turn metadata

  @annotation-detection
  Scenario: Annotation detector produces no false positives for non-matching turns
    Given a completed turn containing only Read and Grep tool calls with no errors
    When the per-turn annotation detector inspects the turn
    Then no FspecMilestone annotations should be created
    And no ErrorResolution annotations should be created

  @annotation-detection
  Scenario: Annotations are zero-cost inline detection
    Given a completed turn with tool call metadata
    When the per-turn annotation detector runs
    Then detection should use only pattern matching on tool call metadata
    And no LLM calls should be made for annotation detection
    And no external processes should be spawned for annotation detection

@CMPCT-007
Feature: Structural Annotation Data Model
  """
  Both enums live in codelet/core/src/compaction/model.rs alongside existing TokenTracker, ConversationTurn, ToolCall, ToolResult
  Consumer: CMPCT-011 (per-turn annotation detector) will attach StructuralAnnotation to persisted messages. Agent sees annotations via SessionSearch metadata during DAG construction.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. StructuralAnnotation enum must derive Debug, Clone, Serialize, Deserialize
  #   2. StructuralAnnotation must have exactly 3 variants: FspecMilestone, ErrorResolution, FileModification
  #   3. FileOp enum must have exactly 3 variants: Created, Modified, Deleted and derive Debug, Clone, Serialize, Deserialize
  #   4. PreservationContext and BuildStatus must be marked #[deprecated] with a note pointing to StructuralAnnotation
  #   5. TokenTracker must remain completely unchanged
  #   6. StructuralAnnotation and FileOp must be re-exported from codelet/core/src/compaction/mod.rs
  #   7. StructuralAnnotation must round-trip through serde JSON serialization/deserialization
  #   8. All existing code that uses PreservationContext and BuildStatus must still compile (deprecated items remain usable with warnings)
  #
  # EXAMPLES:
  #   1. FspecMilestone {command: "update-work-unit-status", args: ["AUTH-001", "implementing"]} serializes to JSON and deserializes back to identical value
  #   2. ErrorResolution {failed_tool: "Bash", resolved_file: "src/main.rs"} round-trips through serde
  #   3. FileModification {path: "src/auth.rs", operation: FileOp::Created} round-trips through serde
  #   4. Using #[deprecated] PreservationContext still compiles but produces a deprecation warning
  #   5. Using #[deprecated] BuildStatus still compiles but produces a deprecation warning
  #   6. TokenTracker::new() and all its methods work identically before and after the change
  #   7. codelet_core::compaction::StructuralAnnotation and codelet_core::compaction::FileOp are accessible via mod.rs re-exports
  #
  # ========================================
  Background: User Story
    As a compaction system
    I want to annotate conversation turns with structural metadata
    So that the agent can identify important turns during DAG construction without reading full content

  @serde
  Scenario: FspecMilestone annotation serializes and deserializes
    Given a StructuralAnnotation::FspecMilestone with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    When I serialize it to JSON and deserialize back
    Then the deserialized value should be identical to the original

  @serde
  Scenario: ErrorResolution annotation round-trips through serde
    Given a StructuralAnnotation::ErrorResolution with failed_tool "Bash" and resolved_file "src/main.rs"
    When I serialize it to JSON and deserialize back
    Then the deserialized value should be identical to the original

  @serde
  Scenario: FileModification annotation round-trips through serde
    Given a StructuralAnnotation::FileModification with path "src/auth.rs" and operation FileOp::Created
    When I serialize it to JSON and deserialize back
    Then the deserialized value should be identical to the original

  @serde
  Scenario: All FileOp variants round-trip through serde
    Given FileOp variants Created, Modified, and Deleted
    When I serialize each variant to JSON and deserialize back
    Then each deserialized variant should be identical to its original

  @deprecation
  Scenario: Deprecated PreservationContext still compiles
    Given existing code that uses PreservationContext
    When the code is compiled
    Then it should compile successfully with deprecation warnings
    And the deprecation note should reference StructuralAnnotation

  @deprecation
  Scenario: Deprecated BuildStatus still compiles
    Given existing code that uses BuildStatus
    When the code is compiled
    Then it should compile successfully with deprecation warnings
    And the deprecation note should reference StructuralAnnotation

  @backwards-compatibility
  Scenario: TokenTracker remains unchanged
    Given the existing TokenTracker struct
    When the structural annotation changes are applied
    Then TokenTracker::new() should work identically
    And all TokenTracker methods should produce the same results

  @exports
  Scenario: StructuralAnnotation and FileOp are accessible via module re-exports
    Given the codelet_core::compaction module
    When I import StructuralAnnotation and FileOp from the module
    Then both types should be accessible without specifying internal module paths

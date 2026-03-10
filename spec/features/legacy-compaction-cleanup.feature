@compaction
@wip
@CMPCT-013
Feature: Legacy Batch LLM Compaction Cleanup

  """
  Remove the old batch LLM-based compaction infrastructure after the in-view DAG
  construction flow (CMPCT-011/012) is validated and stable. Pure deletion, zero
  new functionality. Removes ~1700+ lines of dead code including flaky heuristics
  (detect_build_status matching "pass"/"fail" substrings, extract_goal_from_message
  matching action verbs), expensive batch LLM calls, and the entire anchor/selector/
  compactor/metrics pipeline that has been fully replaced by agent-driven SessionSearch
  retrieval and inject_summary.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Delete selector.rs entirely — TurnSelector, TurnSelection, TurnInfo replaced by SessionSearch
  #   2. Delete deprecated.rs entirely — PreservationContext, BuildStatus replaced by StructuralAnnotation
  #   3. Delete llm_anchor_detection.test.rs — tests for deleted LLM functions
  #   4. Delete anchor.rs entirely — AnchorDetector, AnchorPoint, AnchorType all dead code
  #   5. Delete compactor.rs entirely — ContextCompactor, CompactionStrategy, LLM summary generation
  #   6. Delete metrics.rs entirely — CompactionMetrics/CompactionResult only used by deleted code
  #   7. Remove execute_compaction_legacy() from interactive_helpers.rs
  #   8. Remove compact_messages() from Session — dead code using ContextCompactor
  #   9. Clean NAPI anchor infrastructure — persist_anchor_point, session_get_anchor_points,
  #      session_restore_anchor_points, NapiAnchorType, NapiAnchorPoint, BackgroundSession.anchor_points
  #   10. Keep intact: model.rs, trimmer.rs, trimmer_base64.rs, trimmer_metadata.rs, annotation_detector.rs,
  #       PersistedAnchorPoint in persistence/types.rs (string-based, for reading old manifests)
  #   11. cargo build with zero warnings, full test suite passes, zero dangling references
  #
  # ========================================

  Background: User Story
    Given the in-view DAG construction compaction flow from CMPCT-011 and CMPCT-012 is validated and stable
    And the legacy batch LLM compaction code has zero live callers in production

  @core @deletion
  Scenario: Delete legacy compaction source files from core module
    Given the compaction module contains legacy files: anchor.rs, compactor.rs, selector.rs, deprecated.rs, metrics.rs
    And these files contain LLM-based detection, batch processing, retry logic, flaky heuristics, and turn selection
    When the legacy files are deleted
    Then anchor.rs no longer exists in the compaction module
    And compactor.rs no longer exists in the compaction module
    And selector.rs no longer exists in the compaction module
    And deprecated.rs no longer exists in the compaction module
    And metrics.rs no longer exists in the compaction module

  @core @deletion
  Scenario: Delete legacy LLM anchor detection test file
    Given the compaction __tests__ directory contains llm_anchor_detection.test.rs
    When the legacy test file is deleted
    Then llm_anchor_detection.test.rs no longer exists in the __tests__ directory

  @core @module-registry
  Scenario: Update mod.rs to remove deleted module declarations and re-exports
    Given mod.rs declares modules: selector, deprecated, compactor, anchor, metrics
    And mod.rs has a test module for llm_anchor_detection_tests
    And mod.rs re-exports TurnSelector, TurnSelection, TurnInfo, ContextCompactor, CompactionStrategy, CompactionMetrics, CompactionResult, AnchorDetector, AnchorPoint, AnchorType
    When deleted module declarations and re-exports are removed from mod.rs
    Then mod.rs does not declare mod selector
    And mod.rs does not declare mod deprecated
    And mod.rs does not declare mod compactor
    And mod.rs does not declare mod anchor
    And mod.rs does not declare mod metrics
    And mod.rs does not include llm_anchor_detection_tests
    And mod.rs does not re-export any deleted types
    And mod.rs still declares mod trimmer, mod trimmer_base64, mod trimmer_metadata, mod annotation_detector, mod model

  @core @preservation
  Scenario: Preserved modules remain intact and functional
    Given model.rs contains TokenTracker, ConversationTurn, ToolCall, ToolResult, StructuralAnnotation, FileOp
    And trimmer.rs, trimmer_base64.rs, trimmer_metadata.rs contain Layer 0 structurally lossless trimming
    And annotation_detector.rs contains per-turn structural annotation detection
    When the legacy code is removed
    Then model.rs types are unchanged and accessible via module re-exports
    And trimmer functionality is unchanged
    And annotation_detector functionality is unchanged

  @cli @deletion
  Scenario: Remove execute_compaction_legacy from interactive_helpers.rs
    Given interactive_helpers.rs contains execute_compaction_legacy() which uses ContextCompactor
    And CMPCT-012 replaced all call sites with execute_compaction()
    When execute_compaction_legacy is removed
    Then interactive_helpers.rs does not contain execute_compaction_legacy
    And interactive_helpers.rs does not import ContextCompactor or CompactionMetrics
    And the new execute_compaction function remains intact

  @cli @deletion
  Scenario: Remove compact_messages from Session
    Given Session in codelet/cli/src/session/mod.rs has a compact_messages() method
    And compact_messages() uses ContextCompactor and has zero production callers
    When compact_messages is removed from Session
    Then Session does not have a compact_messages method
    And Session does not import ContextCompactor

  @napi @deletion
  Scenario: Remove dead anchor infrastructure from NAPI layer
    Given session_manager.rs contains persist_anchor_point() with zero callers
    And session_manager.rs contains session_get_anchor_points() and session_restore_anchor_points()
    And BackgroundSession has an anchor_points Mutex field
    And types.rs contains NapiAnchorType enum and NapiAnchorPoint struct
    When the dead NAPI anchor infrastructure is removed
    Then persist_anchor_point no longer exists in session_manager.rs
    And session_get_anchor_points no longer exists
    And session_restore_anchor_points no longer exists
    And BackgroundSession no longer has an anchor_points field
    And NapiAnchorType and NapiAnchorPoint are removed from types.rs

  @napi @preservation
  Scenario: PersistedAnchorPoint remains for backward compatibility
    Given PersistedAnchorPoint in persistence/types.rs uses String for anchor_type
    And existing session manifests on disk may contain persisted anchor points
    When the legacy code is removed
    Then PersistedAnchorPoint still exists in persistence/types.rs
    And existing persisted session manifests can still be deserialized

  @test @deletion
  Scenario: Delete downstream test files that exclusively test deleted code
    Given test files exist that exclusively test deleted legacy code
    When the legacy test files are deleted
    Then codelet/core/tests/llm_anchor_integration_test.rs no longer exists
    And codelet/core/tests/retry_llm_summary_test.rs no longer exists
    And codelet/core/tests/compaction_anchor_detection_test.rs no longer exists
    And codelet/cli/tests/context_compaction_fix_test.rs no longer exists
    And codelet/cli/tests/manual_compaction_command_test.rs no longer exists
    And codelet/napi/tests/compaction_to_anchor_flow_test.rs no longer exists
    And codelet/napi/tests/anchor_persistence_test.rs no longer exists

  @test @update
  Scenario: Update test files that reference deleted types
    Given structural_annotation.test.rs contains tests for deprecated PreservationContext and BuildStatus
    And context_compaction_test.rs references TurnSelector and ContextCompactor
    And system_reminder_infrastructure_test.rs calls compact_messages()
    When tests referencing deleted types are updated or removed
    Then structural_annotation.test.rs no longer references PreservationContext or BuildStatus
    And no remaining test file imports deleted types
    And demo_compaction.rs example is removed or updated

  @build @validation
  Scenario: cargo build succeeds with zero warnings from deleted modules
    Given all legacy compaction source files and their dependents have been cleaned up
    When cargo build is run
    Then the build succeeds with exit code 0
    And there are no warnings about unused imports from deleted modules
    And there are no warnings about dead code in the compaction module

  @build @validation
  Scenario: Full project test suite passes with no regressions
    Given all legacy code has been removed and remaining tests updated
    When the full project test suite is run
    Then all tests pass
    And no test failures are caused by missing deleted types or functions

  @validation @exhaustive
  Scenario: Zero references to deleted types remain in source files
    Given the legacy cleanup is complete
    When the codebase is searched for references to deleted identifiers
    Then grep for "LlmAnchorResponse" in Rust source files returns zero matches
    And grep for "detect_batch" in Rust source files returns zero matches
    And grep for "TurnSelector" in Rust source files returns zero matches
    And grep for "PreservationContext" in Rust source files returns zero matches
    And grep for "BuildStatus" in Rust source files returns zero matches
    And grep for "ContextCompactor" in Rust source files returns zero matches
    And grep for "RETRY_DELAYS_MS" in Rust source files returns zero matches
    And grep for "FALLBACK_SUMMARY" in Rust source files returns zero matches
    And grep for "execute_compaction_legacy" in Rust source files returns zero matches
    And grep for "AnchorDetector" in Rust source files returns zero matches
    And grep for "CompactionMetrics" in Rust source files returns zero matches
    And grep for "compact_messages" in Rust source files returns zero matches

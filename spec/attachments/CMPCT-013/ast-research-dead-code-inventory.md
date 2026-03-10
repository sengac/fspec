# CMPCT-013 Dead Code Inventory — AST Research

## Dead Types (to delete)

### codelet/core/src/compaction/

| File | Type | Kind | Line |
|------|------|------|------|
| compactor.rs | `ContextCompactor` | struct | 54 |
| compactor.rs | `CompactionStrategy` | enum | (within file) |
| selector.rs | `TurnSelector` | struct | 15 |
| selector.rs | `TurnSelection` | struct | (within file) |
| selector.rs | `TurnInfo` | struct | (within file) |
| anchor.rs | `AnchorDetector` | struct | 159 |
| anchor.rs | `AnchorPoint` | struct | (within file) |
| anchor.rs | `AnchorType` | enum | 92 |
| anchor.rs | `LlmAnchorResponse` | struct | (within file) |
| deprecated.rs | `PreservationContext` | struct | 55 |
| deprecated.rs | `BuildStatus` | enum | 21 |
| metrics.rs | `CompactionMetrics` | struct | 16 |
| metrics.rs | `CompactionResult` | struct | (within file) |

### codelet/napi/src/types.rs

| Type | Kind | Line |
|------|------|------|
| `NapiAnchorType` | enum | 22 |
| `NapiAnchorPoint` | struct | 32 |
| `NapiAnchorToolCall` | struct | 58 |

## Dead Functions (to delete)

### codelet/napi/src/session_manager.rs

- `persist_anchor_point()` — zero callers
- `session_get_anchor_points()` — dead NAPI function
- `session_restore_anchor_points()` — dead NAPI function

### codelet/cli/src/interactive_helpers.rs

- `execute_compaction_legacy()` — zero callers (CMPCT-012 replaced all)

### codelet/cli/src/session/mod.rs

- `compact_messages()` on Session — zero production callers

### codelet/napi/src/session_manager.rs (BackgroundSession)

- `anchor_points: Mutex<Vec<AnchorPoint>>` field — only used by dead functions

## Files to Delete Entirely

1. `codelet/core/src/compaction/anchor.rs` (486 lines)
2. `codelet/core/src/compaction/compactor.rs` (445 lines)
3. `codelet/core/src/compaction/selector.rs` (123 lines)
4. `codelet/core/src/compaction/deprecated.rs` (230 lines)
5. `codelet/core/src/compaction/metrics.rs` (53 lines)
6. `codelet/core/src/compaction/__tests__/llm_anchor_detection.test.rs` (376 lines)
7. `codelet/core/tests/llm_anchor_integration_test.rs`
8. `codelet/core/tests/retry_llm_summary_test.rs`
9. `codelet/core/tests/compaction_anchor_detection_test.rs`
10. `codelet/core/tests/context_compaction_test.rs`
11. `codelet/cli/tests/context_compaction_fix_test.rs`
12. `codelet/cli/tests/manual_compaction_command_test.rs`
13. `codelet/napi/tests/compaction_to_anchor_flow_test.rs`
14. `codelet/napi/tests/anchor_persistence_test.rs`
15. `codelet/examples/demo_compaction.rs`

## Preserved Types (DO NOT delete)

- `model.rs`: TokenTracker, ConversationTurn, ToolCall, ToolResult, StructuralAnnotation, FileOp
- `trimmer.rs`, `trimmer_base64.rs`, `trimmer_metadata.rs`
- `annotation_detector.rs`
- `persistence/types.rs`: PersistedAnchorPoint (string-based, backward compat)

# AST Research: model.rs structs/enums and mod.rs re-exports

## Purpose
Identify all existing types in `codelet/core/src/compaction/model.rs` and current re-exports
in `mod.rs` to plan where to add `StructuralAnnotation` and `FileOp`, and what to deprecate.

## Existing Enums in model.rs

| Line | Type | Name |
|------|------|------|
| 278 | enum | `BuildStatus` — deprecation target |

## Existing Structs in model.rs

| Line | Type | Name |
|------|------|------|
| 57 | struct | `TokenTracker` — keep unchanged |
| 213 | struct | `ConversationTurn` — keep unchanged |
| 232 | struct | `ToolCall` — keep unchanged |
| 261 | struct | `ToolResult` — keep unchanged |
| 311 | struct | `PreservationContext` — deprecation target |

## Current Re-exports in mod.rs

| Line | Export |
|------|--------|
| 31 | `model::{ConversationTurn, TokenTracker, ToolCall, ToolResult}` |
| 34 | `anchor::{AnchorDetector, AnchorPoint, AnchorType}` |
| 37 | `metrics::{CompactionMetrics, CompactionResult}` |
| 40 | `compactor::{CompactionStrategy, ContextCompactor}` |
| 43 | `selector::{TurnInfo, TurnSelection, TurnSelector}` |
| 46 | `trimmer::Trimmer` |

**Note:** `BuildStatus` and `PreservationContext` are NOT currently re-exported from mod.rs.
They are used internally by `compactor.rs` via `use super::model::{ConversationTurn, PreservationContext}`.

## Callers of PreservationContext (grep results)

- `compactor.rs:11` — `use super::model::{ConversationTurn, PreservationContext};`
- `compactor.rs:284` — `let preservation_context = PreservationContext::extract_from_turns(kept_turns);`
- `compactor.rs:366` — `preservation_context: &PreservationContext,`

## Plan

1. Add `StructuralAnnotation` enum and `FileOp` enum to model.rs (after line ~268, before BuildStatus)
2. Add `#[deprecated]` attribute to `BuildStatus` and `PreservationContext`
3. Add re-exports for `StructuralAnnotation` and `FileOp` to mod.rs line 31
4. `compactor.rs` callers will get deprecation warnings but still compile

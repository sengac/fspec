# AST Research: Compaction Module

## anchor.rs (codelet/core/src/compaction/anchor.rs)

### Functions
| Function | Line | Description |
|----------|------|-------------|
| `weight()` | 38 | Returns weight for anchor type (ErrorResolution=0.9, TaskCompletion=0.8, FeatureMilestone=0.75, UserCheckpoint=0.7) |
| `new()` | 78 | Creates new AnchorDetector with confidence threshold |
| `detect()` | 87 | Main anchor detection - currently only detects Edit/Write + test success patterns |

### Current Detection Patterns
- **ErrorResolution**: previous_error + file_modification + test_success (confidence=0.95)
- **TaskCompletion**: !previous_error + file_modification + test_success (confidence=0.92)

### Missing Patterns (to implement)
- Web search + synthesis detection
- Bash milestone detection (successfully, installed, built, compiled)
- Information synthesis detection

---

## model.rs (codelet/core/src/compaction/model.rs)

### Functions
| Function | Line | Description |
|----------|------|-------------|
| `effective_tokens()` | 34 | Calculate effective tokens with cache discount |
| `total_tokens()` | 41 | Sum input + output tokens |
| `file_path()` | 86 | Extract file_path from tool call parameters |
| `filename()` | 95 | Extract filename from file_path |

### Missing Structures (to implement)
- `PreservationContext` struct (active_files, current_goals, error_states, build_status, last_user_intent)
- `ConversationFlow` struct (turns, anchor_points, total_tokens, preservation_context, migration_strategy, synthetic_anchor)
- `BuildStatus` enum (Passing, Failing, Unknown)
- `MigrationStrategy` enum (AnchorPointOnly, LegacyFallback)

---

## compactor.rs (codelet/core/src/compaction/compactor.rs)

### Functions
| Function | Line | Description |
|----------|------|-------------|
| `new()` | 50 | Create compactor with default config (threshold=0.9, compression=0.6, strategy=AnchorBased) |
| `with_confidence_threshold()` | 59 | Builder for confidence threshold |
| `with_compression_threshold()` | 65 | Builder for compression threshold |
| `with_strategy()` | 71 | Builder for strategy |
| `compact()` | 88 | Main compaction method - detects anchors, selects turns, generates summary |
| `generate_weighted_summary()` | 187 | **PROBLEM**: Uses hardcoded template instead of PreservationContext |
| `turn_to_outcome()` | 211 | Transform turn to outcome description |
| `default()` | 253 | Default implementation |

### Current Issues (line 203-204)
```rust
let context_summary =
    "Active files: [from conversation]\nGoals: Continue development\nBuild: unknown";
```
This is hardcoded template text - needs to use actual PreservationContext values.

---

## Implementation Plan

1. **model.rs**: Add PreservationContext, ConversationFlow, BuildStatus, MigrationStrategy structs
2. **anchor.rs**: Add detect_successful_search(), detect_bash_milestone() methods
3. **compactor.rs**: Update generate_weighted_summary() to accept and use PreservationContext
4. **Add synthetic anchor creation**: In ConversationFlow::from_turns() when no natural anchors found

# Anchor-Based Compaction System Implementation Specification

## CTX-001: Complete Anchor-Based Context Compaction

This document details the implementation requirements for completing the anchor-based compaction system in fspec's Rust codebase to match the TypeScript reference implementation.

---

## 1. Executive Summary

The current Rust implementation of anchor-based compaction is **incomplete**. While the basic data structures exist, critical features that make the TypeScript implementation effective are missing or hardcoded. This results in:

- **No anchors detected** for non-coding conversations (web search, bash, general Q&A)
- **Useless summaries** with hardcoded template text instead of actual context
- **No synthetic fallback** when no natural anchors exist
- **Lost context** because PreservationContext is not tracked

---

## 2. Current State Analysis

### 2.1 What Exists in Rust

#### Data Structures (codelet/core/src/compaction/)

```rust
// model.rs - Basic structures exist
pub struct ConversationTurn {
    pub user_message: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub assistant_response: String,
    pub tokens: u64,
    pub timestamp: SystemTime,
    pub previous_error: Option<bool>,
}

pub struct ToolCall {
    pub tool: String,
    pub id: String,
    pub parameters: serde_json::Value,
}

pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

// anchor.rs - Anchor types exist
pub enum AnchorType {
    ErrorResolution,    // weight 0.9
    TaskCompletion,     // weight 0.8
    UserCheckpoint,     // weight 0.7
    FeatureMilestone,   // weight 0.75
}

pub struct AnchorPoint {
    pub turn_index: usize,
    pub anchor_type: AnchorType,
    pub weight: f64,
    pub confidence: f64,
    pub description: String,
    pub timestamp: SystemTime,
}
```

#### Anchor Detection (codelet/core/src/compaction/anchor.rs)

```rust
// PROBLEM: Only detects anchors for Edit/Write + test success patterns
pub fn detect(&self, turn: &ConversationTurn, turn_index: usize) -> Result<Option<AnchorPoint>> {
    let has_test_success = turn.tool_results.iter().any(|result| {
        result.success
            && result.output.to_lowercase().contains("test")
            && (result.output.contains("pass") || result.output.contains("success"))
    });

    let has_file_modification = turn.tool_calls.iter()
        .any(|call| call.tool == "Edit" || call.tool == "Write");

    // Only these two patterns create anchors:
    // 1. Error resolution: previous_error + file_mod + test_success
    // 2. Task completion: !previous_error + file_mod + test_success

    // MISSING: Web search, bash, read, and other tool patterns
}
```

#### Summary Generation (codelet/core/src/compaction/compactor.rs)

```rust
// PROBLEM: Hardcoded template instead of dynamic context
fn generate_weighted_summary(&self, turns: &[&ConversationTurn], anchors: &[AnchorPoint]) -> String {
    let context_summary =
        "Active files: [from conversation]\nGoals: Continue development\nBuild: unknown";
    // ^^^ HARDCODED - should use actual PreservationContext

    format!("{}\n\nKey outcomes:\n{}", context_summary, outcomes.join("\n"))
}
```

### 2.2 What's Missing in Rust

| Feature | TypeScript | Rust | Impact |
|---------|------------|------|--------|
| **PreservationContext** | Tracks activeFiles, currentGoals, errorStates, buildStatus, lastUserIntent | Not implemented (hardcoded string) | Summary contains no useful context |
| **Synthetic Anchor** | Creates user-checkpoint when no anchors found | Not implemented | Falls back to simple truncation |
| **ConversationFlow** | Rich structure with migrationStrategy, syntheticAnchor | Not implemented | No unified flow management |
| **Broader Anchor Detection** | Could detect web_search, bash success, etc. | Only Edit/Write + test | Most conversations have no anchors |
| **Batch Processing** | Processes large histories in batches | Not implemented | Performance issues possible |
| **Dynamic Summary** | Uses actual PreservationContext values | Hardcoded template | Summary is useless |

---

## 3. Target State (Match TypeScript)

### 3.1 New Data Structures Required

```rust
// New: PreservationContext
pub struct PreservationContext {
    /// Files actively being worked on (from Edit/Write/Read tools)
    pub active_files: Vec<String>,
    /// Current user goals extracted from conversation
    pub current_goals: Vec<String>,
    /// Active error states (build errors, test failures)
    pub error_states: Vec<String>,
    /// Build status: passing, failing, or unknown
    pub build_status: BuildStatus,
    /// Last user intent extracted from most recent user message
    pub last_user_intent: String,
}

pub enum BuildStatus {
    Passing,
    Failing,
    Unknown,
}

// New: ConversationFlow
pub struct ConversationFlow {
    pub turns: Vec<ConversationTurn>,
    pub anchor_points: Vec<AnchorPoint>,
    pub total_tokens: u64,
    pub preservation_context: PreservationContext,
    pub migration_strategy: Option<MigrationStrategy>,
    pub synthetic_anchor: Option<AnchorPoint>,
}

pub enum MigrationStrategy {
    AnchorPointOnly,
    LegacyFallback,
}
```

### 3.2 Enhanced Anchor Detection

```rust
impl AnchorDetector {
    pub fn detect(&self, turn: &ConversationTurn, turn_index: usize) -> Result<Option<AnchorPoint>> {
        // Existing patterns (keep)
        if let Some(anchor) = self.detect_error_resolution(turn, turn_index)? {
            return Ok(Some(anchor));
        }
        if let Some(anchor) = self.detect_task_completion(turn, turn_index)? {
            return Ok(Some(anchor));
        }

        // NEW: Additional patterns for non-coding conversations
        if let Some(anchor) = self.detect_successful_search(turn, turn_index)? {
            return Ok(Some(anchor));
        }
        if let Some(anchor) = self.detect_bash_milestone(turn, turn_index)? {
            return Ok(Some(anchor));
        }
        if let Some(anchor) = self.detect_information_synthesis(turn, turn_index)? {
            return Ok(Some(anchor));
        }

        Ok(None)
    }

    /// Detect successful web search that answered user's question
    fn detect_successful_search(&self, turn: &ConversationTurn, turn_index: usize) -> Result<Option<AnchorPoint>> {
        let has_web_search = turn.tool_calls.iter()
            .any(|call| call.tool == "web_search" || call.tool == "WebSearch");

        let search_succeeded = turn.tool_results.iter()
            .any(|r| r.success && r.output.len() > 100); // Non-trivial results

        // Look for synthesis indicators in assistant response
        let has_synthesis = turn.assistant_response.contains("Based on")
            || turn.assistant_response.contains("According to")
            || turn.assistant_response.contains("The search results show");

        if has_web_search && search_succeeded && has_synthesis {
            return Ok(Some(AnchorPoint {
                turn_index,
                anchor_type: AnchorType::TaskCompletion,
                weight: 0.75, // Slightly lower than code completion
                confidence: 0.85,
                description: "Web search completed with synthesized results".to_string(),
                timestamp: turn.timestamp,
            }));
        }

        Ok(None)
    }

    /// Detect bash command milestones (successful installs, builds, etc.)
    fn detect_bash_milestone(&self, turn: &ConversationTurn, turn_index: usize) -> Result<Option<AnchorPoint>> {
        let has_bash = turn.tool_calls.iter()
            .any(|call| call.tool == "bash" || call.tool == "Bash");

        let bash_success = turn.tool_results.iter()
            .any(|r| r.success);

        // Look for milestone indicators
        let is_milestone = turn.tool_results.iter().any(|r| {
            let output_lower = r.output.to_lowercase();
            output_lower.contains("successfully")
                || output_lower.contains("installed")
                || output_lower.contains("built")
                || output_lower.contains("compiled")
                || output_lower.contains("completed")
        });

        if has_bash && bash_success && is_milestone {
            return Ok(Some(AnchorPoint {
                turn_index,
                anchor_type: AnchorType::TaskCompletion,
                weight: 0.8,
                confidence: 0.88,
                description: "Bash command milestone completed".to_string(),
                timestamp: turn.timestamp,
            }));
        }

        Ok(None)
    }
}
```

### 3.3 Synthetic Anchor Creation

```rust
impl ConversationFlow {
    /// Create from turns, detecting anchors and creating synthetic if needed
    pub fn from_turns(turns: Vec<ConversationTurn>) -> Self {
        let detector = AnchorDetector::new(0.9);
        let mut anchor_points = detector.detect_historical_anchors(&turns);

        // CRITICAL: Create synthetic anchor if none found
        let synthetic_anchor = if anchor_points.is_empty() && !turns.is_empty() {
            let synthetic = AnchorPoint {
                turn_index: turns.len() - 1,
                anchor_type: AnchorType::UserCheckpoint,
                weight: 0.7,
                description: "Synthetic anchor at conversation end".to_string(),
                timestamp: SystemTime::now(),
                confidence: 0.8,
            };
            anchor_points.push(synthetic.clone());
            Some(synthetic)
        } else {
            None
        };

        let total_tokens = turns.iter().map(|t| t.tokens).sum();
        let preservation_context = PreservationContext::extract_from_turns(&turns);

        Self {
            turns,
            anchor_points,
            total_tokens,
            preservation_context,
            migration_strategy: Some(MigrationStrategy::AnchorPointOnly),
            synthetic_anchor,
        }
    }
}
```

### 3.4 Dynamic PreservationContext Extraction

```rust
impl PreservationContext {
    /// Extract context from conversation turns
    pub fn extract_from_turns(turns: &[ConversationTurn]) -> Self {
        let mut active_files = Vec::new();
        let mut error_states = Vec::new();
        let mut build_status = BuildStatus::Unknown;

        for turn in turns {
            // Extract active files from tool calls
            for call in &turn.tool_calls {
                if let Some(file_path) = call.filename() {
                    if !active_files.contains(&file_path) {
                        active_files.push(file_path);
                    }
                }
            }

            // Extract error states from failed tool results
            for result in &turn.tool_results {
                if !result.success {
                    // Extract first line of error as state
                    if let Some(first_line) = result.output.lines().next() {
                        let error_summary = first_line.chars().take(100).collect::<String>();
                        error_states.push(error_summary);
                    }
                }
            }

            // Update build status from test results
            for result in &turn.tool_results {
                let output_lower = result.output.to_lowercase();
                if output_lower.contains("test") {
                    if output_lower.contains("pass") || output_lower.contains("success") {
                        build_status = BuildStatus::Passing;
                    } else if output_lower.contains("fail") || output_lower.contains("error") {
                        build_status = BuildStatus::Failing;
                    }
                }
            }
        }

        // Extract last user intent from most recent turn
        let last_user_intent = turns.last()
            .map(|t| t.user_message.chars().take(200).collect::<String>())
            .unwrap_or_else(|| "Continue conversation".to_string());

        // Extract goals from conversation (simplified - could use NLP)
        let current_goals = Self::extract_goals_from_turns(turns);

        Self {
            active_files,
            current_goals,
            error_states,
            build_status,
            last_user_intent,
        }
    }

    fn extract_goals_from_turns(turns: &[ConversationTurn]) -> Vec<String> {
        let mut goals = Vec::new();

        for turn in turns {
            let msg_lower = turn.user_message.to_lowercase();

            // Extract goal indicators
            if msg_lower.contains("help me") || msg_lower.contains("i want to")
                || msg_lower.contains("i need to") || msg_lower.contains("please") {
                // Take first sentence as goal
                if let Some(first_sentence) = turn.user_message.split('.').next() {
                    let goal = first_sentence.chars().take(100).collect::<String>();
                    if !goals.contains(&goal) {
                        goals.push(goal);
                    }
                }
            }
        }

        // Keep last 3 goals (most recent are most relevant)
        if goals.len() > 3 {
            goals = goals.into_iter().rev().take(3).collect();
            goals.reverse();
        }

        goals
    }

    /// Format context for summary
    pub fn format_for_summary(&self) -> String {
        let files = if self.active_files.is_empty() {
            "None".to_string()
        } else {
            self.active_files.join(", ")
        };

        let goals = if self.current_goals.is_empty() {
            "Continue conversation".to_string()
        } else {
            self.current_goals.join("; ")
        };

        let build = match self.build_status {
            BuildStatus::Passing => "passing",
            BuildStatus::Failing => "failing",
            BuildStatus::Unknown => "unknown",
        };

        format!(
            "Active files: {}\nGoals: {}\nBuild: {}",
            files, goals, build
        )
    }
}
```

### 3.5 Updated Summary Generation

```rust
impl ContextCompactor {
    fn generate_weighted_summary(
        &self,
        turns: &[&ConversationTurn],
        anchors: &[AnchorPoint],
        context: &PreservationContext,  // NEW: Pass actual context
    ) -> String {
        let outcomes: Vec<String> = turns
            .iter()
            .map(|turn| self.turn_to_outcome(turn, anchors))
            .collect();

        // Use dynamic context instead of hardcoded string
        let context_summary = context.format_for_summary();

        format!("{}\n\nKey outcomes:\n{}", context_summary, outcomes.join("\n"))
    }

    fn turn_to_outcome(&self, turn: &ConversationTurn, anchors: &[AnchorPoint]) -> String {
        let is_anchor = anchors.iter().any(|a| a.timestamp == turn.timestamp);

        if is_anchor {
            // Anchor points get detailed preservation (full response, truncated)
            let response_preview = turn.assistant_response.chars().take(500).collect::<String>();
            return format!("[ANCHOR] {}", response_preview);
        }

        // Regular turns get compressed to outcomes
        let files = self.extract_files_from_turn(turn);
        let success = turn.tool_results.iter().any(|r| r.success);
        let success_marker = if success { "✓" } else { "✗" };

        let file_info = if !files.is_empty() {
            format!("Modified {}: ", files.join(", "))
        } else {
            String::new()
        };

        // Get first sentence of assistant response
        let first_sentence = turn.assistant_response
            .split('.')
            .next()
            .unwrap_or(&turn.assistant_response)
            .chars()
            .take(150)
            .collect::<String>();

        format!("{} {}{}", success_marker, file_info, first_sentence)
    }

    fn extract_files_from_turn(&self, turn: &ConversationTurn) -> Vec<String> {
        turn.tool_calls
            .iter()
            .filter(|call| call.tool == "Edit" || call.tool == "Write" || call.tool == "Read")
            .filter_map(|call| call.filename())
            .collect()
    }
}
```

---

## 4. Business Rules

### BR-001: Anchor Detection Confidence Threshold
- **Rule**: Anchors are only created when detection confidence >= 0.9 (90%)
- **Rationale**: High threshold prevents false positives from routine operations
- **Example**: A simple file read should NOT create an anchor; a successful test run SHOULD

### BR-002: Anchor Type Weights
| Type | Weight | Description |
|------|--------|-------------|
| ErrorResolution | 0.9 | Highest - error fixed and verified |
| TaskCompletion | 0.8 | High - task completed successfully |
| FeatureMilestone | 0.75 | Medium - feature checkpoint |
| UserCheckpoint | 0.7 | Lower - synthetic/explicit save |

### BR-003: Recent Turn Preservation
- **Rule**: Always preserve last 2-3 conversation turns regardless of anchors
- **Rationale**: Recent context is always relevant; compaction targets older turns
- **Example**: With 10 turns, compaction considers turns 0-7, always keeps 8-10

### BR-004: Synthetic Anchor Fallback
- **Rule**: When no natural anchors are detected, create a synthetic UserCheckpoint at conversation end
- **Rationale**: Ensures compaction always has a reference point
- **Example**: Web search conversation with no Edit/Write → synthetic anchor at turn N-1

### BR-005: Compression Ratio Warning
- **Rule**: Warn when compression ratio < 60%
- **Rationale**: Low compression indicates fresh conversation may be better
- **Example**: "Compression ratio 45% - consider starting fresh conversation"

### BR-006: PreservationContext Extraction
- **Rule**: Extract active files, goals, error states, and build status from conversation
- **Rationale**: Context makes summaries useful for continuation
- **Example**: "Active files: auth.rs, login.ts; Goals: Implement OAuth; Build: passing"

---

## 5. Concrete Examples

### Example 1: Coding Conversation (Anchor Detected)

**Input Conversation:**
```
Turn 0: User asks to fix a bug in auth.rs
Turn 1: Assistant reads auth.rs, identifies issue
Turn 2: User confirms the fix approach
Turn 3: Assistant edits auth.rs, runs tests → PASS ✓
Turn 4: User asks to also update documentation
Turn 5: Assistant updates README.md
```

**Anchor Detection:**
- Turn 3 creates TaskCompletion anchor (Edit + test pass)
- Confidence: 0.92, Weight: 0.8

**Compaction Result:**
- Turns 0-2: Summarized
- Turn 3+: Kept (anchor forward)
- Summary: "Active files: auth.rs, README.md\nGoals: Fix auth bug\nBuild: passing\n\nKey outcomes:\n✓ Modified auth.rs: Fixed authentication bug\n✓ Modified README.md: Updated documentation"

### Example 2: Web Search Conversation (No Natural Anchor)

**Input Conversation:**
```
Turn 0: User asks about Brisbane job market
Turn 1: Assistant searches web, synthesizes results
Turn 2: User asks for more details on salaries
Turn 3: Assistant searches again, provides salary data
```

**Anchor Detection:**
- With current code: NO anchors (no Edit/Write)
- With enhanced code: Turn 1 and 3 could be TaskCompletion (web search + synthesis)

**Without Synthetic Anchor (Current Bug):**
- Falls back to simple truncation
- Keeps last 3 turns, summarizes turn 0
- Summary is hardcoded useless text

**With Synthetic Anchor (Fixed):**
- Creates UserCheckpoint at turn 3
- Keeps turn 3 forward
- Summarizes turns 0-2
- Summary includes: "Goals: Research Brisbane job market; Research salary expectations"

### Example 3: Mixed Conversation

**Input Conversation:**
```
Turn 0: User asks to set up a new project
Turn 1: Assistant runs `npm init`, `npm install` → SUCCESS ✓
Turn 2: User asks to create initial files
Turn 3: Assistant creates index.ts, writes config
Turn 4: User asks to run tests
Turn 5: Assistant runs tests → PASS ✓
Turn 6: User asks about deployment options
Turn 7: Assistant searches web for deployment info
```

**Anchor Detection:**
- Turn 1: TaskCompletion (bash milestone - "installed")
- Turn 5: TaskCompletion (test pass)

**Compaction:**
- Most recent anchor is Turn 5
- Keeps turns 5-7
- Summarizes turns 0-4
- Summary includes project setup outcomes

---

## 6. Implementation Plan

### Phase 1: Data Structures
1. Add `PreservationContext` struct
2. Add `ConversationFlow` struct
3. Add `BuildStatus` enum
4. Add `MigrationStrategy` enum

### Phase 2: PreservationContext Extraction
1. Implement `PreservationContext::extract_from_turns()`
2. Add file extraction from tool calls
3. Add goal extraction from user messages
4. Add build status detection

### Phase 3: Enhanced Anchor Detection
1. Add `detect_successful_search()` method
2. Add `detect_bash_milestone()` method
3. Add `detect_information_synthesis()` method
4. Update `detect()` to use new patterns

### Phase 4: Synthetic Anchor Creation
1. Implement `ConversationFlow::from_turns()`
2. Add synthetic anchor logic when no anchors found
3. Add migration strategy tracking

### Phase 5: Dynamic Summary Generation
1. Update `generate_weighted_summary()` to accept PreservationContext
2. Remove hardcoded context string
3. Use `context.format_for_summary()`

### Phase 6: Integration
1. Update `execute_compaction()` in interactive_helpers.rs
2. Pass PreservationContext through compaction flow
3. Update NAPI bindings to expose new types

### Phase 7: Testing
1. Unit tests for each new pattern
2. Integration tests for compaction flow
3. Test synthetic anchor creation
4. Test PreservationContext extraction

---

## 7. Files to Modify

| File | Changes |
|------|---------|
| `codelet/core/src/compaction/model.rs` | Add PreservationContext, ConversationFlow, BuildStatus |
| `codelet/core/src/compaction/anchor.rs` | Add new detection patterns |
| `codelet/core/src/compaction/compactor.rs` | Update summary generation |
| `codelet/core/src/compaction/mod.rs` | Export new types |
| `codelet/cli/src/interactive_helpers.rs` | Update execute_compaction |
| `codelet/napi/src/session.rs` | Update compact() method |
| `codelet/napi/src/types.rs` | Add NAPI types for new structures |

---

## 8. Success Criteria

1. **Non-coding conversations have anchors**: Web search, bash commands create anchors
2. **No hardcoded summary text**: All context dynamically extracted
3. **Synthetic anchor fallback**: Always creates checkpoint when no natural anchors
4. **PreservationContext populated**: Active files, goals, build status extracted
5. **60%+ compression achieved**: For conversations with 5+ turns
6. **All existing tests pass**: No regressions
7. **New patterns tested**: Unit tests for each new anchor type

---

## 9. References

- TypeScript reference: `~/projects/codelet/src/agent/anchor-point-compaction.ts`
- Current Rust implementation: `fspec/codelet/core/src/compaction/`
- NAPI bindings: `fspec/codelet/napi/src/session.rs`

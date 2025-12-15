# Codelet Context Compaction & Anchoring System - Comprehensive Analysis

## Executive Summary

The Codelet project implements a sophisticated **anchor point-based context compaction system** designed to handle long CLI tool conversations while maintaining coherent context. The system achieves **60-80% compression ratios** through intelligent conversation flow analysis and weighted summarization, replacing a broken message-based approach that only achieved 0.2-1.0% compression.

### Key Innovations

1. **Anchor Point Detection**: Identifies natural conversation breakpoints (task completions, error resolutions)
2. **Turn-Based Architecture**: Groups messages into conversation turns instead of individual message protection
3. **Weighted Summarization**: Preserves critical context while aggressively compressing routine operations
4. **Cache-Aware Triggering**: Accounts for 90% prompt cache discount when determining compaction need
5. **LLM-Based Summarization**: Uses the active provider to generate intelligent summaries

---

## 1. Context Compaction Architecture

### 1.1 Core Components

The system consists of several integrated modules:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Agent Runner (runner.ts)                     │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Token State Manager                                       │  │
│  │ - Tracks total tokens (input + output)                    │  │
│  │ - Monitors cache read/creation tokens                     │  │
│  │ - Calculates effective tokens (with 90% cache discount)   │  │
│  │ - Provides warning levels (80%, 90%, 95%)                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            ↓                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Compaction Trigger Logic                                  │  │
│  │ - Threshold: 90% of context window                        │  │
│  │ - Uses effective tokens = input - (cacheRead * 0.9)       │  │
│  │ - Dynamic threshold based on model limits                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            ↓                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Anchor Point Compaction System                            │  │
│  │ - Converts messages to conversation turns                 │  │
│  │ - Detects anchor points (90%+ precision)                  │  │
│  │ - Selects turns for preservation                          │  │
│  │ - Generates weighted summaries                            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            ↓                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Message Reconstruction                                     │  │
│  │ - Injects summary as user message                         │  │
│  │ - Adds session continuation message                       │  │
│  │ - Clears prompt cache (signals boundary)                  │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 File Locations

**Primary Implementation Files:**
- `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` - Core anchor point system
- `/home/rquast/projects/codelet/src/agent/compaction.ts` - Bridge to legacy API
- `/home/rquast/projects/codelet/src/agent/runner.ts` - Integration point
- `/home/rquast/projects/codelet/src/agent/llm-summary-provider.ts` - LLM-based summarization
- `/home/rquast/projects/codelet/src/agent/token-state-manager.ts` - Token tracking
- `/home/rquast/projects/codelet/src/agent/system-reminders.ts` - Deduplication system
- `/home/rquast/projects/codelet/src/utils/prompt-cache.ts` - Cache state tracking

---

## 2. The Anchoring System

### 2.1 What is an Anchor Point?

An **anchor point** is a natural conversation breakpoint representing meaningful completion or resolution. Unlike message-level compaction, anchor points identify semantic boundaries in tool-heavy CLI conversations.

**Anchor Point Interface:**
```typescript
interface AnchorPoint {
  turnIndex: number;              // Position in conversation
  type: 'task-completion'         // Type of anchor
      | 'error-resolution'
      | 'user-checkpoint'
      | 'feature-milestone';
  weight: number;                 // 0.7-0.9 preservation weight
  description: string;            // Human-readable description
  timestamp: Date;                // When anchor was created
  confidence: number;             // Detection confidence (0.0-1.0)
}
```

### 2.2 Anchor Point Types & Weights

| Type | Weight | Trigger Pattern | Example |
|------|--------|----------------|---------|
| **error-resolution** | 0.9 | Previous error + Fix + Test success | Build fails → Edit config.ts → Tests pass |
| **task-completion** | 0.8 | File modification + Test success | Edit auth.ts → Run tests → All pass |
| **user-checkpoint** | 0.7 | Explicit user direction change | "ok now let's work on..." |
| **feature-milestone** | 0.8 | Scope/focus shift | Complete auth → Start database |

### 2.3 Anchor Point Detection Algorithm

**Location:** `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` (lines 116-254)

```typescript
class AnchorPointDetector {
  private readonly CONFIDENCE_THRESHOLD = 0.9;  // 90%+ precision requirement

  detectAnchorPoint(turn: ConversationTurn): AnchorPoint | null {
    const patterns = this.analyzeCompletionPatterns(turn);

    // Priority 1: Error Resolution (0.9 weight)
    if (patterns.errorResolution?.confidence >= 0.9) {
      return {
        type: 'error-resolution',
        weight: 0.9,
        description: patterns.errorResolution.description,
        confidence: patterns.errorResolution.confidence
      };
    }

    // Priority 2: Task Completion (0.8 weight)
    if (patterns.taskCompletion?.confidence >= 0.9) {
      return {
        type: 'task-completion',
        weight: 0.8,
        description: patterns.taskCompletion.description,
        confidence: patterns.taskCompletion.confidence
      };
    }

    return null;  // No high-confidence anchor
  }

  private analyzeCompletionPatterns(turn: ConversationTurn) {
    // Error resolution: Previous error + Fix + Success
    if (turn.previousError && hasFileModification && hasTestSuccess) {
      return {
        errorResolution: {
          confidence: 0.95,
          description: `Build error fixed in ${files} and tests now pass`
        }
      };
    }

    // Task completion: Modify + Test + Success (no previous error)
    if (!turn.previousError && hasFileModification && hasTestSuccess) {
      return {
        taskCompletion: {
          confidence: 0.92,
          description: `File changes implemented in ${files} and tests pass`
        }
      };
    }

    return {};  // No pattern detected
  }
}
```

**Key Detection Patterns:**

1. **Error Resolution Pattern** (confidence: 0.95)
   - `previousError = true`
   - Tool calls include Edit or Write
   - Tool results include successful test execution

2. **Task Completion Pattern** (confidence: 0.92)
   - `previousError = false`
   - Tool calls include Edit or Write
   - Tool results include successful test execution

3. **Conservative Strategy**
   - High precision (90%+) over recall
   - Avoids false positives from routine operations
   - Pattern-based (sequences not individual ops)
   - Context-aware (tests after error ≠ routine tests)

### 2.4 Conversation Turn Structure

**Instead of individual messages**, the system groups into turns:

```typescript
interface ConversationTurn {
  userMessage: string;              // User's request
  toolCalls: ToolCall[];            // All tools invoked
  toolResults: ToolResult[];        // All tool outputs
  assistantResponse: string;        // Assistant's summary
  tokens: number;                   // Total tokens for turn
  timestamp: Date;                  // Turn timestamp
  previousError?: boolean;          // Context for detection
}

interface ConversationFlow {
  turns: ConversationTurn[];        // All conversation turns
  anchorPoints: AnchorPoint[];      // Detected anchors
  totalTokens: number;              // Sum of all turn tokens
  preservationContext: {            // Critical state
    activeFiles: string[];
    currentGoals: string[];
    errorStates: string[];
    buildStatus: 'passing' | 'failing' | 'unknown';
    lastUserIntent: string;
  };
  migrationStrategy?: 'anchor-point-only' | 'legacy-fallback';
  syntheticAnchor?: AnchorPoint;    // Fallback if no anchors found
}
```

---

## 3. Compaction Trigger & Token Management

### 3.1 Effective Token Calculation

**Location:** `/home/rquast/projects/codelet/src/agent/runner.ts` (lines 124-146)

The system uses **effective tokens** that account for Anthropic's prompt cache discount:

```typescript
export function calculateEffectiveTokens(tracker: TokenUsage): number {
  const cacheDiscount = tracker.cacheReadInputTokens * 0.9;  // 90% cheaper
  return tracker.inputTokens - cacheDiscount;
}

// Example:
// Input: 100k tokens, 80k from cache
// Effective: 100k - (80k * 0.9) = 100k - 72k = 28k tokens
```

**Why this matters:** When prompt caching is effective, we don't need to compact as aggressively because cached content is 90% cheaper. This prevents unnecessary compaction cycles.

### 3.2 Compaction Trigger Logic

**Location:** `/home/rquast/projects/codelet/src/agent/runner.ts` (lines 100-107, 829-949)

```typescript
const COMPACTION_THRESHOLD_PERCENT = 0.9;  // 90% of context window

function getCompactionThreshold(contextWindow: number): number {
  return Math.floor(contextWindow * 0.9);
}

function shouldTriggerCompaction(tracker: TokenUsage, threshold: number): boolean {
  const effectiveTokens = calculateEffectiveTokens(tracker);
  return effectiveTokens > threshold;
}
```

**Compaction Process (runner.ts lines 829-949):**

1. **Check threshold** before each LLM call
2. **If exceeded:**
   - Display "[Generating summary...]"
   - Convert messages to conversation flow
   - Calculate summarization budget
   - Execute anchor point compaction
   - Clear prompt cache
   - Inject summary + session continuation
   - Display metrics
3. **Continue** with compacted context

**Budget Calculation:**
```typescript
export const AUTOCOMPACT_BUFFER = 50000;  // Reserved for operations

function calculateSummarizationBudget(
  contextWindow: number,
  autocompactBuffer: number
): number {
  if (contextWindow <= autocompactBuffer) {
    return Math.floor(contextWindow * 0.8);  // 80% of window
  }
  return contextWindow - autocompactBuffer;  // Window - buffer
}
```

---

## 4. Turn Selection & Compression Strategy

### 4.1 Selection Algorithm

**Location:** `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` (lines 321-363)

```typescript
export function selectTurnsForCompaction(
  flow: ConversationFlow,
  targetTokens: number
): TurnSelectionResult {
  const totalTurns = flow.turns.length;

  // ALWAYS preserve last 2-3 complete conversation turns
  const turnsToAlwaysKeep = Math.min(3, totalTurns);
  const recentTurns = flow.turns.slice(-turnsToAlwaysKeep);
  const olderTurns = flow.turns.slice(0, -turnsToAlwaysKeep);

  // Find most recent anchor point in older turns
  const anchorPointInOlderTurns = flow.anchorPoints
    .filter(anchor => anchor.turnIndex < totalTurns - turnsToAlwaysKeep)
    .sort((a, b) => b.turnIndex - a.turnIndex)[0];

  if (anchorPointInOlderTurns) {
    // Keep: anchor + everything after it + recent turns
    const turnsToKeep = [
      ...olderTurns.slice(anchorPointInOlderTurns.turnIndex),
      ...recentTurns
    ];
    const turnsToSummarize = olderTurns.slice(0, anchorPointInOlderTurns.turnIndex);

    return {
      turnsToKeep,
      turnsToSummarize,
      preservedAnchors: [anchorPointInOlderTurns],
      compressionEstimate: turnsToSummarize.length / totalTurns
    };
  }

  // No anchor found - keep recent, summarize the rest
  return {
    turnsToKeep: recentTurns,
    turnsToSummarize: olderTurns,
    preservedAnchors: [],
    compressionEstimate: olderTurns.length / totalTurns
  };
}
```

**Visual Example:**

```
90 turns total, 90k tokens
↓
Last 3 turns ALWAYS kept (recent context)
↓
Find most recent anchor in older 87 turns
↓
Anchor found at turn 40
↓
Keep: Turn 40 → Turn 90 (51 turns)
Summarize: Turn 1 → Turn 39 (39 turns)
↓
Result: 39/90 = 43% compression
```

### 4.2 Compression Quality Gates

**Location:** `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` (lines 368-427)

```typescript
async function compactConversationFlow(
  flow: ConversationFlow,
  summaryProvider: WeightedSummaryProvider,
  options: { targetTokens: number }
): Promise<ConversationCompactionResult> {

  // ... compaction logic ...

  const compressionRatio = (originalTokens - compactedTokens) / originalTokens;

  // Quality gate: Minimum 60% compression
  const warnings: string[] = [];
  if (compressionRatio < 0.6) {
    warnings.push(
      'Compression ratio below 60% - consider starting fresh conversation'
    );
  }

  return {
    compactedFlow,
    summary,
    compactedTokens,
    originalTokens,
    compressionRatio,
    preservedAnchorPoints,
    uncompactedTurns: selection.turnsToKeep.slice(-2),
    warnings
  };
}
```

**Compression Ratio Targets:**
- **< 60%**: Algorithm failure → Warning + suggest fresh conversation
- **60-70%**: Good compression
- **70-80%**: Excellent compression (target range)
- **80%+**: Outstanding compression

---

## 5. Weighted Summarization

### 5.1 WeightedSummaryProvider

**Location:** `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` (lines 259-316)

```typescript
class WeightedSummaryProvider {
  async generateWeightedSummary(
    turns: ConversationTurn[],
    anchors: AnchorPoint[],
    preservationContext: PreservationContext
  ): Promise<string> {
    // Transform tool sequences into outcome descriptions
    const outcomes = turns.map(turn => this.turnToOutcome(turn, anchors));

    // Preserve critical context
    const contextSummary = this.preserveContext(preservationContext);

    return `${contextSummary}\n\nKey outcomes:\n${outcomes.join('\n')}`;
  }

  private turnToOutcome(turn: ConversationTurn, anchors: AnchorPoint[]): string {
    const isAnchor = anchors.some(a => a.timestamp === turn.timestamp);

    if (isAnchor) {
      // Anchor points get detailed preservation
      return `[ANCHOR] ${turn.assistantResponse}`;
    }

    // Regular turns get compressed to outcomes
    const files = this.extractFilesFromTurn(turn);
    const success = turn.toolResults.some(r => r.success);

    return `${success ? '✓' : '✗'} ${files.length ? `Modified ${files.join(', ')}: ` : ''}${turn.assistantResponse.split('.')[0]}`;
  }

  private preserveContext(context: PreservationContext): string {
    return `Active files: ${context.activeFiles.join(', ')}
Goals: ${context.currentGoals.join('; ')}
Build: ${context.buildStatus}`;
  }
}
```

**Example Output:**

```
Active files: auth.ts, config.ts
Goals: Fix authentication; Improve performance
Build: passing

Key outcomes:
[ANCHOR] Successfully implemented authentication validation in auth.ts, all tests pass
✓ Modified config.ts: Updated database connection settings
✓ Modified auth.ts: Added password hashing
[ANCHOR] Build error fixed in config.ts and tests now pass
✓ Refactored user service
```

### 5.2 LLM-Based Summarization

**Location:** `/home/rquast/projects/codelet/src/agent/llm-summary-provider.ts`

```typescript
const SUMMARY_PROMPT = `You are helping continue a conversation that has grown too long.

Create a concise continuation summary that captures:
1. The main topic/goal of the conversation
2. Key decisions made
3. Files changed or modified
4. Important context needed to continue
5. Current state/progress
6. Any blockers or issues

Format as a natural continuation message that allows the conversation to resume smoothly.
Keep it under 400 words and ensure the summary is under 1024 tokens.`;

class LLMSummaryProvider implements SummaryProvider {
  private readonly MAX_RETRIES = 3;
  private readonly RETRY_DELAYS_MS = [0, 1000, 2000];  // Exponential backoff

  async generateSummary(messages: MessageForCompaction[]): Promise<string> {
    const conversationContext = messages
      .map(msg => `[${msg.role}]: ${msg.content}`)
      .join('\n\n');

    const fullPrompt = `${SUMMARY_PROMPT}\n\nConversation to summarize:\n${conversationContext}`;

    // Retry loop with exponential backoff
    for (let attempt = 0; attempt < this.MAX_RETRIES; attempt++) {
      try {
        if (this.RETRY_DELAYS_MS[attempt] > 0) {
          await sleep(this.RETRY_DELAYS_MS[attempt]);
        }

        const model = await this.providerManager.getModel();
        const result = await generateText({
          model,
          prompt: fullPrompt,
          maxOutputTokens: 1024
        });

        return result.text;
      } catch (error) {
        // Continue to next retry
      }
    }

    throw lastError;  // All retries exhausted
  }
}
```

---

## 6. Implementation Checklist for codelet

**Phase 1: Core Data Structures**
- [ ] Define ConversationTurn struct
- [ ] Define AnchorPoint struct
- [ ] Define ConversationFlow struct
- [ ] Define PreservationContext struct
- [ ] Implement turn grouping from messages

**Phase 2: Anchor Detection**
- [ ] Implement AnchorPointDetector
- [ ] Add analyzeCompletionPatterns method
- [ ] Set CONFIDENCE_THRESHOLD = 0.9
- [ ] Implement error resolution pattern (0.95 confidence)
- [ ] Implement task completion pattern (0.92 confidence)
- [ ] Add detectHistoricalAnchors for migration

**Phase 3: Turn Selection**
- [ ] Implement selectTurnsForCompaction function
- [ ] Always preserve last 2-3 turns
- [ ] Find most recent anchor in older turns
- [ ] Create synthetic anchor if none found
- [ ] Calculate compression estimate

**Phase 4: Weighted Summarization**
- [ ] Implement WeightedSummaryProvider
- [ ] Add turnToOutcome method
- [ ] Add preserveContext method
- [ ] Weight anchors higher in summary
- [ ] Extract files from turns

**Phase 5: LLM Summarization**
- [ ] Create LLMSummaryProvider
- [ ] Define SUMMARY_PROMPT template
- [ ] Implement retry logic (3 attempts)
- [ ] Add exponential backoff
- [ ] Create fallback summary generator

**Phase 6: Integration**
- [ ] Implement effective token calculation
- [ ] Add compaction trigger logic (90% threshold)
- [ ] Convert messages to flow
- [ ] Execute compaction
- [ ] Reconstruct messages with summary
- [ ] Clear cache after compaction

**Phase 7: Quality & Testing**
- [ ] Add compression ratio validation (>= 60%)
- [ ] Emit warnings on failure
- [ ] Test with long conversations
- [ ] Verify anchor detection accuracy
- [ ] Validate summary quality
- [ ] Check cache integration

---

## Summary

The Codelet context compaction system represents a sophisticated approach to managing long CLI conversations through:

1. **Anchor Point Detection**: Identifying natural breakpoints with 90%+ precision
2. **Turn-Based Architecture**: Grouping operations for better compression
3. **Weighted Summarization**: Preserving critical context while compressing routine operations
4. **Cache-Aware Triggering**: Using effective tokens to prevent premature compaction
5. **LLM-Based Summarization**: Intelligent context preservation via current provider

The system achieves 60-80% compression ratios compared to the previous 0.2-1.0%, making it suitable for long-running coding sessions while maintaining conversation coherence.

**Key files to study for implementation:**
- `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts`
- `/home/rquast/projects/codelet/src/agent/runner.ts`
- `/home/rquast/projects/codelet/src/agent/llm-summary-provider.ts`
- `/home/rquast/projects/codelet/src/agent/token-state-manager.ts`

---

**Report Generated**: 2025-12-03
**Codebase**: /home/rquast/projects/codelet
**Analysis Scope**: Context compaction, anchor point system, weighted summarization

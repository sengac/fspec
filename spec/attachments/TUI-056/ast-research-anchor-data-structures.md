# AST Research: Anchor Data Structures

## Rust AnchorPoint Structure (codelet/core/src/compaction/anchor.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorPoint {
    /// Index of turn in conversation history  
    pub turn_index: usize,
    /// Type of anchor
    pub anchor_type: AnchorType,
    /// Weight for preservation (0.7-0.9)
    pub weight: f64,
    /// Detection confidence (0.0-1.0)
    pub confidence: f64,
    /// Human-readable description
    pub description: String,
    /// Timestamp when anchor was created
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorType {
    ErrorResolution,    // weight 0.9
    TaskCompletion,     // weight 0.8  
    UserCheckpoint,     // weight 0.7
    FeatureMilestone,   // weight 0.75
}
```

## TypeScript Interface (src/tui/types/anchor.ts)

```typescript
export type AnchorType = 'ErrorResolution' | 'TaskCompletion' | 'UserCheckpoint' | 'FeatureMilestone';

export interface AnchorPoint {
  turnIndex: number;
  anchorType: AnchorType;
  weight: number;
  confidence: number;
  description: string;
  timestamp: number; // Note: SystemTime -> number conversion needed
}

export interface AnchorTurnDetails {
  turnIndex: number;
  userMessage: string;
  assistantResponse: string;
  toolCalls: Array<{tool: string, parameters: Record<string, any>, success: boolean}>;
  fileModifications: Array<{path: string, operation: 'create'|'edit'|'delete', summary: string}>;
  status: 'success' | 'partial' | 'failed';
  context: string;
}
```

## Key Data Flow

1. **Rust side**: Session stores anchor points from context compaction
2. **NAPI layer**: Convert Rust structs to JavaScript objects
3. **TypeScript side**: Display in AnchorViewerDialog using VirtualList

## NAPI Functions Needed

1. `session.getAnchorPoints() -> Promise<AnchorPoint[]>`
2. `session.getTurnDetails(turnIndex: number) -> Promise<AnchorTurnDetails | null>`
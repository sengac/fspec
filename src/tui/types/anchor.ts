/**
 * Types for TUI-056: Interactive anchor point viewer
 */

export type AnchorType =
  | 'ErrorResolution'
  | 'TaskCompletion'
  | 'UserCheckpoint'
  | 'FeatureMilestone';

/** Tool call info stored with anchor */
export interface AnchorToolCall {
  /** Tool name (e.g., "Edit", "Write", "Bash") */
  tool: string;
  /** Whether the tool call succeeded */
  success: boolean;
}

export interface AnchorPoint {
  /** Index of turn in conversation history */
  turnIndex: number;
  /** Type of anchor */
  anchorType: AnchorType;
  /** Weight for preservation (0.7-0.9) */
  weight: number;
  /** Detection confidence (0.0-1.0) */
  confidence: number;
  /** Human-readable description */
  description: string;
  /** Timestamp when anchor was created */
  timestamp: number;
  /** User message content at this turn (captured at anchor creation time) */
  userMessage?: string;
  /** Assistant response content at this turn (captured at anchor creation time) */
  assistantResponse?: string;
  /** Tool calls made in this turn (captured at anchor creation time) */
  toolCalls: AnchorToolCall[];
}

export interface AnchorTurnDetails {
  /** Turn index for reference */
  turnIndex: number;
  /** User message for this turn */
  userMessage: string;
  /** Assistant response for this turn */
  assistantResponse: string;
  /** Tool calls made during this turn */
  toolCalls: Array<{
    tool: string;
    parameters: Record<string, unknown>;
    success: boolean;
  }>;
  /** File modifications made during this turn */
  fileModifications: Array<{
    path: string;
    operation: 'create' | 'edit' | 'delete';
    summary: string;
  }>;
  /** Overall success/failure status of turn */
  status: 'success' | 'partial' | 'failed';
  /** Brief context about what happened */
  context: string;
}

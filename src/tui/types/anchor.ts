/**
 * Types for TUI-056: Interactive anchor point viewer
 */

export type AnchorType =
  | 'ErrorResolution'
  | 'TaskCompletion'
  | 'UserCheckpoint'
  | 'FeatureMilestone';

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

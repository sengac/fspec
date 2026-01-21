/**
 * Conversation types shared between AgentView and SplitSessionView
 *
 * SOLID: Single source of truth for conversation-related types
 */

// Message type determines semantic meaning and display behavior
export type MessageType =
  | 'user-input' // User's input text
  | 'assistant-text' // Assistant's response text (streaming or complete)
  | 'thinking' // Extended thinking/reasoning content
  | 'tool-call' // Tool invocation (header + result)
  | 'status'; // Status messages (interrupted, errors, etc.)

// Conversation message type for display
// SOLID: type field provides semantic meaning, role is derived for display
export interface ConversationMessage {
  type: MessageType;
  content: string;
  fullContent?: string; // TUI-043: Full uncollapsed content for expandable messages
  isStreaming?: boolean;
  isError?: boolean; // Tool result with isError=true (stderr output)
  toolCallId?: string; // For tool-call messages, links header to result
  /** WATCH-011: Correlation ID for cross-pane selection highlighting */
  correlationId?: string;
  /** WATCH-011: Parent chunk IDs this watcher turn was observing when it responded */
  observedCorrelationIds?: string[];
}

// Line type for VirtualList (flattened from messages)
// Each line represents ONE visual line in the terminal
export interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  messageIndex: number;
  isSeparator?: boolean; // TUI-042: Empty line used as turn separator
  isThinking?: boolean; // Thinking content (for yellow rendering)
  isError?: boolean; // Tool result with isError=true (stderr output)
  /** WATCH-011: Correlation ID for cross-pane selection highlighting */
  correlationId?: string;
  /** WATCH-011: Parent chunk IDs this watcher turn was observing when it responded */
  observedCorrelationIds?: string[];
}

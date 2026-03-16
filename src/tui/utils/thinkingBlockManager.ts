/**
 * Thinking Block Manager
 *
 * Single source of truth for managing thinking blocks in conversation messages.
 * Handles streaming accumulation, block transitions, and proper finalization.
 *
 * SOLID: Single Responsibility - only manages thinking block state
 * DRY: One implementation shared by all consumers
 * Composable: Pure functions that operate on message arrays
 *
 * ## Key Design Decisions
 *
 * 1. **No Index Tracking**: We NEVER store indices because:
 *    - React state updates are asynchronous and may be batched
 *    - Array mutations (splice/pop) invalidate indices
 *    - Indices captured in closures become stale
 *
 * 2. **Marker-Based Identification**: We use `isStreaming: true` to identify
 *    the currently active thinking block. This is robust because:
 *    - The marker is part of the message, not external state
 *    - findLastIndex searches fresh array state each time
 *    - Tool calls finalize the block by setting `isStreaming: false`
 *
 * 3. **Finalization on Transition**: When transitioning from thinking to
 *    another content type (tool call, done), we finalize the thinking block.
 *    This ensures clean boundaries between thinking segments.
 */

import type { ConversationMessage } from '../types/conversation';

// =============================================================================
// Constants
// =============================================================================

/** Prefix added to thinking block content for display */
const THINKING_PREFIX = '[Thinking]\n';

// =============================================================================
// Types
// =============================================================================

/**
 * Options for appending thinking content
 */
export interface AppendThinkingOptions {
  /** Correlation ID for cross-pane selection highlighting */
  correlationId?: string;
  /** Subordinate chunk IDs this supervisor turn was observing */
  observedCorrelationIds?: string[];
}

// =============================================================================
// Core Functions
// =============================================================================

/**
 * Find the currently active (streaming) thinking block.
 * Returns the index, or -1 if no active thinking block exists.
 *
 * IMPORTANT: A thinking block is NOT active if there are user/supervisor
 * messages after it - that means we've moved to a new turn.
 *
 * @param messages - Conversation messages array
 * @returns Index of active thinking block, or -1
 */
export function findActiveThinkingBlock(
  messages: ConversationMessage[]
): number {
  const lastStreamingThinking = messages.findLastIndex(
    m => m.type === 'thinking' && m.isStreaming === true
  );

  if (lastStreamingThinking < 0) {
    return -1;
  }

  // Check if there are any turn-boundary messages after this thinking block
  // User/supervisor input marks a new turn, so this thinking block is stale
  for (let i = lastStreamingThinking + 1; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.type === 'user-input' || msg.type === 'supervisor-input') {
      return -1;
    }
  }

  return lastStreamingThinking;
}

/**
 * Find the last thinking block that could be appended to.
 * This is either an active (streaming) thinking block, OR a thinking block
 * that comes after the last tool call and user input (same turn).
 *
 * This is used for bulk processing where we don't have streaming markers.
 *
 * IMPORTANT: A thinking block is NOT appendable if there are user/supervisor
 * messages after it - that means we've moved to a new turn.
 *
 * @param messages - Conversation messages array
 * @returns Index of appendable thinking block, or -1
 */
export function findAppendableThinkingBlock(
  messages: ConversationMessage[]
): number {
  const lastThinkingIdx = messages.findLastIndex(m => m.type === 'thinking');

  if (lastThinkingIdx < 0) {
    return -1;
  }

  // Check if there are any turn-boundary messages after this thinking block
  // User/supervisor input marks a new turn, so this thinking block is stale
  for (let i = lastThinkingIdx + 1; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.type === 'user-input' || msg.type === 'supervisor-input') {
      return -1;
    }
  }

  // Check if streaming (can append to streaming blocks)
  if (messages[lastThinkingIdx].isStreaming === true) {
    return lastThinkingIdx;
  }

  // For non-streaming, check if it's in the current assistant turn
  // (must be after last tool call)
  const lastToolIdx = messages.findLastIndex(m => m.type === 'tool-call');
  const canAppend = lastToolIdx < 0 || lastThinkingIdx > lastToolIdx;

  return canAppend ? lastThinkingIdx : -1;
}

/**
 * Append thinking content to the conversation.
 *
 * If there's an active thinking block, appends to it.
 * Otherwise, creates a new thinking block.
 *
 * For streaming mode:
 * - New blocks are created with `isStreaming: true`
 * - Use `finalizeThinkingBlock()` when transitioning to other content
 *
 * @param messages - Conversation messages array (mutated in place)
 * @param content - Thinking content to append
 * @param options - Optional correlation IDs
 * @returns The messages array (same reference)
 */
export function appendThinking(
  messages: ConversationMessage[],
  content: string,
  options: AppendThinkingOptions = {}
): ConversationMessage[] {
  if (!content) {
    return messages;
  }

  const activeIdx = findActiveThinkingBlock(messages);

  if (activeIdx >= 0) {
    // Append to existing active thinking block
    const existing = messages[activeIdx];
    const existingContent = existing.content.startsWith(THINKING_PREFIX)
      ? existing.content.slice(THINKING_PREFIX.length)
      : existing.content;

    messages[activeIdx] = {
      ...existing,
      content: `${THINKING_PREFIX}${existingContent}${content}`,
    };

    // Update correlation IDs if provided
    if (options.correlationId && !messages[activeIdx].correlationId) {
      messages[activeIdx].correlationId = options.correlationId;
    }
    if (
      options.observedCorrelationIds &&
      !messages[activeIdx].observedCorrelationIds
    ) {
      messages[activeIdx].observedCorrelationIds =
        options.observedCorrelationIds;
    }
  } else {
    // Create new thinking block
    // Insert before streaming assistant message if one exists
    const streamingIdx = messages.findLastIndex(
      m => m.type === 'assistant-text' && m.isStreaming
    );

    const newThinking: ConversationMessage = {
      type: 'thinking',
      content: `${THINKING_PREFIX}${content}`,
      isStreaming: true,
      correlationId: options.correlationId,
      observedCorrelationIds: options.observedCorrelationIds,
    };

    if (streamingIdx >= 0) {
      messages.splice(streamingIdx, 0, newThinking);
    } else {
      messages.push(newThinking);
    }
  }

  return messages;
}

/**
 * Finalize the active thinking block.
 *
 * Call this when transitioning from thinking content to another type
 * (e.g., tool call, done, error). This marks the thinking block as
 * complete (`isStreaming: false`), ensuring the next thinking content
 * creates a new block.
 *
 * @param messages - Conversation messages array (mutated in place)
 * @returns The messages array (same reference)
 */
export function finalizeThinkingBlock(
  messages: ConversationMessage[]
): ConversationMessage[] {
  const activeIdx = findActiveThinkingBlock(messages);

  if (activeIdx >= 0) {
    messages[activeIdx] = {
      ...messages[activeIdx],
      isStreaming: false,
    };
  }

  return messages;
}

/**
 * Append thinking content for bulk processing (non-streaming).
 *
 * This version is for processing merged chunks where we don't have
 * real-time streaming state. It uses turn boundaries (tool calls,
 * user input) to determine if content can be appended.
 *
 * @param messages - Conversation messages array (mutated in place)
 * @param content - Thinking content to append
 * @param options - Optional correlation IDs
 * @returns The messages array (same reference)
 */
export function appendThinkingBulk(
  messages: ConversationMessage[],
  content: string,
  options: AppendThinkingOptions = {}
): ConversationMessage[] {
  if (!content) {
    return messages;
  }

  const appendableIdx = findAppendableThinkingBlock(messages);

  if (appendableIdx >= 0) {
    // Append to existing thinking block
    const existing = messages[appendableIdx];
    const existingContent = existing.content.startsWith(THINKING_PREFIX)
      ? existing.content.slice(THINKING_PREFIX.length)
      : existing.content;

    messages[appendableIdx] = {
      ...existing,
      content: `${THINKING_PREFIX}${existingContent}${content}`,
    };

    // Update correlation IDs if provided
    if (options.correlationId && !messages[appendableIdx].correlationId) {
      messages[appendableIdx].correlationId = options.correlationId;
    }
    if (
      options.observedCorrelationIds &&
      !messages[appendableIdx].observedCorrelationIds
    ) {
      messages[appendableIdx].observedCorrelationIds =
        options.observedCorrelationIds;
    }
  } else {
    // Create new thinking block (non-streaming for bulk)
    const newThinking: ConversationMessage = {
      type: 'thinking',
      content: `${THINKING_PREFIX}${content}`,
      isStreaming: false, // Bulk processing doesn't use streaming markers
      correlationId: options.correlationId,
      observedCorrelationIds: options.observedCorrelationIds,
    };

    messages.push(newThinking);
  }

  return messages;
}

// =============================================================================
// Streaming State Manager (for use with React state)
// =============================================================================

/**
 * Create a thinking block update for React state.
 *
 * This function is designed to work with React's functional state updates
 * (`setConversation(prev => ...)`). It returns a new array with the
 * thinking content appended.
 *
 * IMPORTANT: This is a pure function that doesn't mutate the input.
 *
 * @param prev - Previous conversation state
 * @param content - Thinking content to append
 * @param options - Optional correlation IDs
 * @returns New conversation array
 */
export function createThinkingUpdate(
  prev: ConversationMessage[],
  content: string,
  options: AppendThinkingOptions = {}
): ConversationMessage[] {
  if (!content) {
    return prev;
  }

  const updated = [...prev];
  return appendThinking(updated, content, options);
}

/**
 * Create a finalization update for React state.
 *
 * Returns a new array with the active thinking block finalized.
 *
 * @param prev - Previous conversation state
 * @returns New conversation array
 */
export function createFinalizationUpdate(
  prev: ConversationMessage[]
): ConversationMessage[] {
  const activeIdx = findActiveThinkingBlock(prev);

  if (activeIdx < 0) {
    return prev; // No change needed
  }

  const updated = [...prev];
  updated[activeIdx] = {
    ...updated[activeIdx],
    isStreaming: false,
  };

  return updated;
}

/**
 * Chunk Processor
 *
 * Single source of truth for converting StreamChunks to ConversationMessages.
 * Handles all chunk types consistently across bulk loading and streaming.
 *
 * SOLID: Single Responsibility - chunk-to-message conversion only
 * DRY: One implementation shared by all consumers
 */

import type { StreamChunk } from '@sengac/codelet-napi';
import type { ConversationMessage } from '../types/conversation';
import { formatMarkdownTables } from './markdown-table-formatter';
import {
  appendThinking,
  appendThinkingBulk,
  finalizeThinkingBlock,
} from './thinkingBlockManager';

// ============================================================================
// Types
// ============================================================================

/**
 * Parsed supervisor/bridge message info
 */
export interface ParsedSupervisorInfo {
  role: string;
  sessionId: string;
  content: string;
}

/**
 * Pending tool call info for tracking inputs across ToolCall → ToolResult
 */
export interface PendingToolCallInfo {
  name: string;
  input: Record<string, unknown>;
}

/**
 * Context needed for chunk processing (formatters, state)
 */
export interface ChunkProcessorContext {
  /** Format tool header: (name, args) => "● ToolName(args)" */
  formatToolHeader: (name: string, args: string) => string;
  /** Format collapsed output for tool results */
  formatCollapsedOutput: (content: string) => string;
  /** Pending tool calls map for Edit/Write diff display */
  pendingToolCalls?: Map<string, PendingToolCallInfo>;
}

/**
 * Result of processing a single chunk
 */
export interface ChunkProcessResult {
  /** Message to add (null if chunk should be merged with existing) */
  message: ConversationMessage | null;
  /** If true, update the last streaming message instead of adding new */
  updateLast?: boolean;
  /** Content to append if updateLast is true */
  appendContent?: string;
  /** If true, mark last streaming message as complete */
  finalizeStreaming?: boolean;
}

// ============================================================================
// Supervisor Prefix Parsing
// ============================================================================

/**
 * Parse supervisor message prefix to extract role, session ID, and content.
 * Format: [SUPERVISOR: role | Session: id]\ncontent
 *
 * Used for both supervisor session injections and bridge inputs.
 *
 * @param text - The raw message text
 * @returns ParsedSupervisorInfo if prefix found, null otherwise
 */
export function parseSupervisorPrefix(
  text: string
): ParsedSupervisorInfo | null {
  const match = text.match(/^\[SUPERVISOR: ([^|]+) \| Session: ([^\]]+)\]\n?/);
  if (match) {
    return {
      role: match[1].trim(),
      sessionId: match[2].trim(),
      content: text.slice(match[0].length),
    };
  }
  return null;
}

/**
 * Format supervisor info for display in conversation.
 * Produces: "[W] role> content"
 */
export function formatSupervisorMessage(info: ParsedSupervisorInfo): string {
  return `[W] ${info.role}> ${info.content}`;
}

/**
 * Process a SupervisorInput chunk into a conversation message.
 */
export function processSupervisorInputChunk(text: string): ConversationMessage {
  const supervisorInfo = parseSupervisorPrefix(text);
  if (supervisorInfo) {
    return {
      type: 'supervisor-input',
      content: formatSupervisorMessage(supervisorInfo),
    };
  }
  // Fallback: display raw message if parsing fails
  return {
    type: 'supervisor-input',
    content: text,
  };
}

// ============================================================================
// Tool Args Display
// ============================================================================

/**
 * Extract args display string from tool input object.
 * DRY: Centralizes the logic for displaying tool arguments in headers.
 * Shows ALL parameters for full visibility into tool calls.
 * Exception: Edit/Write tools only show file_path (content is shown as diff).
 */
export function extractToolArgsDisplay(
  toolName: string,
  inputObj: Record<string, unknown>
): string {
  const toolNameLower = toolName.toLowerCase();

  // Edit/Write tools: only show file_path (content displayed as diff in result)
  if (
    toolNameLower === 'edit' ||
    toolNameLower === 'replace' ||
    toolNameLower === 'write' ||
    toolNameLower === 'write_file'
  ) {
    if (inputObj.file_path) {
      return String(inputObj.file_path);
    }
    return '';
  }

  // Tools with command/action_type: show it first, then remaining params
  const commandKey = inputObj.command
    ? 'command'
    : inputObj.action_type
      ? 'action_type'
      : null;
  if (commandKey) {
    const command = String(inputObj[commandKey]);
    const otherEntries = Object.entries(inputObj).filter(
      ([key]) => key !== commandKey
    );

    if (otherEntries.length === 0) {
      return command;
    }

    const parts = otherEntries.map(([key, value]) => {
      if (typeof value === 'string') {
        const displayValue =
          value.length > 100 ? `${value.slice(0, 100)}...` : value;
        return `${key}: '${displayValue}'`;
      } else if (value === null || value === undefined) {
        return `${key}: ${value}`;
      } else {
        const jsonStr = JSON.stringify(value);
        const displayValue =
          jsonStr.length > 100 ? `${jsonStr.slice(0, 100)}...` : jsonStr;
        return `${key}: ${displayValue}`;
      }
    });

    return `${command}, { ${parts.join(', ')} }`;
  }

  // Show ALL parameters as JSON-like object for full visibility
  const entries = Object.entries(inputObj);
  if (entries.length === 0) {
    return '';
  }

  const parts = entries.map(([key, value]) => {
    if (typeof value === 'string') {
      const displayValue =
        value.length > 100 ? `${value.slice(0, 100)}...` : value;
      return `${key}: '${displayValue}'`;
    } else if (value === null || value === undefined) {
      return `${key}: ${value}`;
    } else {
      const jsonStr = JSON.stringify(value);
      const displayValue =
        jsonStr.length > 100 ? `${jsonStr.slice(0, 100)}...` : jsonStr;
      return `${key}: ${displayValue}`;
    }
  });

  return `{ ${parts.join(', ')} }`;
}

// ============================================================================
// Bulk Chunk Processing (for session restore/resume)
// ============================================================================

/**
 * Process an array of merged chunks into conversation messages.
 * Used when restoring/resuming sessions (bulk loading).
 *
 * @param chunks - Array of StreamChunks from sessionGetMergedOutput
 * @param ctx - Processing context with formatters
 * @returns Array of ConversationMessages
 */
export function processChunksToMessages(
  chunks: StreamChunk[],
  ctx: ChunkProcessorContext
): ConversationMessage[] {
  const messages: ConversationMessage[] = [];
  const pendingToolCalls =
    ctx.pendingToolCalls ?? new Map<string, PendingToolCallInfo>();

  for (const chunk of chunks) {
    // Extract correlation fields
    const correlationId = chunk.correlationId;
    const observedCorrelationIds = chunk.observedCorrelationIds;

    if (chunk.type === 'UserInput' && chunk.text) {
      messages.push({
        type: 'user-input',
        content: chunk.text,
        correlationId,
        observedCorrelationIds,
      });
    } else if (chunk.type === 'SupervisorInput' && chunk.text) {
      const msg = processSupervisorInputChunk(chunk.text);
      msg.correlationId = correlationId;
      msg.observedCorrelationIds = observedCorrelationIds;
      messages.push(msg);
    } else if (chunk.type === 'Text' && chunk.text) {
      // Find last assistant-text message to append to, or create new one
      const lastIdx = messages.findLastIndex(m => m.type === 'assistant-text');
      if (lastIdx >= 0 && messages[lastIdx].isStreaming) {
        messages[lastIdx].content += chunk.text;
        // Merge observed correlation IDs
        if (observedCorrelationIds && observedCorrelationIds.length > 0) {
          if (!messages[lastIdx].observedCorrelationIds) {
            messages[lastIdx].observedCorrelationIds = [];
          }
          for (const id of observedCorrelationIds) {
            if (!messages[lastIdx].observedCorrelationIds!.includes(id)) {
              messages[lastIdx].observedCorrelationIds!.push(id);
            }
          }
        }
      } else {
        messages.push({
          type: 'assistant-text',
          content: chunk.text,
          isStreaming: true,
          correlationId,
          observedCorrelationIds,
        });
      }
    } else if (chunk.type === 'Thinking' && chunk.thinking) {
      appendThinkingBulk(messages, chunk.thinking, {
        correlationId,
        observedCorrelationIds,
      });
    } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
      // Finalize any active thinking block before tool call
      finalizeThinkingBlock(messages);

      const toolCall = chunk.toolCall;
      let argsDisplay = '';
      let parsedInput: Record<string, unknown> = {};

      try {
        parsedInput = JSON.parse(toolCall.input) as Record<string, unknown>;
        argsDisplay = extractToolArgsDisplay(toolCall.name, parsedInput);
      } catch {
        argsDisplay = toolCall.input;
      }

      // Store for ToolResult processing
      pendingToolCalls.set(toolCall.id, {
        name: toolCall.name,
        input: parsedInput,
      });

      // Finalize streaming assistant message
      const streamingIdx = messages.findLastIndex(m => m.isStreaming);
      if (streamingIdx >= 0) {
        if (messages[streamingIdx].content.trim() === '') {
          messages.splice(streamingIdx, 1);
        } else {
          messages[streamingIdx] = {
            ...messages[streamingIdx],
            content: formatMarkdownTables(messages[streamingIdx].content),
            isStreaming: false,
          };
        }
      }

      messages.push({
        type: 'tool-call',
        content: ctx.formatToolHeader(toolCall.name, argsDisplay),
        toolCallId: toolCall.id,
        correlationId,
        observedCorrelationIds,
      });
    } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
      const result = chunk.toolResult;
      const sanitizedContent = result.content.replace(/\t/g, '  ');
      const toolResultContent = ctx.formatCollapsedOutput(sanitizedContent);

      // Find and update the corresponding tool call message
      for (let i = messages.length - 1; i >= 0; i--) {
        const msg = messages[i];
        if (msg.type === 'tool-call' && msg.toolCallId === result.toolCallId) {
          const headerLine = msg.content.split('\n')[0];
          const hasContent = toolResultContent && toolResultContent.trim();
          messages[i] = {
            ...msg,
            content: hasContent
              ? `${headerLine}\n${toolResultContent}`
              : headerLine,
            fullContent: sanitizedContent,
            isError: result.isError,
          };
          break;
        }
      }

      // Add streaming placeholder for continuation
      messages.push({
        type: 'assistant-text',
        content: '',
        isStreaming: true,
        correlationId,
        observedCorrelationIds,
      });
    } else if (chunk.type === 'Done') {
      // Remove empty streaming messages and finalize
      while (
        messages.length > 0 &&
        messages[messages.length - 1].type === 'assistant-text' &&
        messages[messages.length - 1].isStreaming &&
        !messages[messages.length - 1].content
      ) {
        messages.pop();
      }
      // Mark remaining as complete with formatting
      const streamingIdx = messages.findLastIndex(m => m.isStreaming);
      if (streamingIdx >= 0) {
        messages[streamingIdx] = {
          ...messages[streamingIdx],
          content: formatMarkdownTables(messages[streamingIdx].content),
          isStreaming: false,
        };
      }
    } else if (chunk.type === 'HistoryCleared') {
      // Clear all messages when history is cleared
      messages.length = 0;
      messages.push({
        type: 'status',
        content: 'History cleared',
      });
    } else if (chunk.type === 'Error' && chunk.error) {
      // Remove empty streaming messages
      while (
        messages.length > 0 &&
        messages[messages.length - 1].type === 'assistant-text' &&
        messages[messages.length - 1].isStreaming &&
        !messages[messages.length - 1].content
      ) {
        messages.pop();
      }
      messages.push({
        type: 'status',
        content: `API Error: ${chunk.error}`,
      });
    } else if (chunk.type === 'Interrupted') {
      // Handle interruption
      while (
        messages.length > 0 &&
        messages[messages.length - 1].type === 'assistant-text' &&
        messages[messages.length - 1].isStreaming &&
        !messages[messages.length - 1].content
      ) {
        messages.pop();
      }

      let handledInterrupt = false;
      for (let i = messages.length - 1; i >= 0; i--) {
        const msg = messages[i];
        if (msg.type === 'tool-call' && msg.content.startsWith('●')) {
          if (!msg.content.includes('(select turn to /expand)')) {
            messages[i] = {
              ...msg,
              content: `${msg.content}\nL ⚠ Interrupted`,
            };
            handledInterrupt = true;
          }
          break;
        }
      }

      if (!handledInterrupt) {
        messages.push({
          type: 'status',
          content: '⚠ Interrupted',
        });
      }

      // Finalize streaming messages
      const lastAssistantIdx = messages.findLastIndex(
        m => m.type === 'assistant-text' && m.isStreaming
      );
      if (lastAssistantIdx >= 0) {
        messages[lastAssistantIdx] = {
          ...messages[lastAssistantIdx],
          isStreaming: false,
        };
      }
    }
  }

  return messages;
}

// ============================================================================
// Single Chunk Processing (for real-time streaming)
// ============================================================================

/**
 * Process a single streaming chunk and update conversation state.
 * Used for real-time streaming updates.
 *
 * @param chunk - Single StreamChunk to process
 * @param conversation - Current conversation array (will be mutated)
 * @param ctx - Processing context with formatters
 * @returns true if conversation was modified
 */
export function processStreamingChunk(
  chunk: StreamChunk,
  conversation: ConversationMessage[],
  ctx: ChunkProcessorContext
): boolean {
  if (!chunk) {
    return false;
  }

  if (chunk.type === 'SupervisorInput' && chunk.text) {
    const msg = processSupervisorInputChunk(chunk.text);
    conversation.push(msg);
    return true;
  }

  if (chunk.type === 'UserInput' && chunk.text) {
    conversation.push({
      type: 'user-input',
      content: chunk.text,
    });
    return true;
  }

  if (chunk.type === 'Text' && chunk.text) {
    const lastIdx = conversation.findLastIndex(
      m => m.type === 'assistant-text'
    );
    if (lastIdx >= 0 && conversation[lastIdx].isStreaming) {
      conversation[lastIdx] = {
        ...conversation[lastIdx],
        content: conversation[lastIdx].content + chunk.text,
      };
    } else {
      conversation.push({
        type: 'assistant-text',
        content: chunk.text || '',
        isStreaming: true,
      });
    }
    return true;
  }

  if (chunk.type === 'Thinking' && chunk.thinking) {
    appendThinking(conversation, chunk.thinking);
    return true;
  }

  if (chunk.type === 'ToolCall' && chunk.toolCall) {
    // Finalize any active thinking block before tool call
    finalizeThinkingBlock(conversation);

    const toolCall = chunk.toolCall;
    let argsDisplay = '';
    try {
      const parsedInput = JSON.parse(toolCall.input);
      if (typeof parsedInput === 'object' && parsedInput !== null) {
        argsDisplay = extractToolArgsDisplay(
          toolCall.name,
          parsedInput as Record<string, unknown>
        );
      }
    } catch {
      argsDisplay = toolCall.input;
    }

    // Mark streaming message as complete or remove if empty
    const streamingIdx = conversation.findLastIndex(m => m.isStreaming);
    if (streamingIdx >= 0) {
      if (conversation[streamingIdx].content.trim() === '') {
        conversation.splice(streamingIdx, 1);
      } else {
        conversation[streamingIdx] = {
          ...conversation[streamingIdx],
          isStreaming: false,
        };
      }
    }

    conversation.push({
      type: 'tool-call',
      content: ctx.formatToolHeader(toolCall.name, argsDisplay),
      toolCallId: toolCall.id,
    });
    return true;
  }

  if (chunk.type === 'ToolResult' && chunk.toolResult) {
    const result = chunk.toolResult;
    const sanitizedContent = result.content.replace(/\t/g, '  ');
    const toolResultContent = ctx.formatCollapsedOutput(sanitizedContent);

    // Find tool header and combine with result
    for (let i = conversation.length - 1; i >= 0; i--) {
      const msg = conversation[i];
      if (msg.type === 'tool-call' && msg.content.startsWith('●')) {
        const headerLine = msg.content.split('\n')[0];
        const hasContent = toolResultContent && toolResultContent.trim();
        conversation[i] = {
          ...msg,
          content: hasContent
            ? `${headerLine}\n${toolResultContent}`
            : headerLine,
          isError: result.isError,
        };
        break;
      }
    }

    // Add streaming placeholder for continuation
    conversation.push({
      type: 'assistant-text',
      content: '',
      isStreaming: true,
    });
    return true;
  }

  if (chunk.type === 'Done') {
    // Remove empty streaming messages
    while (
      conversation.length > 0 &&
      conversation[conversation.length - 1].type === 'assistant-text' &&
      conversation[conversation.length - 1].isStreaming &&
      !conversation[conversation.length - 1].content
    ) {
      conversation.pop();
    }
    // Mark remaining as complete
    const streamingIdx = conversation.findLastIndex(m => m.isStreaming);
    if (streamingIdx >= 0) {
      conversation[streamingIdx] = {
        ...conversation[streamingIdx],
        content: formatMarkdownTables(conversation[streamingIdx].content),
        isStreaming: false,
      };
    }
    return true;
  }

  if (chunk.type === 'Error' && chunk.error) {
    // Remove empty streaming messages
    while (
      conversation.length > 0 &&
      conversation[conversation.length - 1].type === 'assistant-text' &&
      conversation[conversation.length - 1].isStreaming &&
      !conversation[conversation.length - 1].content
    ) {
      conversation.pop();
    }
    conversation.push({
      type: 'status',
      content: `API Error: ${chunk.error}`,
    });
    return true;
  }

  if (chunk.type === 'HistoryCleared') {
    conversation.length = 0;
    conversation.push({
      type: 'status',
      content: 'History cleared',
    });
    return true;
  }

  return false;
}

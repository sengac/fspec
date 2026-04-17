/**
 * AgentView - Full-screen view for AI agent interactions
 *
 * Integrates codelet-napi native module into fspec's TUI to enable
 * AI-powered conversations within the terminal interface.
 *
 * Implements NAPI-003: Proper TUI Integration Using Existing Codelet Rust Infrastructure
 * - Uses the same streaming loop as codelet-cli (run_agent_stream)
 * - Supports Esc key interruption via session.interrupt()
 * - Full-screen view for maximum conversation space
 *
 * Implements NAPI-006: Session Persistence
 * - Shift+Arrow-Up/Down for command history navigation
 * - /search command for interactive history search
 * - /resume command for session selection
 */

import React, {
  useState,
  useEffect,
  useCallback,
  useRef,
  useMemo,
} from 'react';
import fs from 'fs';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { InputTransition } from './InputTransition';
import { TurnContentModal } from './TurnContentModal';
import { SlashCommandPalette } from './SlashCommandPalette';

import { FileSearchPopup } from './FileSearchPopup';
import { RoleBanner } from './RoleBanner';
import { ProviderSettingsScreen } from './ProviderSettingsScreen';
import { ModelSelectorScreen } from './ModelSelectorScreen';
import {
  messagesToLines,
  wrapMessageToLines,
} from '../utils/conversationUtils';
import {
  extractTokenStateFromChunks,
  calculateContextFillPercentage,
  // REFAC-007: persistTokenState removed - now handled by Rust
} from '../utils/tokenStateUtils';
import { calculatePaneWidth } from '../utils/textWrap';
import { useSlashCommandInput } from '../hooks/useSlashCommandInput';
import { useFileSearchInput } from '../hooks/useFileSearchInput';
import { useTerminalSize } from '../hooks/useTerminalSize';
import { useInputCompat, InputPriority } from '../input/index';
import {
  attachToSession,
  useSessionStreamManager,
} from '../hooks/useSessionStreamManager';
import {
  getSelectionSeparatorType,
  generateArrowBar,
} from '../utils/turnSelection';
import type {
  ConversationMessage,
  ConversationLine,
} from '../types/conversation';
import { getFspecUserDir } from '../../utils/config';
import { logger } from '../../utils/logger';
import {
  // REFAC-007: persistenceStoreMessageEnvelope removed - now handled by Rust
  // TUI-077: testProviderConnection, persistenceLoadSession, persistenceGetSessionMessageEnvelopes removed (unused after slash command cleanup)
  persistenceGetHistory,
  persistenceAddHistory,
  persistenceSearchHistory,
  persistenceListSessions,
  persistenceSetDataDirectory,
  persistenceRenameSession,
  persistenceCreateSessionWithProvider,
  persistenceDeleteSession,
  persistenceCleanupOrphanedMessages,
  getThinkingConfig,
  JsThinkingLevel,
  // TUI-075: Model-related NAPI functions removed (now in useModelSelectorState hook)
  sessionToggleDebug,
  sessionUpdateDebugMetadata,
  sessionSetDebugEnabled,
  toggleDebug,
  sessionSendInput,
  sessionGetMergedOutput,
  sessionInterrupt,
  sessionGetStatus,
  sessionManagerList,
  sessionManagerCreateWithId,
  // MODEL-005: sessionSetModel/Profile for propagating per-model limits after deferred creation
  sessionSetModel as napiSessionSetModel,
  sessionSetModelProfile as napiSessionSetModelProfile,
  // TUI-068: session destroy REMOVED - use destroySession from sessionService
  sessionSetPendingInput,
  sessionGetPendingInput,
  // WATCH-008: Supervisor management NAPI functions
  sessionGetSupervisors,
  sessionGetRole,
  sessionSetRole,
  // PAUSE-001: Pause resume/confirm functions
  sessionPauseResume,
  sessionPauseConfirm,
  sessionPauseTriple,
  // UX-002: Compaction progress polling for automatic compaction
  sessionGetCompactionProgress,
  // TUI-065: Clear session history
  sessionClearHistory,
  // BLOCK-004: Blocklist NAPI functions
  blocklistLoad,
  blocklistInit,
  // REFAC-008: sessionSendFspecResult removed - handled by GlobalSessionStreamManager
} from '@sengac/codelet-napi';
import {
  detectThinkingLevel,
  getThinkingLevelLabel,
  computeEffectiveThinkingLevel,
  hasDisableKeywords,
} from '../../utils/thinkingLevel';
// REFAC-008: fspecCallback removed - handled by GlobalSessionStreamManager
import {
  buildModelString,
  parseModelString,
  findSectionForPersistedModel,
} from '../utils/model-selection';
import { BlocklistListView, type BlocklistRule } from './BlocklistListView';
import { SessionHeader } from './SessionHeader';
import { SessionFooter } from './SessionFooter';
import type { TokenTracker } from '../utils/sessionHeaderUtils';
import { computeLineDiff, changesToDiffLines } from '../../git/diff-parser';
import { useCompaction } from '../hooks/useCompaction';
import { useWorkUnitContext } from '../hooks/useWorkUnitContext';
import { useDefaultThinkingLevel } from '../hooks/useDefaultThinkingLevel';
import { ThreeButtonDialog } from '../../components/ThreeButtonDialog';
import { ErrorDialog } from '../../components/ErrorDialog';
import { CreateSessionDialog } from '../../components/CreateSessionDialog';
import { ThinkingLevelDialog } from './ThinkingLevelDialog';
import { RoleDialog } from '../../components/RoleDialog';
import { formatMarkdownTables } from '../utils/markdown-table-formatter';
import { handleMergeWorktree } from '../handlers/mergeWorktreeHandler';
import { handlePersistentSessionStateChange } from '../handlers/persistentSessionStateHandler';
import type { ActionPrompt } from '../types/actionPrompt';
import {
  parseSupervisorPrefix,
  extractToolArgsDisplay,
  processStreamingChunk,
  type PendingToolCallInfo,
  type ChunkProcessorContext,
} from '../utils/chunkProcessor';
import {
  createThinkingUpdate,
  createFinalizationUpdate,
  appendThinkingBulk,
  finalizeThinkingBlock,
} from '../utils/thinkingBlockManager';
import { useFspecStore } from '../store/fspecStore';
import {
  useCurrentSessionId,
  useIsReadyForNewSession,
  useShouldAutoCreateSession,
  usePendingIsolatedSession,
  useIsIsolated,
  useWorktreePath,
  useShowCreateSessionDialog,
  useSessionActions,
} from '../store/sessionStore';
import {
  useProviderSections,
  useCurrentModel,
  useModelsInitialized,
  useModelStoreActions,
} from '../store/modelStore';
import type { ModelSelection } from '../types/provider';
import {
  useRustSessionState,
  // BRIDGE-012: manualAttach, manualDetach, getSessionChunks removed - replaced by global callback
} from '../hooks/useRustSessionState';
import { getRustStateSource } from '../hooks/rustStateSource';
import { useSessionNavigation } from '../hooks/useSessionNavigation';
import { useHitlInput } from '../hooks/useHitlInput';
import {
  createSession,
  createIsolatedSession,
  restoreSession,
  destroySession,
  attachToWorkUnit,
  detachFromWorkUnit,
  getAttachedWorkUnit,
} from '../services/sessionService';
import { applyPendingIsolationState } from '../services/globalSessionStreamManager';
import { initializeModels } from '../services/modelInitializationService';
import { selectModel } from '../services/modelSelectionService';
import { configureProfileEnvironment } from '../services/profileEnvironmentService';
// PROV-008: Import provider mapping from shared utility (DRY)
import { mapProviderIdToInternal } from '../utils/provider-mapping';
// PROV-057: Detect github-copilot model selection without credentials and
// route to the OAuth login flow instead of surfacing a "requires credentials"
// error toast.
import { shouldDispatchCopilotLogin } from '../utils/copilotLoginDispatch';
import { MOUSE_ENABLE, MOUSE_DISABLE, SGR_BUTTON, parseSgrMouse } from '../utils/mouseProtocol';

interface StreamChunk {
  type: string;
  text?: string;
  thinking?: string; // TOOL-010: Extended thinking content
  toolCall?: { id: string; name: string; input: string };
  toolResult?: { toolCallId: string; content: string; isError: boolean };
  // TOOL-011: Tool execution progress - streaming output from bash/shell tools
  toolProgress?: { toolCallId: string; toolName: string; outputChunk: string };
  status?: string;
  queuedInputs?: string[];
  tokens?: TokenTracker;
  contextFill?: { fillPercentage: number };
  error?: string;
  // Compaction result for CompactionComplete chunks
  compactionResult?: {
    compressionRatio: number;
    originalTokens: number;
    compactedTokens: number;
    turnsSummarized: number;
    turnsKept: number;
  };
  // Session state for SessionStateChange chunks
  state?: string;
  message?: string;
}

// NAPI-006: History entry from persistence
interface HistoryEntry {
  display: string;
  timestamp: string;
  project: string;
  sessionId: string;
  hasPastedContent: boolean;
}

// NAPI-006: Session manifest from persistence
interface SessionManifest {
  id: string;
  name: string;
  project: string;
  provider: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
}

// TUI-047: Extended session type for merged list (background + persisted)
interface MergedSession extends SessionManifest {
  isBackgroundSession: boolean;
  backgroundStatus: 'running' | 'idle' | null; // null = persisted-only
}

// TUI-047: Get status icon for session in resume list
const getSessionStatusIcon = (session: MergedSession): string => {
  if (session.isBackgroundSession) {
    return session.backgroundStatus === 'running' ? '🔄' : '⏸️';
  }
  return '💾';
};

export interface AgentViewProps {
  onExit: () => void;
  workUnitId?: string; // SESS-001: Work unit ID for session attachment
  initialSessionId?: string; // VIEWNV-001: Initial session ID to resume (from navigation)
}

// ConversationMessage and ConversationLine are imported from '../types/conversation'

/**
 * Process merged chunks into conversation messages for reattachment.
 * Used when attaching to a running/idle background session OR restoring from persistence.
 *
 * SOLID: Single source of truth for converting StreamChunks to ConversationMessages.
 * DRY: Unified code path for both background and persisted session resume.
 *
 * Features:
 * - Tracks ToolCall inputs to regenerate diffs for Edit/Write tools
 * - Populates fullContent field for TUI-043 expansion
 * - Handles web_search special arg display
 * - Applies markdown table formatting
 */
const processChunksToConversation = (
  chunks: StreamChunk[],
  formatToolHeaderFn: (name: string, args: string) => string,
  formatCollapsedOutputFn: (content: string) => string
): ConversationMessage[] => {
  const messages: ConversationMessage[] = [];

  // Track pending tool calls for Edit/Write diff regeneration
  const pendingToolCalls = new Map<string, PendingToolCallInfo>();

  for (const chunk of chunks) {
    if (chunk.type === 'UserInput' && chunk.text) {
      messages.push({
        type: 'user-input',
        content: chunk.text,
      });
    } else if (chunk.type === 'IncomingMessage' && chunk.text) {
      // WATCH-012: Handle supervisor input messages - parse prefix and format for display
      const supervisorInfo = parseSupervisorPrefix(chunk.text);
      if (supervisorInfo) {
        // Format content with role prefix (no emoji)
        const formattedContent = `[W] ${supervisorInfo.role}> ${supervisorInfo.content}`;
        messages.push({
          type: 'supervisor-input',
          content: formattedContent,
        });
      } else {
        // Fallback: if parsing fails, display raw message
        messages.push({
          type: 'supervisor-input',
          content: chunk.text,
        });
      }
    } else if (chunk.type === 'Text' && chunk.text) {
      // Find last assistant-text message to append to, or create new one
      const lastIdx = messages.findLastIndex(m => m.type === 'assistant-text');
      if (lastIdx >= 0 && messages[lastIdx].isStreaming) {
        messages[lastIdx].content += chunk.text;
      } else {
        messages.push({
          type: 'assistant-text',
          content: chunk.text,
          isStreaming: true,
        });
      }
    } else if (chunk.type === 'Thinking' && chunk.thinking) {
      appendThinkingBulk(messages, chunk.thinking);
    } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
      // Finalize any active thinking block before tool call
      finalizeThinkingBlock(messages);

      const toolCall = chunk.toolCall;
      let argsDisplay = '';
      let parsedInput: Record<string, unknown> = {};

      try {
        parsedInput = JSON.parse(toolCall.input) as Record<string, unknown>;
        argsDisplay = extractToolArgsDisplay(toolCall.name, parsedInput);
      } catch (err) {
        // Failed to parse tool call input JSON - this indicates malformed data from backend
        logger.error('Failed to parse tool call input as JSON:', err);
        argsDisplay = toolCall.input;
      }

      // Store for ToolResult processing (Edit/Write diff regeneration)
      pendingToolCalls.set(toolCall.id, {
        name: toolCall.name,
        input: parsedInput,
      });

      // Finalize streaming assistant message (remove if empty, or format and mark complete)
      const streamingIdx = messages.findLastIndex(m => m.isStreaming);
      if (streamingIdx >= 0) {
        if (messages[streamingIdx].content.trim() === '') {
          messages.splice(streamingIdx, 1);
        } else {
          // TUI-044: Apply markdown table formatting when finalizing
          messages[streamingIdx].content = formatMarkdownTables(
            messages[streamingIdx].content
          );
          messages[streamingIdx].isStreaming = false;
        }
      }
      messages.push({
        type: 'tool-call',
        content: formatToolHeaderFn(toolCall.name, argsDisplay),
        toolCallId: toolCall.id,
      });
    } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
      const result = chunk.toolResult;
      const sanitizedContent = result.content.replace(/\t/g, '  ');

      // Look up pending tool call for Edit/Write diff regeneration
      const pendingTool = pendingToolCalls.get(result.toolCallId);
      const toolNameLower = pendingTool?.name?.toLowerCase() || '';
      const inputObj = pendingTool?.input || {};

      let resultContent: string;
      let resultFullContent: string;

      // TUI-038: Regenerate diff for Edit/Write tools
      if (
        (toolNameLower === 'edit' || toolNameLower === 'replace') &&
        typeof inputObj.old_string === 'string' &&
        typeof inputObj.new_string === 'string'
      ) {
        // Edit tool - generate diff from old/new strings
        const diffLines = formatEditDiff(
          inputObj.old_string,
          inputObj.new_string
        );
        resultContent = formatDiffForDisplay(diffLines);
        resultFullContent = formatDiffForDisplay(diffLines, diffLines.length);
      } else if (
        (toolNameLower === 'write' || toolNameLower === 'write_file') &&
        typeof inputObj.content === 'string'
      ) {
        // Write tool - generate diff (all additions)
        const diffLines = formatWriteDiff(inputObj.content);
        resultContent = formatDiffForDisplay(diffLines);
        resultFullContent = formatDiffForDisplay(diffLines, diffLines.length);
      } else {
        // Normal tool - use collapsed output
        resultContent = formatCollapsedOutputFn(sanitizedContent);
        resultFullContent = formatFullOutput(sanitizedContent);
      }

      // Find tool header and combine with result
      for (let i = messages.length - 1; i >= 0; i--) {
        if (
          messages[i].type === 'tool-call' &&
          messages[i].content.startsWith('●')
        ) {
          const headerLine = messages[i].content.split('\n')[0];
          // Don't add newline if result is empty
          const hasContent = resultContent && resultContent.trim();
          messages[i].content = hasContent
            ? `${headerLine}\n${resultContent}`
            : headerLine;
          // TUI-043: Set fullContent for expansion
          messages[i].fullContent = hasContent
            ? `${headerLine}\n${resultFullContent}`
            : headerLine;
          messages[i].isError = result.isError;
          break;
        }
      }
      // Add streaming placeholder for continuation
      messages.push({
        type: 'assistant-text',
        content: '',
        isStreaming: true,
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
      const streamingIdx = messages.findLastIndex(m => m.isStreaming);
      if (streamingIdx >= 0) {
        // TUI-044: Apply markdown table formatting when finalizing on Done
        messages[streamingIdx].content = formatMarkdownTables(
          messages[streamingIdx].content
        );
        messages[streamingIdx].isStreaming = false;
      }
    } else if (chunk.type === 'Interrupted') {
      // Finalize and add interrupted marker
      const streamingIdx = messages.findLastIndex(m => m.isStreaming);
      if (streamingIdx >= 0) {
        if (messages[streamingIdx].content.trim() === '') {
          messages.splice(streamingIdx, 1);
        } else {
          // TUI-044: Apply markdown table formatting when finalizing on Interrupted
          messages[streamingIdx].content = formatMarkdownTables(
            messages[streamingIdx].content
          );
          messages[streamingIdx].isStreaming = false;
        }
      }
      messages.push({
        type: 'status',
        content: '⚠ Interrupted',
      });
    } else if (chunk.type === 'UserNotification') {
      // NAPI-010: User-facing notification - display in conversation
      // NET-001: "✓ Reconnected" or "✗ Reconnection failed" replaces prior
      // "⟳ Reconnecting..." for clean display on resume/attach
      const isReconnectionUpdate =
        chunk.message === '✓ Reconnected' ||
        chunk.message === '✗ Reconnection failed';
      if (isReconnectionUpdate) {
        const idx = messages.findLastIndex(
          m => m.type === 'status' && m.content === '⟳ Reconnecting...'
        );
        if (idx !== -1) {
          messages[idx].content = chunk.message;
        } else {
          messages.push({ type: 'status', content: chunk.message });
        }
      } else {
        messages.push({ type: 'status', content: chunk.message });
      }
    }
    // NAPI-010: SessionStateChange is intentionally NOT handled here
    // It's an internal state update, not a conversation message
  }

  return messages;
};

/**
 * Normalize model ID for local section matching within AgentView.
 *
 * INTENTIONALLY DIFFERENT from `extractModelIdForRegistry` in modelInitializationService:
 * - The service strips date suffixes (e.g., "claude-opus-4-5-20251101" → "claude-opus-4-5")
 *   for registry-based lookups where aliases are acceptable.
 * - THIS function preserves the full versioned Anthropic ID because AgentView passes
 *   the exact API model ID to the Anthropic API, and Anthropic requires the dated form.
 *   Stripping suffixes here would break Anthropic API requests.
 *
 * Examples (this function):
 *   "claude-sonnet-4-20250514" -> "claude-sonnet-4-20250514" (preserved — Anthropic needs exact ID)
 *   "gemini-2.5-pro-preview-06-05" -> "gemini-2.5-pro" (strip preview suffix)
 *   "gpt-4o" -> "gpt-4o" (no change)
 */
const normalizeModelIdForMatch = (apiModelId: string): string => {
  // For Anthropic models, we MUST preserve the full versioned ID (e.g., "claude-opus-4-5-20251101")
  // because Anthropic API requires the exact dated version, NOT aliases like "claude-opus-4-5"
  if (apiModelId.startsWith('claude-')) {
    return apiModelId; // Keep full ID including date suffix
  }

  // For other providers (e.g., Gemini), remove preview suffixes
  return apiModelId.replace(/-preview-\d{2}-\d{2}$/, '');
};

// TUI-037: Claude Code style tool display helpers
const STREAMING_WINDOW_SIZE = 10; // Number of lines visible during streaming
const COLLAPSED_LINES = 8; // Number of lines visible when collapsed for normal output
const DIFF_COLLAPSED_LINES = 25; // Number of lines visible when collapsed for diff output (like Claude Code)

/**
 * Format tool header in Claude Code style: ● ToolName(args)
 */
const formatToolHeader = (toolName: string, args: string): string => {
  return `● ${toolName}(${args})`;
};

/**
 * Format output with tree connector: L on first line, indent on rest
 * Creates visual tree structure like:
 *   L first line
 *     second line
 *     third line
 */
const formatWithTreeConnectors = (content: string): string => {
  // Don't add tree connectors for empty content
  if (!content || !content.trim()) {
    return '';
  }
  const lines = content.split('\n');
  return lines
    .map((line, i) => {
      if (i === 0) return `L ${line}`; // First line gets L prefix
      return `  ${line}`; // Subsequent lines get indent
    })
    .join('\n');
};

/**
 * Format collapsed output with expand indicator
 */
const formatCollapsedOutput = (
  content: string,
  visibleLines: number = COLLAPSED_LINES
): string => {
  const lines = content.split('\n');
  if (lines.length <= visibleLines) {
    return formatWithTreeConnectors(content);
  }
  const visible = lines.slice(0, visibleLines);
  const remaining = lines.length - visibleLines;
  // TUI-045: Updated hint text for modal-based viewing
  const collapsedContent = `${visible.join('\n')}\n... +${remaining} lines (Enter to view full)`;
  return formatWithTreeConnectors(collapsedContent);
};

/**
 * TUI-043: Format full output without truncation (for expanded view)
 */
const formatFullOutput = (content: string): string => {
  return formatWithTreeConnectors(content);
};

/**
 * Create streaming window - keep only last N lines
 */
const createStreamingWindow = (
  content: string,
  windowSize: number = STREAMING_WINDOW_SIZE
): string => {
  const lines = content.split('\n');

  if (lines.length <= windowSize) {
    return content;
  }
  const result = lines.slice(-windowSize).join('\n');

  return result;
};

// TUI-038: Diff view color constants matching FileDiffViewer
const DIFF_COLORS = {
  removed: '#8B0000', // Dark red
  added: '#006400', // Dark green
};

/**
 * TUI-038: Format diff output for Edit tool (old_string -> new_string)
 * Returns formatted content with color markers for each line
 */
interface DiffOutputLine {
  content: string;
  color: string | null;
  type: 'context' | 'added' | 'removed';
}

const formatEditDiff = (
  oldString: string,
  newString: string
): DiffOutputLine[] => {
  const changes = computeLineDiff(oldString, newString);
  const diffLines = changesToDiffLines(changes);
  return diffLines.map(line => ({
    content: line.content,
    type: line.type,
    color:
      line.type === 'removed'
        ? DIFF_COLORS.removed
        : line.type === 'added'
          ? DIFF_COLORS.added
          : null,
  }));
};

/**
 * TUI-038: Format diff output for Write tool (new file = all additions)
 */
const formatWriteDiff = (content: string): DiffOutputLine[] => {
  const lines = content.split('\n');
  return lines.map(line => ({
    content: `+${line}`,
    type: 'added' as const,
    color: DIFF_COLORS.added,
  }));
};

/**
 * TUI-038: Convert diff output lines to display format with tree connectors
 *
 * Shows only the changed lines and minimal context (3 lines before/after changes).
 * This provides a focused view of what actually changed, similar to unified diff format.
 *
 * Format:
 * - Line numbers reflect actual position in the file (with startLine offset)
 * - Only shows context around actual changes
 * - "..." indicates skipped context lines
 * - Format: "2513 [R]- content" for removed, "2513 [A]+ content" for added
 * - Context lines: "2535   content" (all dim)
 *
 * @param diffLines - Array of diff output lines
 * @param visibleLines - Maximum lines to show before collapsing
 * @param startLine - Starting line number in the original file (1-based, default 1)
 */
const formatDiffForDisplay = (
  diffLines: DiffOutputLine[],
  visibleLines: number = DIFF_COLLAPSED_LINES,
  startLine: number = 1
): string => {
  // Find lines that have actual changes (not context)
  const changedIndices: number[] = [];
  diffLines.forEach((line, idx) => {
    if (line.type === 'added' || line.type === 'removed') {
      changedIndices.push(idx);
    }
  });

  // Calculate max line number for width padding (considering startLine offset)
  const maxLineNum = startLine + diffLines.length - 1;
  const lineNumWidth = Math.max(String(maxLineNum).length, 3);

  // If no changes, just show collapsed context
  if (changedIndices.length === 0) {
    const formattedLines = diffLines.slice(0, visibleLines).map((line, idx) => {
      const lineNum = String(startLine + idx).padStart(lineNumWidth, ' ');
      const restOfLine = line.content.slice(1);
      return `${lineNum}   ${restOfLine}`;
    });
    if (diffLines.length > visibleLines) {
      formattedLines.push(
        `... +${diffLines.length - visibleLines} lines (select turn to /expand)`
      );
    }
    return formatWithTreeConnectors(formattedLines.join('\n'));
  }

  // Build set of indices to show: changed lines + 3 lines of context around each change
  const CONTEXT_LINES = 3;
  const indicesToShow = new Set<number>();

  changedIndices.forEach(idx => {
    // Add the changed line
    indicesToShow.add(idx);
    // Add context before
    for (let i = Math.max(0, idx - CONTEXT_LINES); i < idx; i++) {
      indicesToShow.add(i);
    }
    // Add context after
    for (
      let i = idx + 1;
      i <= Math.min(diffLines.length - 1, idx + CONTEXT_LINES);
      i++
    ) {
      indicesToShow.add(i);
    }
  });

  // Convert to sorted array
  const sortedIndices = Array.from(indicesToShow).sort((a, b) => a - b);

  // Format the lines, adding "..." for gaps
  const outputLines: string[] = [];
  let lastShownIdx = -1;

  for (const idx of sortedIndices) {
    // Add "..." if there's a gap
    if (lastShownIdx >= 0 && idx > lastShownIdx + 1) {
      const skipped = idx - lastShownIdx - 1;
      outputLines.push(
        `${''.padStart(lineNumWidth, ' ')} ... (${skipped} lines)`
      );
    }

    const line = diffLines[idx];
    const lineNum = String(startLine + idx).padStart(lineNumWidth, ' ');
    const restOfLine = line.content.slice(1);

    if (line.color === DIFF_COLORS.removed) {
      outputLines.push(`${lineNum} [R]- ${restOfLine}`);
    } else if (line.color === DIFF_COLORS.added) {
      outputLines.push(`${lineNum} [A]+ ${restOfLine}`);
    } else {
      outputLines.push(`${lineNum}   ${restOfLine}`);
    }

    lastShownIdx = idx;
  }

  // Add trailing "..." if there are more lines after
  if (lastShownIdx < diffLines.length - 1) {
    const remaining = diffLines.length - 1 - lastShownIdx;
    outputLines.push(
      `${''.padStart(lineNumWidth, ' ')} ... (${remaining} lines)`
    );
  }

  // Apply collapse logic if still too many lines
  if (outputLines.length <= visibleLines) {
    return formatWithTreeConnectors(outputLines.join('\n'));
  }

  const visible = outputLines.slice(0, visibleLines);
  const remaining = outputLines.length - visibleLines;
  const collapsedContent = `${visible.join('\n')}\n... +${remaining} lines (select turn to /expand)`;
  return formatWithTreeConnectors(collapsedContent);
};

/**
 * Calculate the starting line number of an edit in a file.
 *
 * Since the edit has already been applied by the time the TUI receives the event,
 * we search for new_string (which is now in the file) rather than old_string.
 *
 * Returns 1 if file can't be read or string not found.
 */
const calculateStartLine = (
  filePath: string | undefined,
  oldString: string | undefined,
  newString: string | undefined
): number => {
  if (!filePath) return 1;

  try {
    const fileContent = fs.readFileSync(filePath, 'utf-8');

    // The edit has already been applied, so search for new_string first
    if (newString) {
      const idx = fileContent.indexOf(newString);
      if (idx !== -1) {
        const beforeMatch = fileContent.substring(0, idx);
        const lineNumber = (beforeMatch.match(/\n/g) || []).length + 1;
        return lineNumber;
      }
    }

    // Fallback: try old_string (in case edit hasn't been applied yet)
    if (oldString) {
      const idx = fileContent.indexOf(oldString);
      if (idx !== -1) {
        const beforeMatch = fileContent.substring(0, idx);
        const lineNumber = (beforeMatch.match(/\n/g) || []).length + 1;
        return lineNumber;
      }
    }

    return 1;
  } catch (err) {
    // Failed to read file or calculate line number - this indicates file system issues
    logger.error(`Failed to calculate start line for file ${filePath}:`, err);
    return 1;
  }
};

export const AgentView: React.FC<AgentViewProps> = ({
  onExit,
  workUnitId,
  initialSessionId,
}) => {
  // Use useTerminalSize for reactive, deduplicated resize tracking.
  // Ink's core resize handler only recalculates Yoga layout — it does NOT
  // trigger React re-renders. useTerminalSize provides the explicit resize
  // subscription, and its functional setState deduplicates: if the terminal
  // size hasn't actually changed, no re-render occurs. This is critical
  // because terminalWidth is a dependency of the conversationLines useMemo,
  // which re-wraps ALL messages on width change.
  const { width: terminalWidth, height: terminalHeight } = useTerminalSize();

  // NAPI-009: Removed session state - we use SessionManager background sessions exclusively
  const [error, setError] = useState<string | null>(null);
  const [inputValue, setInputValue] = useState('');
  // TUI-049: Skip input animation when switching sessions
  const [skipInputAnimation, _setSkipInputAnimation] = useState(false);

  // TUI-050: Ref for slash command executor (set after handleSubmitWithCommand is defined)
  const executeSlashCommandRef = useRef<(cmd: string) => void>();

  // GIT-038: Flag for auto-submitting conflict resolution message to Rust session
  const pendingAutoSubmitRef = useRef(false);

  const [conversation, setConversation] = useState<ConversationMessage[]>([]);
  const [tokenUsage, setTokenUsage] = useState<TokenTracker>({
    inputTokens: 0,
    outputTokens: 0,
  });
  const [currentProvider, setCurrentProvider] = useState<string>('');
  const [availableProviders, setAvailableProviders] = useState<string[]>([]);
  const [showProviderSelector, setShowProviderSelector] = useState(false);
  const [selectedProviderIndex, setSelectedProviderIndex] = useState(0);
  const [isDebugEnabled, setIsDebugEnabled] = useState(false); // AGENT-021 - local state synced with Rust on toggle
  const [isTurnSelectMode, setIsTurnSelectMode] = useState(false); // TUI-042: Turn selection mode toggle (replaces TUI-041 line selection)
  // TUI-045: Modal state for full turn viewing (replaces expandedMessageIndices)
  const [showTurnModal, setShowTurnModal] = useState(false);
  const [modalMessageIndex, setModalMessageIndex] = useState<number | null>(
    null
  );
  const virtualListSelectionRef = useRef<{ selectedIndex: number }>({
    selectedIndex: 0,
  }); // TUI-043: Ref to get selected index from VirtualList
  // NAPI-009: sessionRef removed - we use SessionManager exclusively now

  // TUI-038: Store pending Edit/Write tool inputs for diff display
  interface PendingToolDiff {
    toolName: string;
    toolCallId: string;
    filePath?: string; // Path to the file being edited
    oldString?: string; // For Edit tool
    newString?: string; // For Edit tool
    content?: string; // For Write tool
    startLine?: number; // Pre-calculated line number for Edit tool
  }
  const pendingToolDiffsRef = useRef<Map<string, PendingToolDiff>>(new Map());

  // TUI-075: Model selection state from shared store
  const currentModel = useCurrentModel();
  const providerSections = useProviderSections();
  const modelsInitialized = useModelsInitialized();
  const { setCurrentModel } = useModelStoreActions();

  // TUI-034: Local screen visibility state
  const [showModelSelector, setShowModelSelector] = useState(false);
  const [showSettingsTab, setShowSettingsTab] = useState(false);
  // PROV-057: When the user picks a github-copilot/* model with no credentials
  // we set this flag and open the settings tab; ProviderSettingsScreen reads
  // it to auto-dispatch the Copilot OAuth login flow on mount.
  const [autoStartCopilotLogin, setAutoStartCopilotLogin] = useState(false);

  // SESS-001: Session attachment state and actions from store
  // TUI-068: attachToWorkUnit and detachFromWorkUnit imported from sessionService
  // TUI-069: getAttachedWorkUnit imported from sessionService (completes facade)
  const getAttachedSession = useFspecStore(state => state.getAttachedSession);

  // PERF-002: Compaction with retry logic hook
  const compaction = useCompaction();

  // UX-002: Ref for compaction functions to avoid stale closures in stream callbacks
  // The useCompaction hook returns new function references on each render, but stream
  // callbacks (handleStreamChunk) capture the initial reference. Using a ref ensures
  // callbacks always have access to the latest functions.
  const compactionRef = useRef(compaction);
  useEffect(() => {
    compactionRef.current = compaction;
  }, [compaction]);

  // NAPI-006: History navigation state
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1); // -1 means not navigating history
  const [savedInput, setSavedInput] = useState(''); // Save current input when navigating history

  // NAPI-006: Search mode state (Ctrl+R)
  const [isSearchMode, setIsSearchMode] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<HistoryEntry[]>([]);
  const [searchResultIndex, setSearchResultIndex] = useState(0);

  // NAPI-006: Session persistence state
  // VIEWNV-001: Session state from Zustand store (atomic state machine transitions)
  const currentSessionId = useCurrentSessionId();
  const isReadyForNewSession = useIsReadyForNewSession();
  const shouldAutoCreateSession = useShouldAutoCreateSession();
  const pendingIsolatedSession = usePendingIsolatedSession();
  const isIsolated = useIsIsolated();
  const worktreePath = useWorktreePath();
  const showCreateSessionDialog = useShowCreateSessionDialog();
  const {
    activateSession,
    prepareForNewSession,
    clearAutoCreateRequest,
    closeCreateSessionDialog,
    setCurrentWorkUnit,
  } = useSessionActions();
  // Ref to track current session ID for use in callbacks without stale closures
  const currentSessionIdRef = useRef<string | null>(null);
  const currentProjectRef = useRef<string>(process.cwd());

  // TUI-091: Footer state (CWD + git info) is now handled by useFooterState hook
  // inside SessionFooter component — no polling or NAPI calls needed here.

  // TUI-059: Work unit context hook - sets context in Rust when entering AgentView
  // This enables environment info to display "Current work unit: ID" and
  // status change notifications when LLM updates a different work unit
  // NOTE: Must be after useCurrentSessionId() since it depends on currentSessionId
  useWorkUnitContext({
    sessionId: currentSessionId,
    workUnitId,
  });

  // BRIDGE-013: Persistent chunk handler for bridge/supervisor input display.
  // Always registered when viewing a session. Skips when handleSubmit's handler
  // is active (sessionCleanupRef.current set) to avoid duplicate updates.
  // TUI-066: Also handles SessionStateChange with Cleared state from bridge /clear
  //
  // NOTE: We use a ref to access setContextFillPercentage since it's defined later
  // in the component and would cause a TDZ error if included in dependencies.
  // This is safe because React setters are stable and never change.
  const setContextFillPercentageRef =
    useRef<React.Dispatch<React.SetStateAction<number>>>(null);
  // Same TDZ avoidance pattern for setCompactionReduction — needed by
  // persistentChunkHandler to handle CompactionComplete chunks from /compact flow.
  const setCompactionReductionRef =
    useRef<React.Dispatch<React.SetStateAction<number | null>>>(null);
  // TDZ avoidance ref for refreshRustState — needed by persistentChunkHandler
  // to refresh isLoading after CompactionComplete so the UI shows "Thinking..." instead of idle.
  const refreshRustStateRef = useRef<(targetSessionId?: string | null) => void>(
    () => {}
  );

  // Shared helper for handling CompactionComplete chunks.
  // Extracted to avoid duplicating the endCompaction + refreshRustState pattern
  // across persistentChunkHandler, handleSubmit's inner handler, and handleStreamChunk.
  const handleCompactionComplete = useCallback(
    (
      result: { compressionRatio: number } | undefined,
      sessionId: string | null | undefined,
    ) => {
      // Defensive: compactionResult is always present for CompactionComplete chunks
      // per NAPI type system, but the local StreamChunk interface marks it optional
      // since it's a flat union. Guard to prevent runtime crash if undefined.
      if (result) {
        setCompactionReductionRef.current?.(Math.round(result.compressionRatio));
      }
      compactionRef.current.endCompaction();
      // Refresh Rust state so isLoading reflects the current session status.
      // After endCompaction() clears isCompacting, the UI needs isLoading=true if the
      // agent loop is still running (status=Running from CompactionContinuing).
      if (sessionId) {
        refreshRustStateRef.current(sessionId);
      }
    },
    []
  );

  const persistentChunkHandler = useCallback(
    (chunk: StreamChunk) => {
      if (!chunk || sessionCleanupRef.current) {
        return;
      }

      // TUI-066: Handle SessionStateChange with Cleared state (from bridge /clear or TUI /clear)
      // Also handle Compacting state for manual /compact command flow.
      // When /compact returns early without setting up a streaming handler, chunks from
      // the agent_loop (which processes the compaction instruction) arrive here.
      // BUG-101: Extracted to handlePersistentSessionStateChange for testability.
      if (chunk.type === 'SessionStateChange') {
        handlePersistentSessionStateChange(chunk.state, {
          resetConversation: () => {
            setConversation([]);
            setTokenUsage({ inputTokens: 0, outputTokens: 0 });
            if (setContextFillPercentageRef.current) {
              setContextFillPercentageRef.current(0);
            }
          },
          startCompaction: (trigger, sessionId, progress) => {
            compactionRef.current.startCompaction(trigger, sessionId, progress);
          },
          getCompactionProgress: (sessionId) =>
            sessionGetCompactionProgress(sessionId),
          refreshRustState: (sessionId) =>
            refreshRustStateRef.current(sessionId),
          getCurrentSessionId: () => currentSessionIdRef.current,
        });
        return;
      }

      // Handle CompactionComplete when no streaming handler is active.
      // Emitted once by emit_post_injection_events (in on_injected) during the stream.
      // Critical for /compact command flow where handleSubmit returns early and
      // chunks arrive via this persistent handler.
      if (chunk.type === 'CompactionComplete') {
        handleCompactionComplete(
          chunk.compactionResult,
          currentSessionIdRef.current,
        );
        return;
      }

      const ctx: ChunkProcessorContext = {
        formatToolHeader,
        formatCollapsedOutput,
        pendingToolCalls: pendingToolDiffsRef.current
          ? new Map(
              Array.from(pendingToolDiffsRef.current.entries()).map(
                ([id, diff]) => [
                  id,
                  { name: diff.toolName, input: {} } as PendingToolCallInfo,
                ]
              )
            )
          : new Map(),
      };

      setConversation(prev => {
        const updated = [...prev];
        processStreamingChunk(chunk, updated, ctx);
        return updated;
      });
    },
    [setConversation, setTokenUsage]
  );

  // BRIDGE-013: Register persistent handler - unregisters on session change or unmount
  useSessionStreamManager(currentSessionId, persistentChunkHandler);

  // NAPI-003: Resume mode state (session selection overlay)
  const [isResumeMode, setIsResumeMode] = useState(false);
  // TUI-050: Trigger state for resume mode initialization (avoids TDZ with handleResumeMode)
  const [triggerResumeModeInit, setTriggerResumeModeInit] = useState(false);
  // TUI-047: Changed to MergedSession to support background session info
  const [availableSessions, setAvailableSessions] = useState<MergedSession[]>(
    []
  );
  const [resumeSessionIndex, setResumeSessionIndex] = useState(0);
  const [resumeScrollOffset, setResumeScrollOffset] = useState(0);

  // TUI-040: Delete session dialog state
  const [showSessionDeleteDialog, setShowSessionDeleteDialog] = useState(false);

  // TUI-092: Exit confirmation modal state (Detach/Close Session/Cancel)
  const [showExitConfirmation, setShowExitConfirmation] = useState(false);

  // TUI-054: Thinking level dialog state
  const [showThinkingLevelDialog, setShowThinkingLevelDialog] = useState(false);

  // AMGR-012: Role dialog state
  const [showRoleDialog, setShowRoleDialog] = useState(false);

  // GIT-037: Generic action prompt state for deferred user confirmation
  const [actionPrompt, setActionPrompt] = useState<ActionPrompt | null>(null);

  // BLOCK-004: Blocklist management state
  const [isBlocklistMode, setIsBlocklistMode] = useState(false);
  const [blocklistRules, setBlocklistRules] = useState<BlocklistRule[]>([]);
  const [disabledBlocklistRules, setDisabledBlocklistRules] = useState<
    Set<string>
  >(new Set());

  // TUI-050: Slash command palette with clean input handling
  // Hook is called here after all state that affects its `disabled` prop is defined
  const slashCommand = useSlashCommandInput({
    inputValue,
    onInputChange: setInputValue,
    onExecuteCommand: cmd => executeSlashCommandRef.current?.(cmd),
    // Disable palette when other overlays/modes are active (TUI-054: add thinking dialog, BLOCK-004: add blocklist)
    disabled:
      isResumeMode ||
      isBlocklistMode ||
      showModelSelector ||
      showSettingsTab ||
      showThinkingLevelDialog,
  });

  // TUI-031: Tok/s display (calculated in Rust, just displayed here)
  const [displayedTokPerSec, setDisplayedTokPerSec] = useState<number | null>(
    null
  );
  const [lastChunkTime, setLastChunkTime] = useState<number | null>(null);

  // TUI-033: Context window fill percentage (received from Rust via ContextFillUpdate event)
  const [contextFillPercentage, setContextFillPercentage] = useState<number>(0);

  // TUI-066: Set ref for persistentChunkHandler to access setContextFillPercentage
  // This avoids TDZ since persistentChunkHandler is defined before this state
  useEffect(() => {
    setContextFillPercentageRef.current = setContextFillPercentage;
  }, [setContextFillPercentage]);

  // TUI-049: Centralized helper for updating token state from streaming chunks
  // This ensures consistent handling of TokenUpdate and ContextFillUpdate across all chunk handlers
  const updateTokenStateFromChunk = useCallback((chunk: StreamChunk) => {
    if (chunk.type === 'TokenUpdate' && chunk.tokens) {
      setTokenUsage(chunk.tokens);
      if (
        chunk.tokens.tokensPerSecond !== undefined &&
        chunk.tokens.tokensPerSecond !== null
      ) {
        setDisplayedTokPerSec(chunk.tokens.tokensPerSecond);
        setLastChunkTime(Date.now());
      }
    } else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
      setContextFillPercentage(chunk.contextFill.fillPercentage);
    }
  }, []);

  // Rust state subscription via useSyncExternalStore
  // CRITICAL: This must be declared BEFORE any useEffect hooks that use displayIsLoading
  const { snapshot: rustSnapshot, refresh: refreshRustState } =
    useRustSessionState(currentSessionId);

  // TUI-075: Default thinking level - applies to every session when it becomes active
  const {
    defaultLevel: defaultThinkingLevel,
    setDefault: setDefaultThinkingLevel,
  } = useDefaultThinkingLevel({
    sessionId: currentSessionId,
    refreshRustState,
  });

  // Helper to find model details from provider sections (Single Responsibility Principle)
  const findModelInProviders = useCallback(
    (providerId: string, modelId: string) => {
      const section = providerSections.find(s => s.providerId === providerId);
      return section?.models.find(
        m => normalizeModelIdForMatch(m.id) === modelId
      );
    },
    [providerSections]
  );

  // Derive display values from Rust snapshot + local state fallbacks
  const rustModelInfo = useMemo(() => {
    // Helper to create model info shape (DRY principle - eliminates repeated object structure)
    const createModelInfo = (
      modelId: string,
      reasoning = false,
      hasVision = false,
      contextWindow = 0,
      compactionThreshold?: number
    ) => ({ modelId, reasoning, hasVision, contextWindow, compactionThreshold });

    // Get fallback model ID from local state
    const localModelId =
      currentModel?.displayName || currentModel?.modelId || currentProvider;

    // No session - use local state with full model info if available
    if (!currentSessionId) {
      return createModelInfo(
        localModelId,
        currentModel?.reasoning ?? false,
        currentModel?.hasVision ?? false,
        currentModel?.contextWindow ?? 0
      );
    }

    // Has session with Rust model data
    const rustModel = rustSnapshot.model;
    if (rustModel?.modelId) {
      // CTX-006: Use Rust-resolved context_window when available (single source of truth).
      // This ensures the displayed value matches what the compaction engine uses.
      const rustContextWindow = rustModel.contextWindow;
      // CTX-009: Extract compaction threshold for SessionHeader badge
      const rustCompactionThreshold = rustModel.compactionThreshold ?? undefined;

      const model = findModelInProviders(
        rustModel.providerId,
        rustModel.modelId
      );
      if (model) {
        return createModelInfo(
          model.name,
          model.reasoning,
          model.hasVision,
          // CTX-006: Prefer Rust-resolved context_window over models.dev data
          rustContextWindow ?? model.contextWindow,
          rustCompactionThreshold
        );
      }
      // Rust model exists but not found in providers - use Rust data as fallback
      return createModelInfo(rustModel.modelId, false, false, rustContextWindow ?? 0, rustCompactionThreshold);
    }

    // Fallback to local state with full model info if available
    return createModelInfo(
      localModelId,
      currentModel?.reasoning ?? false,
      currentModel?.hasVision ?? false,
      currentModel?.contextWindow ?? 0
    );
  }, [
    currentSessionId,
    currentProvider,
    currentModel,
    rustSnapshot.model,
    findModelInProviders,
  ]);

  // Destructure all display values at once (cleaner than individual assignments)
  const {
    modelId: displayModelId,
    reasoning: displayReasoning,
    hasVision: displayHasVision,
    contextWindow: displayContextWindow,
    compactionThreshold: displayCompactionThreshold,
  } = rustModelInfo;

  // VIEWNV-001: Calculate session number (1-based index of current session in list)
  // This helps users identify which session they're in when switching with Shift+Left/Right
  const sessionNumber = useMemo(() => {
    if (!currentSessionId) {
      return undefined;
    }

    const allSessions = sessionManagerList();
    const index = allSessions.findIndex(s => s.id === currentSessionId);
    return index >= 0 ? index + 1 : undefined;
  }, [currentSessionId]);

  // Sync work unit info to sessionStore for SessionHeader display
  // TUI-060: Inline work unit lookup - don't use useMemo as a dependency
  // TUI-069: Use getAttachedWorkUnit from sessionService facade
  const workUnits = useFspecStore(state => state.workUnits);
  useEffect(() => {
    const attachedWorkUnitId = currentSessionId
      ? getAttachedWorkUnit(currentSessionId)
      : null;

    if (attachedWorkUnitId) {
      const workUnit = workUnits.find(wu => wu.id === attachedWorkUnitId);
      setCurrentWorkUnit(attachedWorkUnitId, workUnit?.status ?? null);
    } else {
      setCurrentWorkUnit(null, null);
    }
  }, [currentSessionId, workUnits, setCurrentWorkUnit]);

  // Extract remaining display state from Rust snapshot
  const displayIsLoading = rustSnapshot.isLoading;
  const rustTokens = rustSnapshot.tokens;
  const displayIsDebugEnabled = rustSnapshot.isDebugEnabled || isDebugEnabled;
  const displayIsPaused = rustSnapshot.isPaused;
  const displayPauseInfo = rustSnapshot.pauseInfo;
  const displayHitlRequest = rustSnapshot.hitlRequest;

  // Triple pause selection state: 0 = Allow Once, 1 = Allow Session, 2 = Deny
  const [triplePauseSelection, setTriplePauseSelection] = useState(0);

  // BUG-118: HITL input state — extracted to composable hook
  const hitlInput = useHitlInput({
    sessionId: currentSessionId,
    isPaused: displayIsPaused,
    hitlRequest: displayHitlRequest,
    inputValue,
    clearInputValue: () => setInputValue(''),
  });

  // Reset triple pause selection when pause ends or changes to non-triple
  useEffect(() => {
    if (!displayIsPaused || displayPauseInfo?.kind !== 'triple') {
      setTriplePauseSelection(0);
    }
  }, [displayIsPaused, displayPauseInfo?.kind]);

  // TUI-044: Compaction notification indicator (shows in percentage indicator for 10 seconds)
  const [compactionReduction, setCompactionReduction] = useState<number | null>(
    null
  );

  // Set ref for persistentChunkHandler to access setCompactionReduction
  // Same TDZ avoidance pattern — persistentChunkHandler needs this for CompactionComplete
  // Must be placed AFTER the useState declaration to avoid TDZ errors.
  useEffect(() => {
    setCompactionReductionRef.current = setCompactionReduction;
  }, [setCompactionReduction]);

  // Set ref for persistentChunkHandler to access refreshRustState
  // After CompactionComplete, we need to refresh Rust state so isLoading reflects
  // the current Running status (from CompactionContinuing).
  useEffect(() => {
    refreshRustStateRef.current = refreshRustState;
  }, [refreshRustState]);

  // TOOL-010: Detected thinking level (for UI indicator)
  const [detectedThinkingLevel, setDetectedThinkingLevel] = useState<
    number | null
  >(null);

  // PERF-002: Incremental line computation cache
  // Cache wrapped lines per message to avoid recomputing entire conversation
  // Line wrapping via wrapMessageToLines is expensive (visual width calculation for each char)
  interface CachedMessageLines {
    content: string;
    isStreaming: boolean;
    isThinking: boolean;
    terminalWidth: number;
    lines: ConversationLine[];
  }
  const lineCacheRef = useRef<Map<number, CachedMessageLines>>(new Map());

  // TUI-043: Ref to store current conversationLines for use in callbacks (avoids stale closure)
  const conversationLinesRef = useRef<ConversationLine[]>([]);

  // PERF-003: Previously used useDeferredValue here, but Ink uses LegacyRoot
  // (synchronous rendering) which means deferred values always lag one render behind.
  // This caused Write/Edit tool results to not display until a forced re-render
  // (e.g. session switch). The line cache (lineCacheRef) already handles perf.
  const deferredConversation = conversation;

  // TUI-055: File search popup following the EXACT same architecture as slash commands
  // GIT-033: Pass sessionId for worktree path resolution in isolated sessions
  const fileSearch = useFileSearchInput({
    inputValue,
    onInputChange: setInputValue,
    terminalWidth,
    // Disable popup when other overlays/modes are active (BLOCK-004: add blocklist)
    disabled:
      isResumeMode ||
      isBlocklistMode ||
      showModelSelector ||
      showSettingsTab ||
      showThinkingLevelDialog,
    sessionId: currentSessionId ?? undefined,
  });

  // TUI-074: Settings tab scrolling now handled by ProviderSettingsScreen

  // Resume mode scrolling (each session takes 2 lines: name + details)
  const resumeVisibleHeight = Math.max(
    1,
    Math.floor((terminalHeight - 10) / 2)
  );

  // Keep selected resume session visible by adjusting scroll offset
  useEffect(() => {
    if (!isResumeMode) return;
    if (resumeSessionIndex < resumeScrollOffset) {
      setResumeScrollOffset(resumeSessionIndex);
    } else if (resumeSessionIndex >= resumeScrollOffset + resumeVisibleHeight) {
      setResumeScrollOffset(resumeSessionIndex - resumeVisibleHeight + 1);
    }
  }, [
    resumeSessionIndex,
    resumeScrollOffset,
    resumeVisibleHeight,
    isResumeMode,
  ]);

  // Reset scroll when resume mode opens
  useEffect(() => {
    if (isResumeMode) {
      setResumeScrollOffset(0);
    }
  }, [isResumeMode]);

  // TUI-074: Settings filtering and scroll now handled by ProviderSettingsScreen

  // TUI-051: Sync input to Rust on every change (real-time persistence)
  useEffect(() => {
    if (currentSessionId && inputValue !== undefined) {
      try {
        sessionSetPendingInput(currentSessionId, inputValue);
      } catch (err) {
        // Session may not exist or may have been detached - this indicates session management issues
        logger.error('Failed to set pending input:', err);
      }
    }
  }, [currentSessionId, inputValue]);

  // Enable mouse tracking for model selector and settings tab scrolling
  useEffect(() => {
    if (showModelSelector || showSettingsTab || isResumeMode) {
      // BUG-131: Enable SGR mouse button event tracking (clicks and scroll wheel)
      process.stdout.write(MOUSE_ENABLE);
      return () => {
        // Disable mouse tracking on unmount or when screens close
        process.stdout.write(MOUSE_DISABLE);
      };
    }
  }, [showModelSelector, showSettingsTab, isResumeMode]);

  // TUI-031: Hide tok/s after 10 seconds of no chunks
  useEffect(() => {
    if (!displayIsLoading || lastChunkTime === null) return;
    const timeout = setTimeout(() => {
      setDisplayedTokPerSec(null);
    }, 10000);
    return () => clearTimeout(timeout);
  }, [displayIsLoading, lastChunkTime]);

  // TUI-044: Hide compaction notification after 10 seconds
  useEffect(() => {
    if (compactionReduction === null) return;
    const timeout = setTimeout(() => {
      setCompactionReduction(null);
    }, 10000);
    return () => clearTimeout(timeout);
  }, [compactionReduction]);

  // Initialize session when view opens
  useEffect(() => {
    const initSession = async () => {
      try {
        // NAPI-006: Set up data directory (single source of truth)
        // All subdirectories (sessions, cache, blobs, etc.) derive from this
        const fspecDir = getFspecUserDir();
        try {
          persistenceSetDataDirectory(fspecDir);
        } catch (err) {
          // Failed to set up data directory - this is a critical error
          // Cannot continue without data directory - model cache and session storage depend on it
          logger.error('Failed to set up data directory:', err);
          setError(
            `Failed to initialize data directory: ${err instanceof Error ? err.message : String(err)}`
          );
          return; // Stop initialization - cannot proceed without data directory
        }

        // BLOCK-001: Initialize blocklist system with current project directory
        // This loads rules from ~/.fspec/blocklist.json (system) and .fspec/blocklist.json (project)
        // Must be called at TUI startup for blocklist rules to be enforced
        try {
          blocklistInit(process.cwd());
          logger.debug('Blocklist system initialized');
        } catch (err) {
          // Non-critical - blocklist is a safety feature but TUI can still work without it
          logger.warn('Failed to initialize blocklist:', err);
        }

        // TUI-075: Initialize models (loads from NAPI, restores persisted selection)
        try {
          const initResult = await initializeModels();

          // Set available providers for the provider selector
          setAvailableProviders(initResult.availableProviders);

          // Set current provider from initialization result
          if (initResult.currentProvider) {
            setCurrentProvider(initResult.currentProvider);
          }

          logger.debug(
            `Model initialization complete: ${initResult.sections.length} sections, ` +
              `current=${initResult.currentModel?.displayName || 'none'}, ` +
              `persisted=${initResult.persistedModelRestored}`
          );
        } catch (err) {
          logger.error('Failed to initialize models:', err);
        }

        // BUG-122: History loading moved to separate deferred useEffect below.
        // persistenceGetHistory() triggers init_stores() which was loading the
        // entire 1GB messages.jsonl. Even with lazy per-store init (Layer 1),
        // keeping it separate ensures model name renders at ~28ms, not blocked
        // by any persistence I/O.

        setError(null);
      } catch (err) {
        const errorMessage =
          err instanceof Error
            ? err.message
            : 'Failed to initialize AI session';
        setError(errorMessage);
      }
    };

    void initSession();
  }, []);

  // BUG-122 Layer 3: Deferred history loading — does not block model name rendering.
  // persistenceGetHistory() only needs HistoryStore (2.6MB history.jsonl),
  // not MessageStore (1GB). With Layer 1 lazy init this is fast (~50ms),
  // but keeping it deferred means even that I/O never blocks the first render.
  // Shift+↑/↓ gracefully degrades to no-op if history hasn't loaded yet.
  useEffect(() => {
    const timer = setTimeout(() => {
      try {
        const history = persistenceGetHistory(currentProjectRef.current, 100);

        // Convert NAPI history entries (camelCase from NAPI-RS) to our interface
        const entries: HistoryEntry[] = history.map(
          (h: {
            display: string;
            timestamp: string;
            project: string;
            sessionId: string;
            hasPastedContent?: boolean;
          }) => ({
            display: h.display,
            timestamp: h.timestamp,
            project: h.project,
            sessionId: h.sessionId,
            hasPastedContent: h.hasPastedContent ?? false,
          })
        );
        setHistoryEntries(entries);
      } catch (err) {
        logger.error(
          `Failed to load history: ${err instanceof Error ? err.message : String(err)}`
        );
      }
    }, 0);
    return () => clearTimeout(timer);
  }, []);

  // SESS-001: Set current work unit ID on mount/unmount
  // TUI-068: Use sessionStore.setCurrentWorkUnit instead of fspecStore.setCurrentWorkUnitId
  useEffect(() => {
    if (workUnitId) {
      // Get work unit status if available
      const workUnit = workUnits.find(wu => wu.id === workUnitId);
      setCurrentWorkUnit(workUnitId, workUnit?.status ?? null);
    }
    return () => {
      // Clear current work unit when unmounting (returning to board)
      setCurrentWorkUnit(null, null);
    };
  }, [workUnitId, setCurrentWorkUnit, workUnits]);

  // SESS-001: Track if we need to auto-resume an attached session
  const needsAutoResumeRef = useRef<string | null>(null);

  // Track cleanup function for current session's handler registration
  // When navigating away, we call this cleanup to unregister the handler.
  // The GlobalSessionStreamManager stays subscribed; only the handler is unregistered.
  const sessionCleanupRef = useRef<(() => void) | null>(null);

  // Helper to cleanup current session handler and clear ref
  const cleanupCurrentSessionHandler = useCallback(() => {
    if (sessionCleanupRef.current) {
      sessionCleanupRef.current();
      sessionCleanupRef.current = null;
    }
  }, []);

  // TUI-066: Shared handler for /clear command - clears session history
  const handleClearCommand = useCallback(() => {
    setInputValue('');
    if (currentSessionId) {
      try {
        sessionClearHistory(currentSessionId);
      } catch (err) {
        logger.error('[AgentView] Failed to clear session history:', err);
      }
    }
  }, [currentSessionId]);

  // SESS-001: Check for attached session on mount and mark for auto-resume
  useEffect(() => {
    if (workUnitId) {
      const attachedSessionId = getAttachedSession(workUnitId);
      if (attachedSessionId) {
        needsAutoResumeRef.current = attachedSessionId;
        logger.debug(
          `SESS-001: Found attached session ${attachedSessionId} for work unit ${workUnitId}, will auto-resume`
        );
      }
    }
  }, [workUnitId, getAttachedSession]);

  // Handle sending a prompt
  const handleSubmit = useCallback(async () => {
    const userMessage = inputValue.trim();

    // TUI-050: Slash commands should be executed via handleSubmitWithCommand.
    // This handles the case where user types a command and presses Enter without
    // the palette being visible (e.g., after Tab completion closes the palette).
    if (userMessage.startsWith('/') && userMessage.length > 1) {
      executeSlashCommandRef.current?.(userMessage);
      return;
    }

    // TUI-045: /expand command removed - now handled by Enter key opening modal
    // (intentionally removed - /expand will be sent to agent as regular message)

    // NAPI-009: Check if we have a provider configured and not already loading
    if (!currentProvider || !inputValue.trim() || displayIsLoading) {
      // TUI-DEBUG: Log why handleSubmit is blocked (debug level - these are expected conditions)
      if (!currentProvider) {
        logger.debug(
          `[TUI-DEBUG] handleSubmit blocked: currentProvider is empty (currentSessionId=${currentSessionId})`
        );
      }
      if (!inputValue.trim()) {
        logger.debug(`[TUI-DEBUG] handleSubmit blocked: inputValue is empty`);
      }
      if (displayIsLoading) {
        logger.debug(`[TUI-DEBUG] handleSubmit blocked: displayIsLoading=true`);
      }
      return;
    }

    // AGENT-021: Handle /debug command - toggle debug capture mode
    // Supports toggling debug before a session exists
    if (userMessage === '/debug') {
      setInputValue('');
      try {
        const debugDir = getFspecUserDir();
        let result;
        if (currentSessionId) {
          // Session exists - toggle with metadata
          result = await sessionToggleDebug(currentSessionId, debugDir);
        } else {
          // No session yet - toggle without metadata (will be updated when session is created)
          result = toggleDebug(debugDir);
        }
        setIsDebugEnabled(result.enabled);
        setConversation(prev => [
          ...prev,
          { type: 'status', content: result.message },
        ]);
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        setConversation(prev => [
          ...prev,
          { type: 'status', content: `Debug toggle failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /search command - enter history search mode
    if (userMessage === '/search') {
      setInputValue('');
      handleSearchMode();
      return;
    }

    if (userMessage === '/clear') {
      handleClearCommand();
      return;
    }

    // NAPI-003: Handle /resume command - show session selection overlay
    if (userMessage === '/resume') {
      setInputValue('');
      void handleResumeMode();
      return;
    }

    // AMGR-012: Handle /role command — open role dialog for current session
    if (userMessage === '/role') {
      setInputValue('');
      if (!currentSessionId) {
        setConversation(prev => [
          ...prev,
          { type: 'status', content: 'Start a session first to set a role.' },
        ]);
        return;
      }
      setShowRoleDialog(true);
      return;
    }

    // BLOCK-004: Handle /blocklist command - show blocklist management overlay
    if (userMessage === '/blocklist') {
      setInputValue('');
      void handleBlocklistMode();
      return;
    }

    // SESS-001: Handle /detach command - detach session from work unit and clear conversation
    if (userMessage === '/detach') {
      setInputValue('');
      // TUI-069: Use getAttachedWorkUnit from sessionService facade to find the ACTUAL attached work unit,
      // not the workUnitId prop (which is the original board context).
      const attachedWorkUnitId = currentSessionId
        ? getAttachedWorkUnit(currentSessionId)
        : undefined;
      if (attachedWorkUnitId && currentSessionId) {
        // TUI-068: Use sessionService facade for detachment
        detachFromWorkUnit(currentSessionId);
        logger.debug(
          `SESS-001: Detached session from work unit ${attachedWorkUnitId}`
        );
        // Clear conversation for fresh start
        setConversation([]);
        setTokenUsage({ inputTokens: 0, outputTokens: 0 });
        // Reset session state (atomic transition via store)
        prepareForNewSession();
        setConversation([
          {
            type: 'status',
            content:
              'Session detached from work unit. Ready for fresh session.',
          },
        ]);
      } else {
        setConversation(prev => [
          ...prev,
          {
            type: 'status',
            content:
              '/detach only works when viewing a work unit from the board.',
          },
        ]);
      }
      return;
    }

    setInputValue('');
    setHistoryIndex(-1); // Reset history navigation
    setSavedInput('');
    // TUI-031: Reset tok/s display for new prompt (Rust will send new values)
    setDisplayedTokPerSec(null);
    setLastChunkTime(null);

    // NAPI-006: Deferred session creation - only create session on first message
    // This prevents empty sessions from being persisted when user opens modal
    // but doesn't send any messages
    // TUI-034: Store full model path (provider/model-id) for proper restore
    // VIEWNV-001: Use isReadyForNewSession from store (replaces isFirstMessageRef)
    let activeSessionId = currentSessionId;
    if (!activeSessionId && isReadyForNewSession) {
      try {
        const project = currentProjectRef.current;
        // Use first message as session name (truncated to 500 chars to allow wrapping in UI)
        const sessionName =
          userMessage.slice(0, 500) + (userMessage.length > 500 ? '...' : '');

        // TUI-034: Use full model path if available, fallback to provider
        const modelPath = currentModel
          ? `${currentModel.providerId}/${currentModel.modelId}`
          : currentProvider;

        const persistedSession = persistenceCreateSessionWithProvider(
          sessionName,
          project,
          modelPath
        );

        activeSessionId = persistedSession.id;
        // Atomic state transition via store (sets currentSessionId + isReadyForNewSession=false)
        activateSession(activeSessionId);

        // GIT-029: Apply any pending isolation state that arrived before activation
        applyPendingIsolationState(activeSessionId);

        // Register session with SessionManager for background execution
        // This enables ESC + Detach and /resume to work properly
        // Note: Using sessionManagerCreateWithId directly here because session is already created in persistence
        // and we only need the Rust background session (createSession service would duplicate persistence)
        try {
          await sessionManagerCreateWithId(
            activeSessionId,
            modelPath,
            project,
            sessionName
          );
        } catch (err) {
          logger.error('Failed to register session with SessionManager:', err);
          throw new Error(
            `Session registration failed: ${err instanceof Error ? err.message : String(err)}`
          );
        }

        // MODEL-005: Propagate per-model context window and max output tokens to ProviderManager.
        // sessionManagerCreateWithId only passes the model string — for profile/codex models
        // set_model_direct is called with None context params. For cloud models, select_model
        // sets values from the models.dev registry. Either way, we push the TypeScript-side
        // ModelSelection values to ensure they take priority.
        if (currentModel) {
          try {
            if (currentModel.profileConfig || currentModel.providerId === 'codex') {
              await napiSessionSetModelProfile(
                activeSessionId,
                currentModel.providerId,
                currentModel.modelId,
                currentModel.contextWindow,
                currentModel.maxOutput,
                currentModel.facade ?? null
              );
            } else {
              await napiSessionSetModel(
                activeSessionId,
                currentModel.providerId,
                currentModel.modelId,
                currentModel.contextWindow,
                currentModel.maxOutput
              );
            }
          } catch (err) {
            // MODEL-005: Log but don't fail — session works with provider-constant fallback
            logger.error('MODEL-005: Failed to propagate model limits after deferred session creation', {
              error: err,
            });
          }
        }

        // If debug was enabled before session was created, sync debug state to session
        if (isDebugEnabled) {
          try {
            await sessionUpdateDebugMetadata(activeSessionId);
            // Set the debug state on the session (don't toggle - it's already enabled globally)
            sessionSetDebugEnabled(activeSessionId, true);
          } catch (err) {
            logger.error('Failed to sync debug state to session', {
              error: err,
            });
          }
        }

        // SESS-001: Auto-attach session to work unit on first message
        if (workUnitId) {
          // TUI-068: Use sessionService facade for attachment
          // TUI-069: Pass work unit title to avoid hardcoded placeholder
          const workUnit = workUnits.find(wu => wu.id === workUnitId);
          attachToWorkUnit(
            activeSessionId,
            workUnitId,
            workUnit?.status ?? 'backlog',
            workUnit?.title
          );
          logger.debug(
            `SESS-001: Attached session ${activeSessionId} to work unit ${workUnitId}`
          );
        }
        // Note: activateSession() already sets isReadyForNewSession=false atomically
      } catch (err) {
        // Session creation failed - show error and abort
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.error('Failed to create session:', errorMsg);
        setConversation(prev => [
          ...prev,
          { type: 'status', content: `Failed to create session: ${errorMsg}` },
        ]);
        return;
      }
    }

    // NAPI-006: Save command to history
    if (activeSessionId) {
      try {
        persistenceAddHistory(
          userMessage,
          currentProjectRef.current,
          activeSessionId
        );
        // Update local history entries
        setHistoryEntries(prev => [
          {
            display: userMessage,
            timestamp: new Date().toISOString(),
            project: currentProjectRef.current,
            sessionId: activeSessionId,
            hasPastedContent: false,
          },
          ...prev,
        ]);
      } catch (err) {
        logger.error(
          `Failed to save history: ${err instanceof Error ? err.message : String(err)}`
        );
      }
    } else {
      logger.debug('No activeSessionId - history will not be saved');
    }

    // VIEWNV-001: Rename auto-created session on first message
    // If session was auto-created with generic name, rename it to use first message
    if (activeSessionId && sessionNeedsRenameRef.current) {
      sessionNeedsRenameRef.current = false;
      try {
        const sessionName =
          userMessage.slice(0, 500) + (userMessage.length > 500 ? '...' : '');
        persistenceRenameSession(activeSessionId, sessionName);
        logger.debug(
          `VIEWNV-001: Renamed session ${activeSessionId} to: ${sessionName.slice(0, 50)}...`
        );
      } catch (err) {
        logger.error('Failed to rename session:', err);
      }
    }

    // Add user message to conversation
    setConversation(prev => [
      ...prev,
      { type: 'user-input', content: userMessage },
    ]);

    // REFAC-007: User message persistence now handled by Rust in agent_loop
    // (see persist_user_message in session_manager.rs)

    // Add streaming assistant message placeholder
    setConversation(prev => [
      ...prev,
      { type: 'assistant-text', content: '', isStreaming: true },
    ]);

    try {
      // TOOL-010: Detect thinking level from prompt keywords
      const detectedLevel = detectThinkingLevel(userMessage);

      // TUI-054: Compute effective thinking level from base level and detected level
      // Base level comes from /thinking dialog, detected level from prompt keywords
      const baseLevel = rustSnapshot.baseThinkingLevel as JsThinkingLevel;
      const forceOff = hasDisableKeywords(userMessage);
      const effectiveLevel = computeEffectiveThinkingLevel(
        baseLevel,
        detectedLevel,
        forceOff
      );
      setDetectedThinkingLevel(effectiveLevel);

      // Get thinking config JSON if level is not Off
      let thinkingConfig: string | null = null;
      if (effectiveLevel !== JsThinkingLevel.Off) {
        thinkingConfig = getThinkingConfig(currentProvider, effectiveLevel);
        const label = getThinkingLevelLabel(effectiveLevel);
        if (label) {
          logger.debug(
            `Thinking level: ${label} (base=${baseLevel}, detected=${detectedLevel}, forceOff=${forceOff})`
          );
        }
      }

      // Track current text segment (resets after tool calls)
      let currentSegment = '';
      // CLAUDE-THINK: Track current thinking segment for streaming accumulation
      let _currentThinking = '';
      // Track full assistant response for persistence (includes ALL content blocks)
      let _fullAssistantResponse = '';
      // Track assistant message content blocks for envelope storage
      const assistantContentBlocks: Array<{
        type: string;
        text?: string;
        thinking?: string;
        id?: string;
        name?: string;
        input?: unknown;
      }> = [];
      // TOOL-011: Track if we've streamed tool progress (to skip redundant tool result preview)
      let hasStreamedToolProgress = false;

      // This enables detach/attach to work - the background session continues running
      // even when the UI is detached

      // REFAC-008: Cleanup any existing handler before registering new one
      cleanupCurrentSessionHandler();

      // Create a promise that resolves when the agent completes (Done chunk received)
      const promptComplete = new Promise<void>((resolve, reject) => {
        // REFAC-008: Attach via GlobalSessionStreamManager and track cleanup
        // FspecCommandRequest is handled globally - we only get UI chunks here
        sessionCleanupRef.current = attachToSession(
          activeSessionId,
          (chunk: StreamChunk) => {
            if (!chunk) return;

            if (chunk.type === 'Text' && chunk.text) {
              // Text chunks are batched in Rust for efficiency
              currentSegment += chunk.text;
              _fullAssistantResponse += chunk.text; // Accumulate for display persistence
              // Add to content blocks for envelope storage
              const lastBlock =
                assistantContentBlocks[assistantContentBlocks.length - 1];
              if (lastBlock && lastBlock.type === 'text') {
                lastBlock.text = (lastBlock.text || '') + chunk.text;
              } else {
                assistantContentBlocks.push({ type: 'text', text: chunk.text });
              }
              // Update streaming message content
              const segmentSnapshot = currentSegment;
              setConversation(prev => {
                const updated = [...prev];
                const streamingIdx = updated.findLastIndex(m => m.isStreaming);
                if (streamingIdx >= 0) {
                  updated[streamingIdx] = {
                    ...updated[streamingIdx],
                    content: segmentSnapshot,
                  };
                }
                return updated;
              });
            } else if (chunk.type === 'Thinking' && chunk.thinking) {
              // CLAUDE-THINK: Handle thinking/reasoning content from extended thinking
              _currentThinking += chunk.thinking;

              // Store thinking block for envelope persistence
              const lastBlock =
                assistantContentBlocks[assistantContentBlocks.length - 1];
              if (lastBlock && lastBlock.type === 'thinking') {
                lastBlock.thinking =
                  (lastBlock.thinking || '') + chunk.thinking;
              } else {
                assistantContentBlocks.push({
                  type: 'thinking',
                  thinking: chunk.thinking,
                });
              }

              // Update or create thinking message using the ThinkingBlockManager
              // This handles proper block creation and streaming state management
              setConversation(prev =>
                createThinkingUpdate(prev, chunk.thinking || '')
              );
            } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
              // Reset thinking state - new thinking after tool call needs new block
              _currentThinking = '';

              // Finalize any active thinking block before tool call
              setConversation(prev => createFinalizationUpdate(prev));

              // Finalize current streaming message and add tool call (match CLI format)
              const toolCall = chunk.toolCall;

              // Add tool_use block to content blocks for envelope storage
              let parsedInput: unknown;
              try {
                parsedInput = JSON.parse(toolCall.input);
              } catch (err) {
                // Failed to parse tool call input as JSON - indicates malformed data from backend
                logger.error('Failed to parse tool call input as JSON:', err);
                parsedInput = toolCall.input;
              }
              assistantContentBlocks.push({
                type: 'tool_use',
                id: toolCall.id,
                name: toolCall.name,
                input: parsedInput,
              });

              // TUI-038: Store Edit/Write tool inputs for diff display
              // Tool names are lowercase from the streaming API (edit, write, replace, write_file)
              if (typeof parsedInput === 'object' && parsedInput !== null) {
                const inputObj = parsedInput as Record<string, unknown>;
                const toolNameLower = toolCall.name.toLowerCase();
                // Handle both Claude (edit) and Gemini (replace) tool names
                if (
                  (toolNameLower === 'edit' || toolNameLower === 'replace') &&
                  typeof inputObj.old_string === 'string' &&
                  typeof inputObj.new_string === 'string'
                ) {
                  // Calculate start line (edit has already been applied, so we search for new_string)
                  const filePath =
                    typeof inputObj.file_path === 'string'
                      ? inputObj.file_path
                      : undefined;
                  const startLine = calculateStartLine(
                    filePath,
                    inputObj.old_string,
                    inputObj.new_string
                  );
                  pendingToolDiffsRef.current.set(toolCall.id, {
                    toolName: 'Edit',
                    toolCallId: toolCall.id,
                    filePath,
                    oldString: inputObj.old_string,
                    newString: inputObj.new_string,
                    startLine,
                  });
                  // Handle both Claude (write) and Gemini (write_file) tool names
                } else if (
                  (toolNameLower === 'write' ||
                    toolNameLower === 'write_file') &&
                  typeof inputObj.content === 'string'
                ) {
                  pendingToolDiffsRef.current.set(toolCall.id, {
                    toolName: 'Write',
                    toolCallId: toolCall.id,
                    content: inputObj.content,
                  });
                }
              }

              // TUI-037: Format tool header in Claude Code style: ● ToolName(args)
              // Show ALL parameters for full visibility into tool calls
              let argsDisplay = '';
              if (typeof parsedInput === 'object' && parsedInput !== null) {
                const inputObj = parsedInput as Record<string, unknown>;
                argsDisplay = extractToolArgsDisplay(toolCall.name, inputObj);
              } else if (toolCall.input) {
                argsDisplay = toolCall.input;
              }
              const toolContent = formatToolHeader(toolCall.name, argsDisplay);
              const toolContentSnapshot = toolContent;
              setConversation(prev => {
                const updated = [...prev];
                // TUI-037: Remove empty streaming assistant messages before adding tool call
                while (
                  updated.length > 0 &&
                  updated[updated.length - 1].type === 'assistant-text' &&
                  updated[updated.length - 1].isStreaming &&
                  !updated[updated.length - 1].content
                ) {
                  updated.pop();
                }
                // Mark any remaining streaming message as complete
                const streamingIdx = updated.findLastIndex(m => m.isStreaming);
                if (streamingIdx >= 0) {
                  updated[streamingIdx] = {
                    ...updated[streamingIdx],
                    isStreaming: false,
                  };
                }
                // Add tool call message
                updated.push({
                  type: 'tool-call',
                  content: toolContentSnapshot,
                  toolCallId: toolCall.id,
                });
                return updated;
              });
            } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
              // Show tool result in CLI format, then start new streaming message
              const result = chunk.toolResult;

              // REFAC-007: Assistant and tool_result persistence now handled by Rust
              // - Assistant messages persisted on Done/Error/Interrupted in BackgroundOutput::emit()
              // - Tool results persisted immediately in BackgroundOutput::emit()
              // (see persist_assistant_message and persist_tool_result_internal in session_manager.rs)

              // Clear local accumulator (Rust now owns persistence)
              assistantContentBlocks.length = 0;

              // REFAC-007: Tool result persistence now handled by Rust in BackgroundOutput::emit()
              // (see persist_tool_result_internal in session_manager.rs)

              // TUI-037 + TUI-038: Sanitize and format with collapsed output style
              // Check for Edit/Write tool diff display
              const pendingDiff = pendingToolDiffsRef.current.get(
                result.toolCallId
              );
              let toolResultContent: string;
              let toolResultFullContent: string; // TUI-043: Full content for expansion
              // Track if this is an error result for styling
              const isErrorResult = result.isError;

              if (pendingDiff) {
                // TUI-038: Format as diff for Edit/Write tools
                pendingToolDiffsRef.current.delete(result.toolCallId); // Clean up
                if (
                  pendingDiff.toolName === 'Edit' &&
                  pendingDiff.oldString !== undefined &&
                  pendingDiff.newString !== undefined
                ) {
                  const diffLines = formatEditDiff(
                    pendingDiff.oldString,
                    pendingDiff.newString
                  );
                  // Use pre-calculated startLine (or fallback to 1)
                  const startLine = pendingDiff.startLine ?? 1;
                  toolResultContent = formatDiffForDisplay(
                    diffLines,
                    DIFF_COLLAPSED_LINES,
                    startLine
                  );
                  // TUI-043: Full content shows all diff lines
                  toolResultFullContent = formatDiffForDisplay(
                    diffLines,
                    diffLines.length,
                    startLine
                  );
                } else if (
                  pendingDiff.toolName === 'Write' &&
                  pendingDiff.content !== undefined
                ) {
                  const diffLines = formatWriteDiff(pendingDiff.content);
                  toolResultContent = formatDiffForDisplay(diffLines);
                  // TUI-043: Full content shows all diff lines
                  toolResultFullContent = formatDiffForDisplay(
                    diffLines,
                    diffLines.length
                  );
                } else {
                  // Fallback to normal formatting
                  const sanitizedContent = result.content.replace(/\t/g, '  ');
                  toolResultContent = formatCollapsedOutput(sanitizedContent);
                  // TUI-043: Full content without truncation
                  toolResultFullContent = formatFullOutput(sanitizedContent);
                }
              } else {
                // Normal tool result formatting
                const sanitizedContent = result.content.replace(/\t/g, '  ');
                toolResultContent = formatCollapsedOutput(sanitizedContent);
                // TUI-043: Full content without truncation
                toolResultFullContent = formatFullOutput(sanitizedContent);
              }
              currentSegment = ''; // Reset for next text segment

              // TOOL-011 + TUI-037: Combine tool header with result as ONE message
              // First output line has NO L prefix (starts tree), subsequent lines have L prefix
              // formatCollapsedOutput already applies this pattern via formatWithTreeConnectors

              if (hasStreamedToolProgress) {
                hasStreamedToolProgress = false; // Reset for next tool call
                setConversation(prev => {
                  const updated = [...prev];
                  // Find tool header and combine with result
                  for (let i = updated.length - 1; i >= 0; i--) {
                    const msg = updated[i];
                    // Remove [Tool output] messages (streaming placeholder)
                    if (
                      msg.type === 'tool-call' &&
                      msg.content.includes('[Tool output]')
                    ) {
                      updated.splice(i, 1);
                      continue;
                    }
                    // TUI-037: Combine tool header with formatted result
                    // TUI-043: Store both collapsed and full content
                    if (
                      msg.type === 'tool-call' &&
                      msg.content.startsWith('●')
                    ) {
                      const headerLine = msg.content.split('\n')[0];
                      // Don't add newline if result is empty
                      const hasContent =
                        toolResultContent && toolResultContent.trim();
                      updated[i] = {
                        ...msg,
                        content: hasContent
                          ? `${headerLine}\n${toolResultContent}`
                          : headerLine,
                        fullContent: hasContent
                          ? `${headerLine}\n${toolResultFullContent}`
                          : headerLine,
                        isError: isErrorResult,
                      };
                      break;
                    }
                  }
                  return [
                    ...updated,
                    // Add new streaming placeholder for AI continuation
                    {
                      type: 'assistant-text' as const,
                      content: '',
                      isStreaming: true,
                    },
                  ];
                });
              } else {
                // Non-streaming: find the last tool header and combine with result
                setConversation(prev => {
                  const updated = [...prev];
                  // Find tool header (search backwards)
                  for (let i = updated.length - 1; i >= 0; i--) {
                    const msg = updated[i];
                    // TUI-043: Store both collapsed and full content
                    if (
                      msg.type === 'tool-call' &&
                      msg.content.startsWith('●')
                    ) {
                      const headerLine = msg.content.split('\n')[0];
                      // Don't add newline if result is empty
                      const hasContent =
                        toolResultContent && toolResultContent.trim();
                      updated[i] = {
                        ...msg,
                        content: hasContent
                          ? `${headerLine}\n${toolResultContent}`
                          : headerLine,
                        fullContent: hasContent
                          ? `${headerLine}\n${toolResultFullContent}`
                          : headerLine,
                        isError: isErrorResult,
                      };
                      break;
                    }
                  }
                  return [
                    ...updated,
                    // Add new streaming placeholder for AI continuation
                    {
                      type: 'assistant-text' as const,
                      content: '',
                      isStreaming: true,
                    },
                  ];
                });
              }
            } else if (chunk.type === 'Done') {
              // Mark streaming complete and remove empty trailing assistant messages
              // TUI-044: Also apply markdown table formatting to completed assistant messages
              setConversation(prev => {
                const updated = [...prev];
                // Remove empty streaming assistant messages at the end
                while (
                  updated.length > 0 &&
                  updated[updated.length - 1].type === 'assistant-text' &&
                  updated[updated.length - 1].isStreaming &&
                  !updated[updated.length - 1].content
                ) {
                  updated.pop();
                }
                // Mark any remaining streaming message as complete
                // TUI-044: Apply markdown table formatting when marking complete
                const lastAssistantIdx = updated.findLastIndex(
                  m => m.type === 'assistant-text' && m.isStreaming
                );
                if (lastAssistantIdx >= 0) {
                  const originalContent = updated[lastAssistantIdx].content;
                  const formattedContent =
                    formatMarkdownTables(originalContent);
                  updated[lastAssistantIdx] = {
                    ...updated[lastAssistantIdx],
                    content: formattedContent,
                    isStreaming: false,
                  };
                }
                return updated;
              });

              refreshRustState(activeSessionId);

              // REFAC-007: Token state persistence now handled by Rust
              // (Rust persists token state when streaming completes - TODO: implement in session_manager.rs)

              // NAPI-009: Resolve the promise when agent completes
              resolve();
            } else if (chunk.type === 'SessionStateChange') {
              // NAPI-010: Internal state change - update state machine, do NOT add to conversation

              if (chunk.state === 'Cleared') {
                // TUI-066: React state update as side effect of Rust clear_history()
                setConversation([]);
                setTokenUsage({ inputTokens: 0, outputTokens: 0 });
                setContextFillPercentage(0);
              } else if (chunk.state === 'Compacting') {
                // UX-002: Use unified compaction hook for ALL compaction state
                const progress = sessionGetCompactionProgress(activeSessionId);
                compactionRef.current.startCompaction(
                  'hook-triggered',
                  activeSessionId,
                  progress ?? undefined
                );
              }
              // Do NOT call endCompaction() for Running state.
              // During active compaction, CompactionContinuing emits SessionStateChange(Running)
              // but the DAG construction is still in progress. Only CompactionComplete
              // should end the compaction indicator.

              refreshRustState(activeSessionId);
            } else if (chunk.type === 'CompactionComplete') {
              handleCompactionComplete(
                chunk.compactionResult,
                activeSessionId,
              );
              // Don't add to conversation - compaction feedback is via input area indicator
            } else if (chunk.type === 'UserNotification') {
              // NAPI-010: User-facing notification - display in conversation
              // UX-002: Compaction success messages now come via CompactionComplete chunk (above)
              // Only failure messages come through UserNotification
              const statusMessage = chunk.message;
              // Filter compaction failure messages from conversation (they show in retry dialog)
              const isCompactionFailure = /^Compaction failed:/.test(
                statusMessage
              );
              if (!isCompactionFailure) {
                // NET-001: Handle network reconnection messages with replace semantics.
                // "✓ Reconnected" or "✗ Reconnection failed" replaces the prior
                // "⟳ Reconnecting..." message, so the user sees one message that
                // transitions rather than accumulating clutter.
                const isReconnectionUpdate =
                  statusMessage === '✓ Reconnected' ||
                  statusMessage === '✗ Reconnection failed';
                setConversation(prev => {
                  if (isReconnectionUpdate) {
                    const idx = prev.findLastIndex(
                      m => m.type === 'status' && m.content === '⟳ Reconnecting...'
                    );
                    if (idx !== -1) {
                      const updated = [...prev];
                      updated[idx] = { type: 'status', content: statusMessage };
                      return updated;
                    }
                  }
                  return [
                    ...prev,
                    { type: 'status', content: statusMessage },
                  ];
                });
              }
            } else if (chunk.type === 'Interrupted') {
              // Agent was interrupted by user
              // TUI-037: Only append to tool if it's still streaming (no collapse indicator)
              // If tool has collapse indicator, it completed - interrupt is for AI continuation
              setConversation(prev => {
                const updated = [...prev];

                // First, remove empty streaming assistant messages
                while (
                  updated.length > 0 &&
                  updated[updated.length - 1].type === 'assistant-text' &&
                  updated[updated.length - 1].isStreaming &&
                  !updated[updated.length - 1].content
                ) {
                  updated.pop();
                }

                // Find the last tool message
                let handledInterrupt = false;
                for (let i = updated.length - 1; i >= 0; i--) {
                  const msg = updated[i];
                  if (msg.type === 'tool-call' && msg.content.startsWith('●')) {
                    // Only append if tool is still streaming (no collapse indicator = no ToolResult yet)
                    if (!msg.content.includes('(select turn to /expand)')) {
                      updated[i] = {
                        ...msg,
                        content: `${msg.content}\nL ⚠ Interrupted`,
                      };
                      handledInterrupt = true;
                    }
                    // If tool has collapse indicator, it completed - don't append
                    break;
                  }
                }

                // If no tool was streaming, add interrupt as status (not appended to anything)
                if (!handledInterrupt) {
                  updated.push({
                    type: 'status' as const,
                    content: '⚠ Interrupted',
                  });
                }

                // Mark any remaining streaming message as complete
                const lastAssistantIdx = updated.findLastIndex(
                  m => m.type === 'assistant-text' && m.isStreaming
                );
                if (lastAssistantIdx >= 0) {
                  updated[lastAssistantIdx] = {
                    ...updated[lastAssistantIdx],
                    isStreaming: false,
                  };
                }
                return updated;
              });
            } else if (
              chunk.type === 'TokenUpdate' ||
              chunk.type === 'ContextFillUpdate'
            ) {
              // TUI-049: Use centralized helper for token state updates (DRY)
              updateTokenStateFromChunk(chunk);
            } else if (chunk.type === 'ToolProgress' && chunk.toolProgress) {
              // TOOL-011 + TUI-037: Stream tool execution progress with rolling window
              // Display the output chunk in a fixed-height window (last N lines)
              hasStreamedToolProgress = true;
              // Mark stderr output with special prefix for red rendering
              const isStderr = chunk.toolProgress.isStderr;
              const rawChunk = chunk.toolProgress.outputChunk;
              // Prefix each line of stderr with marker for visual distinction
              const outputChunk = isStderr
                ? rawChunk
                    .split('\n')
                    .map(line => (line ? `⚠stderr⚠${line}` : line))
                    .join('\n')
                : rawChunk;
              setConversation(prev => {
                const updated = [...prev];
                const lastIdx = updated.length - 1;
                if (lastIdx >= 0) {
                  const lastMsg = updated[lastIdx];
                  // TUI-037: If last message is a tool header (●), append streaming output with tree connectors
                  if (
                    lastMsg.type === 'tool-call' &&
                    lastMsg.content.startsWith('●')
                  ) {
                    // Separate header from streaming content
                    const lines = lastMsg.content.split('\n');
                    const header = lines[0]; // ● ToolName(args)
                    // Extract raw output by removing tree prefixes (L or indent)
                    const existingOutput = lines
                      .slice(1)
                      .map(l => {
                        if (l.startsWith('L ')) return l.slice(2);
                        if (l.startsWith('  ')) return l.slice(2);
                        return l;
                      })
                      .join('\n');
                    const newOutput = existingOutput + outputChunk;
                    // Apply streaming window - keep only last N lines of output
                    const windowedOutput = createStreamingWindow(newOutput);
                    // Format with tree connectors: L on first line, indent on rest
                    const windowedLines = windowedOutput.split('\n');
                    const formattedOutput = windowedLines
                      .map((l, i) => {
                        if (i === 0) return `L ${l}`;
                        return `  ${l}`;
                      })
                      .join('\n');
                    updated[lastIdx] = {
                      ...lastMsg,
                      content: `${header}\n${formattedOutput}`,
                    };
                  } else if (
                    lastMsg.type === 'tool-call' &&
                    lastMsg.content.includes('[Tool output]')
                  ) {
                    // Already showing tool output, append and apply window
                    const existingContent = lastMsg.content.replace(
                      '[Tool output]\n',
                      ''
                    );
                    const newOutput = existingContent + outputChunk;
                    const windowedOutput = createStreamingWindow(newOutput);
                    updated[lastIdx] = {
                      ...lastMsg,
                      content: `[Tool output]\n${windowedOutput}`,
                    };
                  } else {
                    // Create new tool output message
                    updated.push({
                      type: 'tool-call',
                      content: `[Tool output]\n${outputChunk}`,
                    });
                  }
                }
                return updated;
              });
            } else if (chunk.type === 'Error' && chunk.error) {
              // Log the error
              logger.error(`Stream error: ${chunk.error}`);

              // Show error in modal for user visibility
              setError(chunk.error);

              // API error occurred - clean up streaming placeholder and show error in conversation
              setConversation(prev => {
                const updated = [...prev];
                // Remove empty streaming assistant messages at the end
                while (
                  updated.length > 0 &&
                  updated[updated.length - 1].type === 'assistant-text' &&
                  updated[updated.length - 1].isStreaming &&
                  !updated[updated.length - 1].content
                ) {
                  updated.pop();
                }
                // Add error as status message so it's visible in conversation
                updated.push({
                  type: 'status',
                  content: `API Error: ${chunk.error}`,
                });
                return updated;
              });
              // NAPI-009: Reject the promise on error
              reject(new Error(chunk.error));
            } else if (chunk.type === 'IncomingMessage' && chunk.text) {
              // BRIDGE-006: Handle supervisor/bridge input messages during streaming
              const supervisorInfo = parseSupervisorPrefix(chunk.text);
              setConversation(prev => {
                if (supervisorInfo) {
                  // Format content with role prefix (no emoji)
                  const formattedContent = `[W] ${supervisorInfo.role}> ${supervisorInfo.content}`;
                  return [
                    ...prev,
                    { type: 'supervisor-input', content: formattedContent },
                  ];
                } else {
                  // Fallback: if parsing fails, display raw message
                  return [
                    ...prev,
                    { type: 'supervisor-input', content: chunk.text! },
                  ];
                }
              });
            }
            // REFAC-008: FspecCommandRequest is handled globally by GlobalSessionStreamManager
            // AgentView no longer processes FspecCommandRequest chunks
          }
        );
      });

      // NAPI-009: Send the input to the background session (non-blocking)
      // The background session's agent_loop will process it and emit chunks via the callback
      sessionSendInput(activeSessionId, userMessage, thinkingConfig);

      // Refresh Rust state to pick up status change (running) after sending input
      // CRITICAL: Pass activeSessionId explicitly to handle race condition with Zustand state updates.
      // When creating a new session, activateSession() schedules a batched update that hasn't
      // taken effect yet, so the hook's captured sessionId is still null. Using the local variable
      // ensures we refresh the correct session immediately.
      refreshRustState(activeSessionId);

      // Wait for the prompt to complete (Done chunk received)
      await promptComplete;

      // BRIDGE-013: Cleanup handleSubmit's handler so persistent handler can take over
      cleanupCurrentSessionHandler();

      // Persist full envelopes to session (includes tool calls and results)
      // REFAC-007: Final assistant message persistence now handled by Rust
      // BackgroundOutput::emit() persists on Done/Error/Interrupted events
      // (see persist_assistant_message in session_manager.rs)

      // Token usage is now handled by background session via TokenUpdate chunks
    } catch (err) {
      // BRIDGE-013: Cleanup handleSubmit's handler on error so persistent handler can take over
      cleanupCurrentSessionHandler();

      const errorMessage =
        err instanceof Error ? err.message : 'Failed to send prompt';
      // Clean up streaming placeholder and show error in conversation
      setConversation(prev => {
        const updated = [...prev];
        // Remove empty streaming assistant messages at the end
        while (
          updated.length > 0 &&
          updated[updated.length - 1].type === 'assistant-text' &&
          updated[updated.length - 1].isStreaming &&
          !updated[updated.length - 1].content
        ) {
          updated.pop();
        }
        // Add error as status message so it's visible in conversation
        updated.push({ type: 'status', content: `Error: ${errorMessage}` });
        return updated;
      });
    }
  }, [
    inputValue,
    displayIsLoading,
    currentSessionId,
    currentProvider,
    currentModel,
    workUnitId,
    workUnits,
    // TUI-068: attachToWorkUnit, detachFromWorkUnit, getAttachedWorkUnit are module-level imports (stable)
    isReadyForNewSession,
    activateSession,
    prepareForNewSession,
  ]);

  // TUI-050: Handle submit with explicit command string (for slash command palette Enter)
  // This avoids race condition with setTimeout by passing the command directly
  const handleSubmitWithCommand = useCallback(
    async (commandText: string) => {
      const userMessage = commandText.trim();

      // Handle /model command
      // TUI-073: ModelSelectorScreen now manages its own state via useModelSelectorState hook
      // TUI-075: Models are loaded lazily when ModelSelectorScreen mounts (no pre-check needed)
      if (userMessage === '/model') {
        setInputValue('');
        setShowModelSelector(true);
        return;
      }

      // Handle /provider command - TUI-074: ProviderSettingsScreen manages its own state
      if (userMessage === '/provider') {
        setInputValue('');
        setShowSettingsTab(true);
        return;
      }

      // Handle /debug command
      if (userMessage === '/debug') {
        setInputValue('');
        try {
          const debugDir = getFspecUserDir();
          let result;
          if (currentSessionId) {
            result = await sessionToggleDebug(currentSessionId, debugDir);
          } else {
            result = toggleDebug(debugDir);
          }
          setIsDebugEnabled(result.enabled);
          setConversation(prev => [
            ...prev,
            { type: 'status', content: result.message },
          ]);
        } catch (error) {
          const errorMessage =
            error instanceof Error ? error.message : String(error);
          setConversation(prev => [
            ...prev,
            { type: 'status', content: `Debug toggle failed: ${errorMessage}` },
          ]);
        }
        return;
      }

      // Handle /compact command
      if (userMessage === '/compact') {
        setInputValue('');
        // PERF-002: Check if there's an active session to compact
        if (!currentSessionId) {
          setConversation(prev => [
            ...prev,
            {
              type: 'status',
              content:
                'No active session to compact. Start a conversation first.',
            },
          ]);
          return;
        }

        // PERF-002: Use the compaction hook for clean separation of concerns
        try {
          // UX-002: Don't add compaction messages to conversation
          // All compaction feedback is shown in input area via isCompacting state

          const result =
            await compaction.performManualCompaction(currentSessionId);

          // Update token display from compaction result
          setTokenUsage(prev => ({
            ...prev,
            inputTokens: result.compactedTokens,
          }));

          // UX-002: Don't add success message to conversation
          // Compaction completion is handled by state transition

          setCompactionReduction(result.compressionRatio);
        } catch (err) {
          const errorMessage =
            err instanceof Error ? err.message : 'Failed to compact';
          // Error handling and retry dialog are managed by the hook
          if (!compaction.retryState.isVisible) {
            setConversation(prev => [
              ...prev,
              { type: 'status', content: `Compaction failed: ${errorMessage}` },
            ]);
          }
        }
        return;
      }

      // Handle /search command - inline state changes to avoid TDZ
      if (userMessage === '/search') {
        setInputValue('');
        setIsSearchMode(true);
        setSearchQuery('');
        setSearchResults([]);
        setSearchResultIndex(0);
        return;
      }

      if (userMessage === '/clear') {
        handleClearCommand();
        return;
      }

      // Handle /resume command - trigger initialization (avoiding TDZ with handleResumeMode)
      if (userMessage === '/resume') {
        setInputValue('');
        setTriggerResumeModeInit(true); // Will trigger useEffect to call handleResumeMode
        return;
      }

      // AMGR-012: Handle /role command - open role dialog
      if (userMessage === '/role') {
        setInputValue('');
        if (currentSessionId) {
          setShowRoleDialog(true);
        } else {
          setConversation(prev => [
            ...prev,
            { type: 'status', content: 'Start a session first to set a role.' },
          ]);
        }
        return;
      }

      // BLOCK-004: Handle /blocklist command - show blocklist management overlay
      if (userMessage === '/blocklist') {
        setInputValue('');
        void handleBlocklistMode();
        return;
      }

      // Handle /detach command
      if (userMessage === '/detach') {
        setInputValue('');
        // TUI-069: Use getAttachedWorkUnit from sessionService facade to find the ACTUAL attached work unit,
        // not the workUnitId prop (which is the original board context).
        const attachedWorkUnitId = currentSessionId
          ? getAttachedWorkUnit(currentSessionId)
          : undefined;
        if (attachedWorkUnitId && currentSessionId) {
          // TUI-068: Use sessionService facade for detachment
          detachFromWorkUnit(currentSessionId);
          setConversation([]);
          setTokenUsage({ inputTokens: 0, outputTokens: 0 });
          prepareForNewSession();
          setConversation([
            {
              type: 'status',
              content:
                'Session detached from work unit. Ready for fresh session.',
            },
          ]);
        } else {
          setConversation(prev => [
            ...prev,
            {
              type: 'status',
              content:
                '/detach only works when viewing a work unit from the board.',
            },
          ]);
        }
        return;
      }

      // GIT-036, GIT-037, GIT-038: Handle /merge-worktree command - merge worktree changes and close session
      if (userMessage === '/merge-worktree') {
        await handleMergeWorktree({
          isIsolated,
          currentSessionId,
          repoPath: currentProjectRef.current,
          worktreePath,
          setConversation,
          setInputValue,
          cleanupCurrentSessionHandler,
          onExit,
          setActionPrompt,
          // GIT-038: Send conflict context as a user message to the Rust session.
          // Setting inputValue + flagging auto-submit causes handleSubmit to fire
          // on the next render, sending the conflict details to the LLM as real input
          // so it can read the files and resolve the conflict markers.
          injectLlmContext: (content: string) => {
            setInputValue(content);
            pendingAutoSubmitRef.current = true;
          },
        });
        return;
      }

      // TUI-054: Handle /thinking command - set base thinking level
      // Accepts: /thinking (opens dialog) or /thinking <level> (sets directly)
      // Levels: off, low, med/medium, high (case insensitive)
      if (userMessage === '/thinking' || userMessage.startsWith('/thinking ')) {
        setInputValue('');

        // Require an active session
        if (!currentSessionId) {
          setConversation(prev => [
            ...prev,
            {
              type: 'status',
              content: 'Start a session first to set the thinking level.',
            },
          ]);
          return;
        }

        // Parse optional argument
        const arg = userMessage.slice('/thinking'.length).trim().toLowerCase();

        if (!arg) {
          // No argument - open the dialog
          setShowThinkingLevelDialog(true);
          return;
        }

        // Parse level argument
        let level: JsThinkingLevel | null = null;
        if (arg === 'off') {
          level = JsThinkingLevel.Off;
        } else if (arg === 'low') {
          level = JsThinkingLevel.Low;
        } else if (arg === 'med' || arg === 'medium') {
          level = JsThinkingLevel.Medium;
        } else if (arg === 'high') {
          level = JsThinkingLevel.High;
        }

        if (level !== null) {
          getRustStateSource().setBaseThinkingLevel(currentSessionId, level);
          const levelNames = ['Off', 'Low', 'Medium', 'High'];
          setConversation(prev => [
            ...prev,
            {
              type: 'status',
              content: `Thinking level set to ${levelNames[level]}.`,
            },
          ]);
        } else {
          setConversation(prev => [
            ...prev,
            {
              type: 'status',
              content: `Invalid thinking level "${arg}". Use: off, low, med, medium, or high.`,
            },
          ]);
        }
        return;
      }

      // SCHED-008: Handle /schedule command — manage scheduled jobs
      if (userMessage === '/schedule' || userMessage.startsWith('/schedule ')) {
        setInputValue('');

        const { handleScheduleCommand } = await import(
          '../services/schedule-service'
        );
        const cwd = process.cwd();
        const result = await handleScheduleCommand(userMessage, cwd);

        setConversation(prev => [
          ...prev,
          {
            type: 'status',
            content: result.message,
          },
        ]);
        return;
      }

      // SCHED-011: Handle /loop command — session-scoped recurring loops
      if (userMessage === '/loop' || userMessage.startsWith('/loop ')) {
        setInputValue('');

        const { handleLoopCommand } = await import(
          '../services/loop-service'
        );
        const result = await handleLoopCommand(userMessage, currentSessionIdRef.current ?? null);

        setConversation(prev => [
          ...prev,
          {
            type: 'status',
            content: result.message,
          },
        ]);
        return;
      }

      // For any unrecognized command, just clear input (user typed incomplete command)
      setInputValue('');
    },
    [
      providerSections,
      currentModel,
      currentSessionId,
      // TUI-068/TUI-069: detachFromWorkUnit, getAttachedWorkUnit are module-level imports (stable)
      prepareForNewSession,
    ]
  );

  // TUI-050: Update slash command executor ref after handleSubmitWithCommand is defined
  useEffect(() => {
    executeSlashCommandRef.current = (cmd: string) =>
      void handleSubmitWithCommand(cmd);
  }, [handleSubmitWithCommand]);

  // GIT-038: Auto-submit conflict resolution message to Rust session.
  // When /merge-worktree detects conflicts, it sets inputValue to the conflict
  // details and flags pendingAutoSubmitRef. On the next render (after inputValue
  // and handleSubmit have updated), this effect fires and submits the message
  // to the Rust session so the LLM actually sees the conflict info.
  useEffect(() => {
    if (pendingAutoSubmitRef.current && inputValue) {
      pendingAutoSubmitRef.current = false;
      void handleSubmit();
    }
  }, [inputValue, handleSubmit]);

  // Handle provider switching - now just updates local state
  // Actual provider change happens on next session creation
  const handleSwitchProvider = useCallback(async (providerName: string) => {
    setCurrentProvider(providerName);
    setShowProviderSelector(false);
  }, []);

  // Handle model selection from ModelSelectorScreen
  // PROV-008: Delegates to selectModel service for DRY/SOLID compliance
  // BUG-097: Now handles failure result and shows error to user
  // PROV-057: When the user picks a github-copilot/* model and no credentials
  // exist on disk yet, route into the Copilot OAuth login flow instead of
  // surfacing a "requires credentials" error.
  const handleModelSelect = useCallback(
    async (selection: ModelSelection) => {
      setShowModelSelector(false);

      // PROV-057: github-copilot + missing credentials → launch login flow
      // via ProviderSettingsScreen (which owns the useProviderSettingsState
      // hook needed by startCopilotLogin).
      if (shouldDispatchCopilotLogin(providerSections, selection)) {
        setAutoStartCopilotLogin(true);
        setShowSettingsTab(true);
        return;
      }

      const result = await selectModel({
        sessionId: currentSessionId,
        selection,
        onRefreshRustState: refreshRustState,
        onSetCurrentModel: setCurrentModel,
        onSetCurrentProvider: setCurrentProvider,
      });

      if (!result.success) {
        setError(`Failed to switch model: ${result.error || 'Unknown error'}`);
      }
    },
    [currentSessionId, providerSections, refreshRustState, setCurrentModel]
  );

  // NAPI-006: Navigate to previous history entry (Shift+Arrow-Up)
  const handleHistoryPrev = useCallback(() => {
    if (historyEntries.length === 0) {
      return;
    }

    // Save current input if we're starting navigation
    if (historyIndex === -1) {
      setSavedInput(inputValue);
    }

    const newIndex =
      historyIndex === -1
        ? 0
        : Math.min(historyIndex + 1, historyEntries.length - 1);
    setHistoryIndex(newIndex);
    setInputValue(historyEntries[newIndex].display);
  }, [historyEntries, historyIndex, inputValue]);

  // NAPI-006: Navigate to next history entry (Shift+Arrow-Down)
  const handleHistoryNext = useCallback(() => {
    if (historyIndex === -1) return;

    if (historyIndex === 0) {
      // Return to saved input
      setHistoryIndex(-1);
      setInputValue(savedInput);
    } else {
      const newIndex = historyIndex - 1;
      setHistoryIndex(newIndex);
      setInputValue(historyEntries[newIndex].display);
    }
  }, [historyEntries, historyIndex, savedInput]);

  // TUI-050/TUI-055: Combined input change handler for both slash commands and file search
  const handleInputChange = useCallback(
    (newValue: string) => {
      // Both hooks handle their respective detection logic and call setInputValue
      slashCommand.handleInputChange(newValue);
      fileSearch.handleInputChange(newValue);
    },
    [slashCommand.handleInputChange, fileSearch.handleInputChange]
  );

  // NAPI-006: Enter search mode (Ctrl+R)
  const handleSearchMode = useCallback(() => {
    setIsSearchMode(true);
    setSearchQuery('');
    setSearchResults([]);
    setSearchResultIndex(0);
  }, []);

  // NAPI-006: Handle search input
  const handleSearchInput = useCallback(async (query: string) => {
    setSearchQuery(query);
    if (!query.trim()) {
      setSearchResults([]);
      return;
    }

    try {
      const results = persistenceSearchHistory(
        query,
        currentProjectRef.current
      );
      const entries: HistoryEntry[] = results.map(
        (h: {
          display: string;
          timestamp: string;
          project: string;
          sessionId: string;
          hasPastedContent?: boolean;
        }) => ({
          display: h.display,
          timestamp: h.timestamp,
          project: h.project,
          sessionId: h.sessionId,
          hasPastedContent: h.hasPastedContent ?? false,
        })
      );
      setSearchResults(entries);
      setSearchResultIndex(0);
    } catch (err) {
      // Failed to search persistence history - indicates persistence system issues
      logger.error('Failed to search persistence history:', err);
    }
  }, []);

  // NAPI-006: Select search result and exit search mode
  const handleSearchSelect = useCallback(() => {
    if (searchResults.length > 0 && searchResultIndex < searchResults.length) {
      setInputValue(searchResults[searchResultIndex].display);
    }
    setIsSearchMode(false);
    setSearchQuery('');
    setSearchResults([]);
  }, [searchResults, searchResultIndex]);

  // NAPI-006: Cancel search mode
  const handleSearchCancel = useCallback(() => {
    setIsSearchMode(false);
    setSearchQuery('');
    setSearchResults([]);
  }, []);

  // NAPI-003: Format relative time in human-readable format
  const formatTimeAgo = useCallback((date: Date): string => {
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    // Format time as HH:MM
    const timeStr = date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays === 1) return `yesterday ${timeStr}`;
    if (diffDays < 7) {
      const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });
      return `${dayName} ${timeStr}`;
    }
    // For older sessions, show date and time
    const monthDay = date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    });
    return `${monthDay} ${timeStr}`;
  }, []);

  // TUI-047: Helper function to process streaming chunks from background sessions
  // Used by handleResumeSelect when attaching to running/idle background sessions
  // This is a simplified version of the inline chunk handling in handleSubmit,
  // suitable for reattaching to sessions that may have produced output while detached.
  const handleStreamChunk = useCallback((chunk: StreamChunk) => {
    if (!chunk) return;

    if (chunk.type === 'Text' && chunk.text) {
      // Update the last assistant message, or create one if needed
      setConversation(prev => {
        const updated = [...prev];
        const lastIdx = updated.findLastIndex(m => m.type === 'assistant-text');
        if (lastIdx >= 0 && updated[lastIdx].isStreaming) {
          updated[lastIdx] = {
            ...updated[lastIdx],
            content: updated[lastIdx].content + chunk.text,
          };
        } else {
          // Create new streaming assistant message
          updated.push({
            type: 'assistant-text',
            content: chunk.text || '',
            isStreaming: true,
          });
        }
        return updated;
      });
    } else if (chunk.type === 'Thinking' && chunk.thinking) {
      // Show thinking content in a separate message
      setConversation(prev => createThinkingUpdate(prev, chunk.thinking || ''));
    } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
      // Finalize any active thinking block before tool call
      setConversation(prev => createFinalizationUpdate(prev));

      const toolCall = chunk.toolCall;
      let argsDisplay = '';
      let parsedInput: unknown;
      try {
        parsedInput = JSON.parse(toolCall.input);
        if (typeof parsedInput === 'object' && parsedInput !== null) {
          const inputObj = parsedInput as Record<string, unknown>;
          argsDisplay = extractToolArgsDisplay(toolCall.name, inputObj);
        }
      } catch (err) {
        // Failed to parse tool call input JSON for display - indicates malformed data from backend
        logger.error('Failed to parse tool call input JSON for display:', err);
        argsDisplay = toolCall.input;
      }

      // TUI-038: Store Edit/Write tool inputs for diff display (same as handleSubmit)
      if (typeof parsedInput === 'object' && parsedInput !== null) {
        const inputObj = parsedInput as Record<string, unknown>;
        const toolNameLower = toolCall.name.toLowerCase();
        if (
          (toolNameLower === 'edit' || toolNameLower === 'replace') &&
          typeof inputObj.old_string === 'string' &&
          typeof inputObj.new_string === 'string'
        ) {
          const filePath =
            typeof inputObj.file_path === 'string'
              ? inputObj.file_path
              : undefined;
          const startLine = calculateStartLine(
            filePath,
            inputObj.old_string,
            inputObj.new_string
          );
          pendingToolDiffsRef.current.set(toolCall.id, {
            toolName: 'Edit',
            toolCallId: toolCall.id,
            filePath,
            oldString: inputObj.old_string,
            newString: inputObj.new_string,
            startLine,
          });
        } else if (
          (toolNameLower === 'write' || toolNameLower === 'write_file') &&
          typeof inputObj.content === 'string'
        ) {
          pendingToolDiffsRef.current.set(toolCall.id, {
            toolName: 'Write',
            toolCallId: toolCall.id,
            content: inputObj.content,
          });
        }
      }

      const toolContent = formatToolHeader(toolCall.name, argsDisplay);
      setConversation(prev => {
        const updated = [...prev];
        // Mark streaming message as complete, or remove if empty
        const streamingIdx = updated.findLastIndex(m => m.isStreaming);
        if (streamingIdx >= 0) {
          if (updated[streamingIdx].content.trim() === '') {
            updated.splice(streamingIdx, 1);
          } else {
            updated[streamingIdx] = {
              ...updated[streamingIdx],
              isStreaming: false,
            };
          }
        }
        updated.push({
          type: 'tool-call',
          content: toolContent,
          toolCallId: toolCall.id,
        });
        return updated;
      });
    } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
      const result = chunk.toolResult;

      // TUI-038: Check for Edit/Write tool diff display (same as handleSubmit)
      const pendingDiff = pendingToolDiffsRef.current.get(
        result.toolCallId
      );
      let toolResultContent: string;
      let toolResultFullContent: string;

      if (pendingDiff) {
        pendingToolDiffsRef.current.delete(result.toolCallId);
        if (
          pendingDiff.toolName === 'Edit' &&
          pendingDiff.oldString !== undefined &&
          pendingDiff.newString !== undefined
        ) {
          const diffLines = formatEditDiff(
            pendingDiff.oldString,
            pendingDiff.newString
          );
          const startLine = pendingDiff.startLine ?? 1;
          toolResultContent = formatDiffForDisplay(
            diffLines,
            DIFF_COLLAPSED_LINES,
            startLine
          );
          toolResultFullContent = formatDiffForDisplay(
            diffLines,
            diffLines.length,
            startLine
          );
        } else if (
          pendingDiff.toolName === 'Write' &&
          pendingDiff.content !== undefined
        ) {
          const diffLines = formatWriteDiff(pendingDiff.content);
          toolResultContent = formatDiffForDisplay(diffLines);
          toolResultFullContent = formatDiffForDisplay(
            diffLines,
            diffLines.length
          );
        } else {
          const sanitizedContent = result.content.replace(/\t/g, '  ');
          toolResultContent = formatCollapsedOutput(sanitizedContent);
          toolResultFullContent = formatFullOutput(sanitizedContent);
        }
      } else {
        const sanitizedContent = result.content.replace(/\t/g, '  ');
        toolResultContent = formatCollapsedOutput(sanitizedContent);
        toolResultFullContent = formatFullOutput(sanitizedContent);
      }

      setConversation(prev => {
        const updated = [...prev];
        // Find tool header and combine with result
        for (let i = updated.length - 1; i >= 0; i--) {
          const msg = updated[i];
          if (msg.type === 'tool-call' && msg.content.startsWith('●')) {
            const headerLine = msg.content.split('\n')[0];
            // Don't add newline if result is empty
            const hasContent = toolResultContent && toolResultContent.trim();
            updated[i] = {
              ...msg,
              content: hasContent
                ? `${headerLine}\n${toolResultContent}`
                : headerLine,
              // TUI-043: Set fullContent for expansion
              fullContent: hasContent
                ? `${headerLine}\n${toolResultFullContent}`
                : headerLine,
              isError: result.isError,
            };
            break;
          }
        }
        // Add streaming placeholder for continuation
        updated.push({
          type: 'assistant-text',
          content: '',
          isStreaming: true,
        });
        return updated;
      });
    } else if (chunk.type === 'Done') {
      // Mark streaming complete
      setConversation(prev => {
        const updated = [...prev];
        // Remove empty streaming messages
        while (
          updated.length > 0 &&
          updated[updated.length - 1].type === 'assistant-text' &&
          updated[updated.length - 1].isStreaming &&
          !updated[updated.length - 1].content
        ) {
          updated.pop();
        }
        // Mark remaining as complete
        const streamingIdx = updated.findLastIndex(m => m.isStreaming);
        if (streamingIdx >= 0) {
          const originalContent = updated[streamingIdx].content;
          updated[streamingIdx] = {
            ...updated[streamingIdx],
            content: formatMarkdownTables(originalContent),
            isStreaming: false,
          };
        }
        return updated;
      });

      refreshRustState(currentSessionIdRef.current);

      // REFAC-007: Token state persistence now handled by Rust
      // (Rust persists token state when streaming completes - TODO: implement in session_manager.rs)
    } else if (chunk.type === 'SessionStateChange') {
      // NAPI-010: Internal state change - update state machine, do NOT add to conversation

      if (chunk.state === 'Cleared') {
        // TUI-066: React state update as side effect of Rust clear_history()
        setConversation([]);
        setTokenUsage({ inputTokens: 0, outputTokens: 0 });
        setContextFillPercentage(0);
      } else if (chunk.state === 'Compacting') {
        // UX-002: Use unified compaction hook for ALL compaction state
        const sessionId = currentSessionIdRef.current;
        if (sessionId) {
          const progress = sessionGetCompactionProgress(sessionId);
          compactionRef.current.startCompaction(
            'hook-triggered',
            sessionId,
            progress ?? undefined
          );
        }
      }
      // Do NOT call endCompaction() for Running state.
      // During active compaction, CompactionContinuing emits SessionStateChange(Running)
      // but the DAG construction is still in progress. Only CompactionComplete
      // should end the compaction indicator.

      refreshRustState(currentSessionIdRef.current);
    } else if (chunk.type === 'CompactionComplete') {
      handleCompactionComplete(
        chunk.compactionResult,
        currentSessionIdRef.current,
      );
      // Don't add to conversation - compaction feedback is via input area indicator
    } else if (chunk.type === 'UserNotification') {
      // NAPI-010: User-facing notification - display in conversation
      // UX-002: Compaction success messages now come via CompactionComplete chunk (above)
      // Only failure messages come through UserNotification
      const statusMessage = chunk.message;
      // Filter compaction failure messages from conversation (they show in retry dialog)
      const isCompactionFailure = /^Compaction failed:/.test(statusMessage);
      if (!isCompactionFailure) {
        // NET-001: Handle network reconnection messages with replace semantics.
        // "✓ Reconnected" or "✗ Reconnection failed" replaces the prior
        // "⟳ Reconnecting..." message, so the user sees one message that
        // transitions rather than accumulating clutter.
        const isReconnectionUpdate =
          statusMessage === '✓ Reconnected' ||
          statusMessage === '✗ Reconnection failed';
        setConversation(prev => {
          if (isReconnectionUpdate) {
            const idx = prev.findLastIndex(
              m => m.type === 'status' && m.content === '⟳ Reconnecting...'
            );
            if (idx !== -1) {
              const updated = [...prev];
              updated[idx] = { type: 'status', content: statusMessage };
              return updated;
            }
          }
          return [
            ...prev,
            { type: 'status', content: statusMessage },
          ];
        });
      }
    } else if (chunk.type === 'Interrupted') {
      setConversation(prev => {
        const updated = [...prev];
        // Mark streaming as interrupted, or remove if empty
        const streamingIdx = updated.findLastIndex(m => m.isStreaming);
        if (streamingIdx >= 0) {
          if (updated[streamingIdx].content.trim() === '') {
            updated.splice(streamingIdx, 1);
          } else {
            updated[streamingIdx] = {
              ...updated[streamingIdx],
              isStreaming: false,
            };
          }
        }
        updated.push({ type: 'status', content: '⚠ Interrupted' });
        return updated;
      });
      refreshRustState(currentSessionIdRef.current);
    } else if (
      chunk.type === 'TokenUpdate' ||
      chunk.type === 'ContextFillUpdate'
    ) {
      // TUI-049: Use centralized helper for token state updates (DRY)
      updateTokenStateFromChunk(chunk);
    } else if (chunk.type === 'ToolProgress' && chunk.toolProgress) {
      // Mark stderr output with special prefix for red rendering
      const isStderr = chunk.toolProgress.isStderr;
      const rawChunk = chunk.toolProgress.outputChunk;
      const outputChunk = isStderr
        ? rawChunk
            .split('\n')
            .map(line => (line ? `⚠stderr⚠${line}` : line))
            .join('\n')
        : rawChunk;
      setConversation(prev => {
        const updated = [...prev];
        const lastIdx = updated.length - 1;
        if (lastIdx >= 0) {
          const lastMsg = updated[lastIdx];
          if (lastMsg.type === 'tool-call' && lastMsg.content.startsWith('●')) {
            const lines = lastMsg.content.split('\n');
            const header = lines[0];
            const existingOutput = lines
              .slice(1)
              .map(l =>
                l.startsWith('L ')
                  ? l.slice(2)
                  : l.startsWith('  ')
                    ? l.slice(2)
                    : l
              )
              .join('\n');
            const newOutput = existingOutput + outputChunk;
            const windowedOutput = createStreamingWindow(newOutput);
            const windowedLines = windowedOutput.split('\n');
            const formattedOutput = windowedLines
              .map((l, i) => (i === 0 ? `L ${l}` : `  ${l}`))
              .join('\n');
            updated[lastIdx] = {
              ...lastMsg,
              content: `${header}\n${formattedOutput}`,
            };
          }
        }
        return updated;
      });
    } else if (chunk.type === 'Error' && chunk.error) {
      // Log the error
      logger.error(`Stream error: ${chunk.error}`);

      // Show error in modal for user visibility
      setError(chunk.error);

      setConversation(prev => {
        const updated = [...prev];
        // Remove empty streaming messages
        while (
          updated.length > 0 &&
          updated[updated.length - 1].type === 'assistant-text' &&
          updated[updated.length - 1].isStreaming &&
          !updated[updated.length - 1].content
        ) {
          updated.pop();
        }
        updated.push({ type: 'status', content: `API Error: ${chunk.error}` });
        return updated;
      });
      refreshRustState(currentSessionIdRef.current);
    } else if (chunk.type === 'UserInput' && chunk.text) {
      // User input from buffer replay (NAPI-009: resume/attach)
      setConversation(prev => [
        ...prev,
        { type: 'user-input', content: chunk.text! },
      ]);
    } else if (chunk.type === 'IncomingMessage' && chunk.text) {
      // WATCH-012: Handle supervisor/bridge input messages - parse prefix and format for display
      const supervisorInfo = parseSupervisorPrefix(chunk.text);
      setConversation(prev => {
        if (supervisorInfo) {
          // Format content with role prefix (no emoji)
          const formattedContent = `[W] ${supervisorInfo.role}> ${supervisorInfo.content}`;
          return [
            ...prev,
            { type: 'supervisor-input', content: formattedContent },
          ];
        } else {
          // Fallback: if parsing fails, display raw message
          return [...prev, { type: 'supervisor-input', content: chunk.text! }];
        }
      });
    }
    // REFAC-008: FspecCommandRequest is handled globally by GlobalSessionStreamManager
    // AgentView no longer processes FspecCommandRequest chunks - they are filtered out
    // by the manager before reaching session handlers
  }, []);

  // SESS-001: Shared function to resume a session by ID (used by /resume, auto-resume, and VIEWNV-001 navigation)
  // UNIFIED: Both background and persisted sessions use the same chunk-based restore flow.
  // NOTE: Defined before sessionNavigation to avoid closure issues with callback references
  const resumeSessionById = useCallback(
    async (sessionId: string): Promise<boolean> => {
      try {
        // Use the session service to handle restoration
        const result = await restoreSession({
          sessionId,
          fallbackModelPath: currentProvider,
          fallbackProject: currentProjectRef.current,
          // Note: We don't pass onStreamChunk here because we need to do UI setup first
        });

        logger.debug(
          `SESS-001: ${result.wasBackgroundSession ? 'Resumed existing background' : 'Restored persisted'} session ${sessionId}`
        );

        // Update token state from service result if available
        if (result.tokenUsage) {
          setTokenUsage({
            inputTokens: result.tokenUsage.currentContextTokens,
            outputTokens: result.tokenUsage.cumulativeBilledOutput,
            cacheReadInputTokens: result.tokenUsage.cacheReadTokens,
            cacheCreationInputTokens: result.tokenUsage.cacheCreationTokens,
          });
        }

        // Update provider state if available
        // PROV-007: Use parseModelString for profile-aware model string parsing
        if (result.provider?.includes('/')) {
          try {
            const parsed = parseModelString(result.provider);
            const { providerId, profileName, modelId } = parsed;
            const internalName = mapProviderIdToInternal(providerId);
            setCurrentProvider(internalName);
            // PROV-007: Find matching section by both providerId AND profileName
            const section = findSectionForPersistedModel(
              providerSections,
              result.provider
            );
            const model = section?.models.find(
              m => normalizeModelIdForMatch(m.id) === modelId
            );
            if (model && section) {
              setCurrentModel({
                providerId,
                modelId,
                apiModelId: model.id,
                displayName: model.name,
                reasoning: model.reasoning,
                hasVision: model.hasVision,
                contextWindow: model.contextWindow,
                maxOutput: model.maxOutput,
                // PROV-007: Include profile name and config for session operations
                profileName,
                profileConfig: section.profileConfig,
              });
            }
          } catch {
            // Invalid model string format - ignore
            logger.warn(
              `Invalid model string format in restore: ${result.provider}`
            );
          }
        }

        // UNIFIED: Get merged output and convert to conversation messages
        const mergedChunks = sessionGetMergedOutput(sessionId);
        const restoredMessages = processChunksToConversation(
          mergedChunks,
          formatToolHeader,
          formatCollapsedOutput
        );
        setConversation(restoredMessages);

        // For background sessions, extract token state from chunks
        if (result.wasBackgroundSession) {
          const extractedState = extractTokenStateFromChunks(mergedChunks);
          if (extractedState.tokenUsage) {
            setTokenUsage(extractedState.tokenUsage);
          }
          if (extractedState.contextFillPercentage !== null) {
            setContextFillPercentage(extractedState.contextFillPercentage);
          }
        }

        // REFAC-008: Cleanup previous handler before attaching to new session
        cleanupCurrentSessionHandler();

        // REFAC-008: Attach via GlobalSessionStreamManager and track cleanup
        sessionCleanupRef.current = attachToSession(
          sessionId,
          (chunk: StreamChunk) => {
            handleStreamChunk(chunk);
          }
        );

        // Update session state (atomic transition via store)
        activateSession(sessionId);

        // GIT-029: Apply any pending isolation state that arrived before activation
        applyPendingIsolationState(sessionId);

        // TUI-052: Restore pending input if available
        try {
          const pendingInput = sessionGetPendingInput(sessionId);
          // Always restore input (even if empty) to avoid showing wrong session's input
          setInputValue(pendingInput || '');
        } catch {
          // Session may not have pending input, ignore
        }

        return true;
      } catch (err) {
        logger.error(
          `SESS-001: Failed to resume session: ${err instanceof Error ? err.message : String(err)}`
        );
        return false;
      }
    },
    [handleStreamChunk, currentProvider, providerSections, activateSession]
  );

  // VIEWNV-001: Handle create session dialog confirmation
  // GIT-029: Now accepts isolated parameter to create isolated session with git worktree
  // Creates session immediately so /thinking and other commands work right away
  const handleCreateSessionConfirm = useCallback(
    async (isolated: boolean = false) => {
      // Wait for models to be initialized before creating session
      // This prevents race condition where session is created with incomplete model info
      if (!modelsInitialized) {
        logger.warn('Models not yet initialized, waiting...');
        // Return early - user can try again once models are loaded
        return;
      }

      // Save reference to current session before detaching (to detect navigation context)
      const wasInSession = !!currentSessionId;

      // REFAC-008: Cleanup current handler before creating new session
      cleanupCurrentSessionHandler();

      try {
        const project = currentProjectRef.current;

        // Require currentModel to be set - throw if not
        if (!currentModel) {
          throw new Error('Cannot create session: model not initialized');
        }

        // PROV-008: Set env vars from profile config before session creation
        if (currentModel.profileConfig) {
          configureProfileEnvironment(currentModel.profileConfig);
        }

        // PROV-007: Use buildModelString for profile-qualified model paths
        // Profile models: 'provider:profile/modelId' (e.g., 'openai:work-vllm/Qwen3-80B')
        // Cloud models: 'provider/modelId' (e.g., 'openai/gpt-4')
        const modelPath = buildModelString(
          {
            providerId: currentModel.providerId,
            profileName: currentModel.profileName,
          },
          currentModel.modelId
        );

        // GIT-029: Use isolated session creation when requested
        // MODEL-005: Pass modelSelection to propagate per-model context window and max output
        let result;
        if (isolated) {
          result = await createIsolatedSession({
            modelPath,
            project,
            modelSelection: {
              providerId: currentModel.providerId,
              modelId: currentModel.modelId,
              contextWindow: currentModel.contextWindow,
              maxOutput: currentModel.maxOutput,
              profileConfig: currentModel.profileConfig,
              facade: currentModel.facade,
            },
          });
        } else {
          result = await createSession({
            modelPath,
            project,
            modelSelection: {
              providerId: currentModel.providerId,
              modelId: currentModel.modelId,
              contextWindow: currentModel.contextWindow,
              maxOutput: currentModel.maxOutput,
              profileConfig: currentModel.profileConfig,
              facade: currentModel.facade,
            },
          });
        }

        // Activate the session in the store
        activateSession(result.sessionId);

        // GIT-029: Apply any pending isolation state that arrived before activation
        applyPendingIsolationState(result.sessionId);

        // TUI-075: Default thinking level is applied automatically by useDefaultThinkingLevel
        // hook when currentSessionId changes after activateSession

        // SESS-001: Only auto-attach session to work unit when creating from board context
        // If we were in a session, we're creating via navigation (Shift+Right) and shouldn't auto-attach
        if (workUnitId && !wasInSession) {
          // TUI-068: Use sessionService facade for attachment
          // TUI-069: Pass work unit title to avoid hardcoded placeholder
          const workUnit = workUnits.find(wu => wu.id === workUnitId);
          attachToWorkUnit(
            result.sessionId,
            workUnitId,
            workUnit?.status ?? 'backlog',
            workUnit?.title
          );
          logger.debug(
            `SESS-001: Attached session ${result.sessionId} to work unit ${workUnitId}`
          );
        } else if (workUnitId && wasInSession) {
          logger.debug(
            `SESS-001: Skipped auto-attach for navigation-created session ${result.sessionId} (created via Shift+Right)`
          );
        }

        // Clear conversation and input for the new session
        setConversation([]);
        setInputValue('');

        // Close the dialog
        closeCreateSessionDialog();

        logger.debug(`VIEWNV-001: Created new session ${result.sessionId}`);
      } catch (err) {
        logger.error('Failed to create new session:', err);
        // Fall back to old behavior if creation fails
        prepareForNewSession();
        setConversation([]);
        setInputValue('');
        closeCreateSessionDialog();
      }
    },
    [
      currentSessionId,
      currentModel,
      modelsInitialized,
      activateSession,
      closeCreateSessionDialog,
      prepareForNewSession,
      workUnitId,
      workUnits,
      // TUI-068: attachToWorkUnit is a module-level import (stable)
    ]
  );

  // VIEWNV-001: Unified session navigation hook for Shift+Arrow navigation
  // This provides the navigation logic that determines targets based on the session tree
  // Note: Hook gets currentSessionId from store and uses store action for create dialog
  const sessionNavigation = useSessionNavigation({
    onNavigate: async (targetSessionId: string) => {
      // Switch to the target session using existing resumeSessionById
      // TUI-053: ALWAYS save pending input before switching (even if empty)
      // This prevents showing wrong session's input after switching
      if (currentSessionId) {
        try {
          sessionSetPendingInput(currentSessionId, inputValue);
        } catch (err) {
          // Failed to set pending input before switching - indicates session management issues
          logger.error(
            'Failed to set pending input before session switch:',
            err
          );
        }
      }

      // REFAC-008: Cleanup current handler before navigating
      // Note: resumeSessionById will also call cleanup, but calling twice is safe
      cleanupCurrentSessionHandler();

      // Resume the target session
      await resumeSessionById(targetSessionId);
    },
    onNavigateToBoard: () => {
      // REFAC-008: Cleanup current handler before exiting to board
      cleanupCurrentSessionHandler();
      onExit();
    },
    // Note: Create dialog is now handled by the hook via store action (openCreateSessionDialog)
  });

  // SESS-001: Auto-resume attached session on mount
  useEffect(() => {
    const sessionIdToResume = needsAutoResumeRef.current;
    if (!sessionIdToResume) return;

    // Clear ref so we don't resume again
    needsAutoResumeRef.current = null;

    void resumeSessionById(sessionIdToResume);
  }, [resumeSessionById]);

  // VIEWNV-001: Auto-resume session from navigation (initialSessionId prop)
  // Track if we need to auto-resume from initialSessionId
  const needsInitialSessionResumeRef = useRef<string | null>(
    initialSessionId ?? null
  );

  // VIEWNV-001: Auto-resume initial session on mount
  useEffect(() => {
    const sessionIdToResume = needsInitialSessionResumeRef.current;
    if (!sessionIdToResume) return;

    // Clear ref so we don't resume again
    needsInitialSessionResumeRef.current = null;

    void resumeSessionById(sessionIdToResume);
  }, [resumeSessionById]);

  // VIEWNV-001: Auto-create session when user confirms "Start New Agent?" dialog
  // This is triggered by shouldAutoCreateSession being set to true
  // The session is created immediately so /thinking and other commands work right away
  // Track if session needs renaming on first message (auto-created with generic name)
  const sessionNeedsRenameRef = useRef(false);
  useEffect(() => {
    // Only auto-create if explicitly requested via dialog confirmation
    if (!shouldAutoCreateSession || currentSessionId) {
      return;
    }

    // Wait for models to be initialized before creating session
    // This prevents race condition where session is created with incomplete model info
    if (!modelsInitialized) {
      logger.debug(
        'Waiting for models to initialize before auto-creating session'
      );
      return;
    }

    // SESS-001: Don't auto-create if there's an attached session that should be resumed
    if (workUnitId) {
      const attachedSessionId = getAttachedSession(workUnitId);
      if (attachedSessionId) {
        logger.debug(
          `SESS-001: Skipping auto-create because work unit ${workUnitId} has attached session ${attachedSessionId} that will be resumed`
        );
        clearAutoCreateRequest();
        return;
      }
    }

    // Clear the request immediately to prevent double-creation
    clearAutoCreateRequest();

    const autoCreateSession = async () => {
      try {
        const project = currentProjectRef.current;

        // Require currentModel to be set - throw if not
        if (!currentModel) {
          throw new Error('Cannot auto-create session: model not initialized');
        }

        // PROV-008: Set env vars from profile config before session creation
        if (currentModel.profileConfig) {
          configureProfileEnvironment(currentModel.profileConfig);
          logger.debug(
            `PROV-008: Set env vars from profile config before auto-create session`
          );
        }

        // PROV-007: Use buildModelString for profile-qualified model paths
        // Profile models: 'provider:profile/modelId' (e.g., 'openai:work-vllm/Qwen3-80B')
        // Cloud models: 'provider/modelId' (e.g., 'openai/gpt-4')
        const modelPath = buildModelString(
          {
            providerId: currentModel.providerId,
            profileName: currentModel.profileName,
          },
          currentModel.modelId
        );

        // GIT-031: Use pendingIsolatedSession to determine if isolated session should be created
        // MODEL-005: Pass modelSelection to propagate per-model context window and max output
        let result;
        if (pendingIsolatedSession) {
          result = await createIsolatedSession({
            modelPath,
            project,
            modelSelection: {
              providerId: currentModel.providerId,
              modelId: currentModel.modelId,
              contextWindow: currentModel.contextWindow,
              maxOutput: currentModel.maxOutput,
              profileConfig: currentModel.profileConfig,
              facade: currentModel.facade,
            },
          });
          logger.debug(
            `GIT-031: Auto-created isolated session ${result.sessionId} at ${result.worktreePath}`
          );
        } else {
          result = await createSession({
            modelPath,
            project,
            modelSelection: {
              providerId: currentModel.providerId,
              modelId: currentModel.modelId,
              contextWindow: currentModel.contextWindow,
              maxOutput: currentModel.maxOutput,
              profileConfig: currentModel.profileConfig,
              facade: currentModel.facade,
            },
          });
        }

        activateSession(result.sessionId);

        // GIT-029: Apply any pending isolation state that arrived before activation
        applyPendingIsolationState(result.sessionId);

        // TUI-075: Default thinking level is applied automatically by useDefaultThinkingLevel
        // hook when currentSessionId changes after activateSession

        // SESS-001: Auto-attach session to work unit when auto-creating
        if (workUnitId) {
          // TUI-068: Use sessionService facade for attachment
          // TUI-069: Pass work unit title to avoid hardcoded placeholder
          const workUnit = workUnits.find(wu => wu.id === workUnitId);
          attachToWorkUnit(
            result.sessionId,
            workUnitId,
            workUnit?.status ?? 'backlog',
            workUnit?.title
          );
          logger.debug(
            `SESS-001: Attached session ${result.sessionId} to work unit ${workUnitId}`
          );
        }

        // Mark that this session needs renaming on first message
        sessionNeedsRenameRef.current = true;
        logger.debug(
          `VIEWNV-001: Auto-created session ${result.sessionId} on AgentView mount`
        );
      } catch (err) {
        logger.error('Failed to auto-create session:', err);
      }
    };

    void autoCreateSession();
  }, [
    shouldAutoCreateSession,
    pendingIsolatedSession,
    currentSessionId,
    currentModel,
    modelsInitialized,
    activateSession,
    clearAutoCreateRequest,
    workUnitId,
    workUnits,
    // TUI-068: attachToWorkUnit is a module-level import (stable)
    getAttachedSession,
  ]);

  // NAPI-003 + TUI-047: Enter resume mode (show session selection overlay)
  // Now queries both persistence and background sessions, merging results
  const handleResumeMode = useCallback(async () => {
    try {
      // Get persisted sessions
      const persistedSessions = persistenceListSessions(
        currentProjectRef.current
      );

      // TUI-047: Get background sessions
      const backgroundSessions = sessionManagerList();

      // TUI-047: Merge sessions - background takes precedence
      const backgroundMap = new Map<string, { status: string }>();
      for (const bg of backgroundSessions) {
        backgroundMap.set(bg.id, { status: bg.status });
      }

      // Convert persisted sessions to MergedSession, marking those with background processes
      const mergedSessions: MergedSession[] = persistedSessions.map(
        (session: SessionManifest) => {
          const bgInfo = backgroundMap.get(session.id);
          if (bgInfo) {
            // Session exists in background - use background status
            return {
              ...session,
              isBackgroundSession: true,
              backgroundStatus: bgInfo.status as 'running' | 'idle',
            };
          }
          // Persisted-only session
          return {
            ...session,
            isBackgroundSession: false,
            backgroundStatus: null,
          };
        }
      );

      // Add any background sessions that aren't in persistence yet
      for (const bg of backgroundSessions) {
        if (!persistedSessions.find((p: SessionManifest) => p.id === bg.id)) {
          // Build provider string from background session's providerId/modelId
          const providerString =
            bg.providerId && bg.modelId
              ? `${bg.providerId}/${bg.modelId}`
              : bg.providerId || 'unknown';
          mergedSessions.push({
            id: bg.id,
            name: bg.name || 'Background Session',
            project: bg.project || currentProjectRef.current,
            provider: providerString,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            messageCount: bg.messageCount || 0,
            isBackgroundSession: true,
            backgroundStatus: bg.status as 'running' | 'idle',
          });
        }
      }

      // Sort by updatedAt descending (most recent first)
      const sorted = [...mergedSessions].sort(
        (a: MergedSession, b: MergedSession) =>
          new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      );

      setAvailableSessions(sorted);
      setResumeSessionIndex(0);
      setIsResumeMode(true);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to list sessions';
      setConversation(prev => [
        ...prev,
        { type: 'status', content: `Resume failed: ${errorMessage}` },
      ]);
    }
  }, []);

  // BLOCK-004: Enter blocklist mode (show blocklist management overlay)
  const handleBlocklistMode = useCallback(async () => {
    try {
      // Load blocklist rules from system and project configs
      const config = blocklistLoad(process.cwd());

      // Map rules with source information
      const rules: BlocklistRule[] = config.rules.map(rule => ({
        id: rule.id,
        pattern: rule.pattern,
        action: rule.action,
        reason: rule.reason,
        guidance: rule.guidance ?? undefined,
        // TODO: Could track source (system vs project) if needed
        source: 'project' as const,
      }));

      setBlocklistRules(rules);
      setIsBlocklistMode(true);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to load blocklist';
      setConversation(prev => [
        ...prev,
        {
          type: 'status',
          content: `Blocklist error: ${errorMessage}`,
        },
      ]);
    }
  }, []);

  // BLOCK-004: Toggle a rule's session state
  const handleToggleBlocklistRule = useCallback((ruleId: string) => {
    setDisabledBlocklistRules(prev => {
      const next = new Set(prev);
      if (next.has(ruleId)) {
        next.delete(ruleId);
      } else {
        next.add(ruleId);
      }
      return next;
    });
  }, []);

  // TUI-050: Trigger useEffect for resume mode initialization (called from handleSubmitWithCommand)
  useEffect(() => {
    if (triggerResumeModeInit) {
      setTriggerResumeModeInit(false);
      void handleResumeMode();
    }
  }, [triggerResumeModeInit, handleResumeMode]);

  // NAPI-003 + TUI-047: Select session and restore conversation
  // UNIFIED: Both background and persisted sessions now use the same chunk-based restore flow.
  // For persisted-only sessions, we first create a background session and restore messages,
  // then use sessionGetMergedOutput() → processChunksToConversation() (same as background).
  const handleResumeSelect = useCallback(async () => {
    if (
      availableSessions.length === 0 ||
      resumeSessionIndex >= availableSessions.length
    ) {
      return;
    }

    const selectedSession = availableSessions[resumeSessionIndex];

    try {
      // Use the session service to handle restoration
      const result = await restoreSession({
        sessionId: selectedSession.id,
        fallbackModelPath: currentProvider,
        fallbackProject: currentProjectRef.current,
        // Pass the session data to avoid unnecessary persistence lookup
        sessionData: {
          name: selectedSession.name,
          provider: selectedSession.provider,
          tokenUsage: selectedSession.tokenUsage,
        },
        // Note: We don't pass onStreamChunk here because we need to do UI setup first
      });

      logger.debug(
        `NAPI-003: ${result.wasBackgroundSession ? 'Resumed existing background' : 'Restored persisted'} session ${selectedSession.id}`
      );

      // Update provider/model state from service result
      // PROV-007: Use parseModelString for profile-aware model string parsing
      if (result.provider?.includes('/')) {
        try {
          const parsed = parseModelString(result.provider);
          const { providerId, profileName, modelId } = parsed;
          const internalName = mapProviderIdToInternal(providerId);
          setCurrentProvider(internalName);
          // PROV-007: Find matching section by both providerId AND profileName
          const section = findSectionForPersistedModel(
            providerSections,
            result.provider
          );
          const model = section?.models.find(
            m => normalizeModelIdForMatch(m.id) === modelId
          );
          if (model && section) {
            setCurrentModel({
              providerId,
              modelId,
              apiModelId: model.id,
              displayName: model.name,
              reasoning: model.reasoning,
              hasVision: model.hasVision,
              contextWindow: model.contextWindow,
              maxOutput: model.maxOutput,
              // PROV-007: Include profile name and config for session operations
              profileName,
              profileConfig: section.profileConfig,
            });
          }
        } catch {
          // Invalid model string format - ignore
          logger.warn(
            `Invalid model string format in resume: ${result.provider}`
          );
        }
      } else if (result.provider) {
        setCurrentProvider(result.provider);
      }

      // UNIFIED: Get merged output and convert to conversation messages
      // For background sessions: output_buffer has live streaming history
      // For persisted sessions: output_buffer populated by sessionRestoreMessages()
      const mergedChunks = sessionGetMergedOutput(selectedSession.id);
      const restoredMessages = processChunksToConversation(
        mergedChunks,
        formatToolHeader,
        formatCollapsedOutput
      );
      setConversation(restoredMessages);

      // Extract token state from chunks (for background sessions)
      // For persisted sessions, prefer service result data which has accurate cumulative values
      if (result.wasBackgroundSession) {
        const extractedState = extractTokenStateFromChunks(mergedChunks);
        if (extractedState.tokenUsage) {
          setTokenUsage(extractedState.tokenUsage);
        }
        if (extractedState.contextFillPercentage !== null) {
          setContextFillPercentage(extractedState.contextFillPercentage);
        }
      } else if (result.tokenUsage) {
        // Use service result token data for persisted sessions
        setTokenUsage({
          inputTokens: result.tokenUsage.currentContextTokens,
          outputTokens: result.tokenUsage.cumulativeBilledOutput,
          cacheReadInputTokens: result.tokenUsage.cacheReadTokens,
          cacheCreationInputTokens: result.tokenUsage.cacheCreationTokens,
        });

        // Calculate context fill percentage from model info
        if (result.provider?.includes('/')) {
          const [providerId, modelId] = result.provider.split('/');
          const section = providerSections.find(
            s => s.providerId === providerId
          );
          const model = section?.models.find(
            m => normalizeModelIdForMatch(m.id) === modelId
          );
          if (model) {
            const fillPercentage = calculateContextFillPercentage(
              result.tokenUsage.currentContextTokens,
              model.contextWindow,
              model.maxOutput
            );
            setContextFillPercentage(fillPercentage);
          }
        }
      }

      // REFAC-008: Attach via GlobalSessionStreamManager and track cleanup
      // Cleanup is done inside try block before attaching
      cleanupCurrentSessionHandler();

      // REFAC-008: Attach via GlobalSessionStreamManager and track cleanup
      sessionCleanupRef.current = attachToSession(
        selectedSession.id,
        (chunk: StreamChunk) => {
          handleStreamChunk(chunk);
        }
      );

      // Update session state (atomic transition via store)
      // Note: activateSession sets both currentSessionId and isReadyForNewSession=false atomically
      activateSession(selectedSession.id);

      // GIT-029: Apply any pending isolation state that arrived before activation
      applyPendingIsolationState(selectedSession.id);

      setIsResumeMode(false);
      setAvailableSessions([]);
      setResumeSessionIndex(0);

      // SESS-001: Attach resumed session to work unit
      if (workUnitId) {
        // TUI-068: Use sessionService facade for attachment
        // TUI-069: Pass work unit title to avoid hardcoded placeholder
        const workUnit = workUnits.find(wu => wu.id === workUnitId);
        attachToWorkUnit(
          selectedSession.id,
          workUnitId,
          workUnit?.status ?? 'backlog',
          workUnit?.title
        );
        logger.debug(
          `SESS-001: Attached resumed session ${selectedSession.id} to work unit ${workUnitId}`
        );
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to restore session';
      setConversation(prev => [
        ...prev,
        { type: 'status', content: `Resume failed: ${errorMessage}` },
      ]);
      setIsResumeMode(false);
      setAvailableSessions([]);
      setResumeSessionIndex(0);
    }
  }, [
    availableSessions,
    resumeSessionIndex,
    handleStreamChunk,
    activateSession,
  ]);

  // NAPI-003: Cancel resume mode
  const handleResumeCancel = useCallback(() => {
    setIsResumeMode(false);
    setAvailableSessions([]);
    setResumeSessionIndex(0);
    setResumeScrollOffset(0);
    setShowSessionDeleteDialog(false);
  }, []);

  // TUI-040 + TUI-047: Handle session delete dialog selection
  // Now handles both background sessions (destroy) and persisted-only (delete from disk)
  const handleSessionDeleteSelect = useCallback(
    async (index: number, option: string) => {
      setShowSessionDeleteDialog(false);

      if (option === 'Cancel') {
        return;
      }

      try {
        if (option === 'Delete This Session') {
          // Delete single session
          const selectedSession = availableSessions[resumeSessionIndex];
          if (selectedSession) {
            // TUI-047: Check if background session - destroy it first
            // TUI-068: Use destroySession from sessionService
            if (selectedSession.isBackgroundSession) {
              await destroySession(selectedSession.id);
            }
            // Always delete from persistence too
            await persistenceDeleteSession(selectedSession.id);
            // Cleanup orphaned messages
            persistenceCleanupOrphanedMessages();
            // Refresh session list using the merged approach
            await handleResumeMode();
            return; // handleResumeMode handles state updates
          }
        } else if (option === 'Delete ALL Sessions') {
          // Delete all sessions
          for (const session of availableSessions) {
            // TUI-047: Destroy background sessions first
            // TUI-068: Use destroySession from sessionService
            if (session.isBackgroundSession) {
              await destroySession(session.id);
            }
            await persistenceDeleteSession(session.id);
          }
          // Cleanup orphaned messages
          persistenceCleanupOrphanedMessages();
          setAvailableSessions([]);
          setIsResumeMode(false);
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to delete session';
        setConversation(prev => [
          ...prev,
          { type: 'status', content: `Delete failed: ${errorMessage}` },
        ]);
      }
    },
    [availableSessions, resumeSessionIndex]
  );

  // TUI-040: Cancel session delete dialog
  const handleSessionDeleteCancel = useCallback(() => {
    setShowSessionDeleteDialog(false);
  }, []);

  // TUI-046: Handle exit confirmation modal selection (Detach/Close Session/Cancel)
  const handleExitChoice = useCallback(
    async (index: number, _option: string) => {
      setShowExitConfirmation(false);

      if (index === 2) {
        // Cancel - stay in AgentView
        return;
      }

      if (index === 0) {
        // Detach - session continues running in background
        // REFAC-008: Cleanup handler - session stays subscribed via GlobalSessionStreamManager
        cleanupCurrentSessionHandler();
        onExit();
      } else if (index === 1) {
        // Close Session - terminate the session
        // REFAC-008: Cleanup handler before destroying session
        cleanupCurrentSessionHandler();
        if (currentSessionId) {
          try {
            // TUI-068: Use destroySession from sessionService
            // destroySession already handles detaching from work unit internally
            await destroySession(currentSessionId);
            logger.debug(
              `SESS-001: Session ${currentSessionId} destroyed and detached from work unit`
            );
          } catch (err) {
            // Log but continue - session may not be in background manager
            logger.error('Failed to destroy session:', err);
          }
        }
        onExit();
      }
    },
    [currentSessionId, onExit]
  );

  // Mouse scroll acceleration state (like VirtualList)
  const resumeLastScrollTime = useRef<number>(0);
  const resumeScrollVelocity = useRef<number>(1);

  // TUI-074: Settings tab scroll now handled by ProviderSettingsScreen

  // Mouse scroll navigation helper for resume mode
  const navigateResumeByDelta = useCallback(
    (delta: number) => {
      if (availableSessions.length === 0) return;

      // Acceleration: scroll faster when scrolling rapidly
      const now = Date.now();
      const timeDelta = now - resumeLastScrollTime.current;
      if (timeDelta < 150) {
        resumeScrollVelocity.current = Math.min(
          resumeScrollVelocity.current + 1,
          5
        );
      } else {
        resumeScrollVelocity.current = 1;
      }
      resumeLastScrollTime.current = now;
      const scrollAmount = resumeScrollVelocity.current * delta;

      setResumeSessionIndex(prev =>
        Math.max(0, Math.min(availableSessions.length - 1, prev + scrollAmount))
      );
    },
    [availableSessions.length]
  );

  // PAUSE-001: Handle keyboard input during pause state with HIGH priority
  useInputCompat({
    id: 'agent-view-pause',
    priority: InputPriority.HIGH,
    description:
      'Agent view pause keyboard handler (Enter to resume, Y/N for confirm)',
    isActive: displayIsPaused && currentSessionId !== null,
    handler: (input, key) => {
      if (!currentSessionId || !displayPauseInfo) {
        return false;
      }

      // Handle Continue pause (Enter to resume)
      if (displayPauseInfo.kind === 'continue') {
        if (key.return) {
          try {
            sessionPauseResume(currentSessionId);
          } catch (e) {
            logger.error('[PAUSE-001] Error resuming pause:', e);
          }
          return true;
        }
        // Esc interrupts the pause (tool will receive Interrupted response)
        // This is handled by the existing Esc handler for session interrupt
        return false;
      }

      // Handle Confirm pause (Y to approve, N to deny)
      if (displayPauseInfo.kind === 'confirm') {
        if (input.toLowerCase() === 'y') {
          try {
            sessionPauseConfirm(currentSessionId, true);
          } catch (e) {
            logger.error('[PAUSE-001] Error confirming pause (approve):', e);
          }
          return true;
        }
        if (input.toLowerCase() === 'n' || key.escape) {
          try {
            sessionPauseConfirm(currentSessionId, false);
          } catch (e) {
            logger.error('[PAUSE-001] Error confirming pause (deny):', e);
          }
          return true;
        }
        return false;
      }

      // Handle Triple pause (←/→ to navigate, Enter to select, Esc to deny)
      if (displayPauseInfo.kind === 'triple') {
        // Left arrow: move selection left
        if (key.leftArrow) {
          setTriplePauseSelection(prev => (prev > 0 ? prev - 1 : 2));
          return true;
        }
        // Right arrow: move selection right
        if (key.rightArrow) {
          setTriplePauseSelection(prev => (prev < 2 ? prev + 1 : 0));
          return true;
        }
        // Enter: confirm current selection
        if (key.return) {
          try {
            const choices = ['allow_once', 'allow_session', 'deny'];
            sessionPauseTriple(currentSessionId, choices[triplePauseSelection]);
            setTriplePauseSelection(0); // Reset for next pause
          } catch (e) {
            logger.error('[BLOCK-007] Error sending triple pause response:', e);
          }
          return true;
        }
        // Esc: deny (same as selecting Deny option)
        if (key.escape) {
          try {
            sessionPauseTriple(currentSessionId, 'deny');
            setTriplePauseSelection(0); // Reset for next pause
          } catch (e) {
            logger.error('[BLOCK-007] Error sending triple pause deny:', e);
          }
          return true;
        }
        return false;
      }

      return false;
    },
  });

  // Handle keyboard input with LOW priority (main view navigation)
  useInputCompat({
    id: 'agent-view-main',
    priority: InputPriority.LOW,
    description: 'Agent view main keyboard handler',
    isActive: !showCreateSessionDialog,
    handler: (input, key) => {
      // BUG-131: Parse SGR mouse events
      const mouseEvent = parseSgrMouse(input);
      if (mouseEvent) {
        // SGR mouse scroll for resume mode
        // Note: TUI-042 turn selection scroll is handled by VirtualList via getNextIndex
        if (isResumeMode) {
          if (mouseEvent.button === SGR_BUTTON.SCROLL_UP) {
            navigateResumeByDelta(-1);
            return true;
          } else if (mouseEvent.button === SGR_BUTTON.SCROLL_DOWN) {
            navigateResumeByDelta(1);
            return true;
          }
        }
        // TUI-074: Settings tab mouse scroll now handled by ProviderSettingsScreen
        // Let unhandled mouse events propagate to VirtualList (BACKGROUND priority)
        // This allows conversation scrolling when not in a modal/overlay mode
        return false;
      }

      // NAPI-006: Search mode keyboard handling
      if (isSearchMode) {
        if (key.escape) {
          handleSearchCancel();
          return true;
        }
        if (key.return) {
          handleSearchSelect();
          return true;
        }
        if (key.upArrow) {
          setSearchResultIndex(prev => Math.max(0, prev - 1));
          return true;
        }
        if (key.downArrow) {
          setSearchResultIndex(prev =>
            Math.min(searchResults.length - 1, prev + 1)
          );
          return true;
        }
        if (key.backspace || key.delete) {
          void handleSearchInput(searchQuery.slice(0, -1));
          return true;
        }
        // Accept printable characters for search query
        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code <= 126;
          })
          .join('');
        if (clean) {
          void handleSearchInput(searchQuery + clean);
        }
        return true;
      }

      // TUI-050: Slash command palette keyboard handling
      // The hook handles all the complexity internally and returns true if it handled the input
      if (slashCommand.handleInput(input, key)) {
        return true;
      }

      // TUI-055: File search popup keyboard handling (exact same architecture as slash commands)
      if (fileSearch.handleInput(input, key)) {
        return true;
      }

      // TUI-050: Handle Enter for slash commands even if palette wasn't shown yet
      // (e.g., user types "/debug" and presses Enter before palette could render)
      // IMPORTANT: Only do this when NOT in another mode (resume, etc.)
      // Otherwise Enter gets incorrectly captured when user is selecting from a list
      if (
        key.return &&
        inputValue.startsWith('/') &&
        inputValue.trim().length > 1 &&
        !isResumeMode &&
        !showModelSelector &&
        !showSettingsTab
      ) {
        void handleSubmitWithCommand(inputValue.trim());
        return true;
      }

      // TUI-055: Handle @ symbol for file search - simple detection like slash commands

      // NAPI-003: Resume mode keyboard handling
      if (isResumeMode) {
        // TUI-040: Handle delete dialog keyboard input first
        if (showSessionDeleteDialog) {
          // Dialog handles its own input via useInput
          return true;
        }
        if (key.escape) {
          handleResumeCancel();
          return true;
        }
        if (key.return) {
          void handleResumeSelect();
          return true;
        }
        if (key.upArrow) {
          setResumeSessionIndex(prev => Math.max(0, prev - 1));
          return true;
        }
        if (key.downArrow) {
          setResumeSessionIndex(prev =>
            Math.min(availableSessions.length - 1, prev + 1)
          );
          return true;
        }
        // TUI-040: D key opens delete confirmation dialog
        if (input.toLowerCase() === 'd' && availableSessions.length > 0) {
          setShowSessionDeleteDialog(true);
          return true;
        }
        // No text input in resume mode - just navigation
        return true;
      }

      if (showProviderSelector) {
        if (key.escape) {
          setShowProviderSelector(false);
          return true;
        }
        if (key.upArrow) {
          setSelectedProviderIndex(prev =>
            prev > 0 ? prev - 1 : availableProviders.length - 1
          );
          return true;
        }
        if (key.downArrow) {
          setSelectedProviderIndex(prev =>
            prev < availableProviders.length - 1 ? prev + 1 : 0
          );
          return true;
        }
        if (key.return) {
          void handleSwitchProvider(availableProviders[selectedProviderIndex]);
          return true;
        }
        return true;
      }

      // CONFIG-004 + PROV-007: Settings tab keyboard handling
      // TUI-074: Now handled by ProviderSettingsScreen component via useInput
      // AgentView should NOT intercept keys when settings tab is shown
      if (showSettingsTab) {
        // Let ProviderSettingsScreen handle all input
        return false;
      }

      // VIEWNV-001: Shift+Left/Right for unified session navigation
      // Uses sessionNavigation hook which determines correct target based on position in tree
      // Check escape sequences first, then Ink key detection
      {
        const isShiftLeft =
          input.includes('[1;2D') ||
          input.includes('\x1b[1;2D') ||
          (key.shift && key.leftArrow);
        const isShiftRight =
          input.includes('[1;2C') ||
          input.includes('\x1b[1;2C') ||
          (key.shift && key.rightArrow);

        if (isShiftLeft) {
          sessionNavigation.handleShiftLeft();
          return true;
        }
        if (isShiftRight) {
          sessionNavigation.handleShiftRight();
          return true;
        }
      }

      // TUI-045: Esc key handling with priority order:
      // 1) Close exit confirmation modal, 2) Close supervisor turn modal, 3) Close turn modal, 4) Disable select mode, 5) Interrupt loading, 6) Clear input, 7) Show exit confirmation or exit
      if (key.escape) {
        // Priority 1: Close exit confirmation modal (TUI-046)
        if (showExitConfirmation) {
          setShowExitConfirmation(false);
          return true;
        }
        // Priority 2: Close turn modal
        if (showTurnModal) {
          setShowTurnModal(false);
          return true;
        }
        // Priority 4: Disable select mode
        if (isTurnSelectMode) {
          setIsTurnSelectMode(false);
          return true;
        }
        // Priority 5: Interrupt loading or compaction - use background session interrupt
        // CMPCT-014: Also check rustSnapshot.isCompacting since isLoading is false during compaction
        if ((displayIsLoading || rustSnapshot.isCompacting) && currentSessionId) {
          try {
            sessionInterrupt(currentSessionId);
            refreshRustState(currentSessionId);
          } catch (err) {
            // Failed to interrupt session - indicates backend issues or connection problems
            logger.error('Failed to interrupt loading session:', err);
          }
          return true;
        }
        // Priority 6: Clear input
        if (inputValue.trim() !== '') {
          setInputValue('');
          return true;
        }
        // Priority 7: Show exit confirmation if session exists, otherwise exit (TUI-046)
        if (currentSessionId) {
          setShowExitConfirmation(true);
        } else {
          onExit();
        }
        return true;
      }

      // TUI-042: Tab to toggle turn selection mode (replaces /select command)
      if (key.tab) {
        const newMode = !isTurnSelectMode;
        setIsTurnSelectMode(newMode);
        // TUI-045: Close modal and clear modal state when disabling select mode
        if (!newMode) {
          setShowTurnModal(false);
          setModalMessageIndex(null);
        }
        // Note: VirtualList will auto-select last item via scrollToEnd when enabled
        return true;
      }
      // Note: TUI-042 turn navigation is handled by VirtualList via getNextIndex/getIsSelected

      return false;
    },
  });

  // PERF-002: Incremental line computation with caching
  // Only recompute lines for messages that changed, reuse cached lines for unchanged messages
  // PERF-003: Uses deferredConversation to prioritize user input over streaming updates
  // TUI-045: Removed expansion logic - modal now handles full content viewing
  const conversationLines = useMemo((): ConversationLine[] => {
    const maxWidth = calculatePaneWidth(terminalWidth, 'full');
    const lines: ConversationLine[] = [];
    const cache = lineCacheRef.current;

    // Track which message indices are still valid
    const validIndices = new Set<number>();

    deferredConversation.forEach((msg, msgIndex) => {
      validIndices.add(msgIndex);

      // TUI-045: Always use collapsed content in main view (modal shows full content)
      const effectiveContent = msg.content;

      // Create effective message for cache check
      const effectiveMsg = { ...msg, content: effectiveContent };

      // Check cache for this message
      const cached = cache.get(msgIndex);
      const isThinking = msg.type === 'thinking';
      if (
        cached &&
        cached.content === effectiveContent &&
        cached.isStreaming === msg.isStreaming &&
        cached.isThinking === isThinking &&
        cached.terminalWidth === terminalWidth
      ) {
        // Cache hit - reuse cached lines
        lines.push(...cached.lines);
      } else {
        // Cache miss - compute lines and cache them
        const messageLines = wrapMessageToLines(
          effectiveMsg,
          msgIndex,
          maxWidth
        );
        cache.set(msgIndex, {
          content: effectiveContent,
          isStreaming: msg.isStreaming ?? false,
          isThinking,
          terminalWidth,
          lines: messageLines,
        });
        lines.push(...messageLines);
      }
    });

    // Clean up stale cache entries (messages that were removed)
    for (const cachedIndex of cache.keys()) {
      if (!validIndices.has(cachedIndex)) {
        cache.delete(cachedIndex);
      }
    }

    return lines;
  }, [deferredConversation, terminalWidth]);

  // TUI-043: Keep ref in sync with conversationLines for use in callbacks
  conversationLinesRef.current = conversationLines;

  // Keep currentSessionIdRef in sync for use in callbacks (avoids stale closure)
  currentSessionIdRef.current = currentSessionId;

  // Error state - show setup instructions (full-screen overlay)
  // Only show this if error occurred before a session was created (no credentials)
  if (error && !currentSessionId) {
    return (
      <Box
        position="absolute"
        flexDirection="column"
        width={terminalWidth}
        height={terminalHeight}
      >
        <Box
          flexDirection="column"
          flexGrow={1}
          borderStyle="double"
          borderColor="red"
          backgroundColor="black"
        >
          <Box
            flexDirection="column"
            padding={2}
            flexGrow={1}
            justifyContent="center"
            alignItems="center"
          >
            <Box marginBottom={1}>
              <Text bold color="red">
                Error: AI Agent Unavailable
              </Text>
            </Box>
            <Box marginBottom={1}>
              <Text color="yellow">{error}</Text>
            </Box>
            <Box flexDirection="column" marginBottom={1}>
              <Text dimColor>No AI provider credentials configured.</Text>
              <Text dimColor>Set one of these environment variables:</Text>
              <Text color="cyan"> ANTHROPIC_API_KEY</Text>
              <Text color="cyan"> CODEX_API_KEY</Text>
              <Text color="cyan"> GOOGLE_GENERATIVE_AI_API_KEY</Text>
            </Box>
            <Box>
              <Text dimColor>Press Esc to close</Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  // Provider selector overlay (full-screen overlay)
  if (showProviderSelector) {
    // Calculate available width for provider text (terminal width minus border, padding)
    const providerTextWidth = terminalWidth - 2 - 4; // 2 for border, 4 for padding
    return (
      <Box
        position="absolute"
        flexDirection="column"
        width={terminalWidth}
        height={terminalHeight}
      >
        <Box
          flexDirection="column"
          flexGrow={1}
          borderStyle="double"
          borderColor="cyan"
          backgroundColor="black"
        >
          <Box
            flexDirection="column"
            padding={2}
            flexGrow={1}
            justifyContent="center"
            alignItems="center"
          >
            <Box marginBottom={1}>
              <Text bold color="cyan">
                Select Provider
              </Text>
            </Box>
            {availableProviders.map((provider, idx) => (
              <Box key={provider} width={providerTextWidth}>
                <Text
                  backgroundColor={
                    idx === selectedProviderIndex ? 'cyan' : undefined
                  }
                  color={idx === selectedProviderIndex ? 'black' : 'white'}
                  wrap="truncate"
                >
                  {idx === selectedProviderIndex ? '> ' : '  '}
                  {provider}
                  {provider === currentProvider ? ' (current)' : ''}
                </Text>
              </Box>
            ))}
            <Box marginTop={1}>
              <Text dimColor>Enter Select | Esc Cancel</Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  if (showModelSelector) {
    return (
      <ModelSelectorScreen
        width={terminalWidth}
        height={terminalHeight}
        currentModelId={currentModel?.apiModelId}
        onSelectModel={handleModelSelect}
        onClose={() => setShowModelSelector(false)}
        onSwitchToSettings={() => {
          setShowModelSelector(false);
          setShowSettingsTab(true);
        }}
      />
    );
  }

  // CONFIG-004 + PROV-007: Settings tab overlay with profile management
  // TUI-074: Now using ProviderSettingsScreen orchestrator component
  if (showSettingsTab) {
    return (
      <ProviderSettingsScreen
        width={terminalWidth}
        height={terminalHeight}
        onClose={() => {
          setShowSettingsTab(false);
          setAutoStartCopilotLogin(false);
        }}
        onSwitchToModels={() => {
          setShowSettingsTab(false);
          setAutoStartCopilotLogin(false);
          setShowModelSelector(true);
        }}
        autoStartCopilotLogin={autoStartCopilotLogin}
        onAutoStartCopilotLoginConsumed={() =>
          setAutoStartCopilotLogin(false)
        }
      />
    );
  }

  // NAPI-006: Search mode overlay (Ctrl+R history search)
  if (isSearchMode) {
    // Calculate available width for search text (terminal width minus padding, scrollbar)
    const searchTextWidth = terminalWidth - 4 - 3; // 4 for padding, 3 for scrollbar margin
    return (
      <Box
        position="absolute"
        flexDirection="column"
        width={terminalWidth}
        height={terminalHeight}
      >
        <Box flexDirection="column" flexGrow={1} backgroundColor="black">
          <Box flexDirection="column" padding={2} flexGrow={1}>
            <Box marginBottom={1}>
              <Text bold color="magenta">
                (search): {searchQuery}
                <Text inverse> </Text>
              </Text>
            </Box>
            {searchResults.length === 0 && searchQuery && (
              <Box>
                <Text dimColor>No matching history entries</Text>
              </Box>
            )}
            {searchResults.slice(0, 10).map((entry, idx) => (
              <Box
                key={`${entry.sessionId}-${entry.timestamp}`}
                width={searchTextWidth}
              >
                <Text
                  backgroundColor={
                    idx === searchResultIndex ? 'magenta' : undefined
                  }
                  color={idx === searchResultIndex ? 'black' : 'white'}
                  wrap="truncate"
                >
                  {idx === searchResultIndex ? '> ' : '  '}
                  {entry.display}
                </Text>
              </Box>
            ))}
            <Box marginTop={1}>
              <Text dimColor>Enter Select | ↑↓ Navigate | Esc Cancel</Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  // NAPI-003: Resume mode overlay (session selection)
  if (isResumeMode) {
    return (
      <Box
        position="absolute"
        flexDirection="column"
        width={terminalWidth}
        height={terminalHeight}
      >
        <Box flexDirection="column" flexGrow={1} backgroundColor="black">
          <Box flexDirection="column" padding={2} flexGrow={1}>
            <Box marginBottom={1}>
              <Text bold color="blue">
                Resume Session ({availableSessions.length} available)
              </Text>
              {availableSessions.length > resumeVisibleHeight && (
                <Text dimColor>
                  {' '}
                  (showing {resumeScrollOffset + 1}-
                  {Math.min(
                    resumeScrollOffset + resumeVisibleHeight,
                    availableSessions.length
                  )}
                  )
                </Text>
              )}
            </Box>
            {availableSessions.length === 0 && (
              <Box flexGrow={1}>
                <Text dimColor>No sessions found for this project</Text>
              </Box>
            )}
            {/* Scrollable session list */}
            <Box flexDirection="row" flexGrow={1}>
              <Box flexDirection="column" flexGrow={1}>
                {availableSessions
                  .slice(
                    resumeScrollOffset,
                    resumeScrollOffset + resumeVisibleHeight
                  )
                  .flatMap((session, visibleIdx) => {
                    const actualIdx = resumeScrollOffset + visibleIdx;
                    const isSelected = actualIdx === resumeSessionIndex;
                    const updatedAt = new Date(session.updatedAt);
                    const timeAgo = formatTimeAgo(updatedAt);
                    const provider = session.provider || 'unknown';
                    // Return two separate row items for each session (name line and detail line)
                    return [
                      <Box key={`${session.id}-name`}>
                        <Box flexGrow={1}>
                          <Text
                            backgroundColor={isSelected ? 'blue' : undefined}
                            color={isSelected ? 'black' : 'white'}
                            wrap="truncate"
                          >
                            {isSelected ? '> ' : '  '}
                            {getSessionStatusIcon(session)} {session.name}
                          </Text>
                        </Box>
                      </Box>,
                      <Box key={`${session.id}-detail`}>
                        <Box flexGrow={1}>
                          <Text
                            backgroundColor={isSelected ? 'blue' : undefined}
                            color={isSelected ? 'black' : 'gray'}
                            dimColor={!isSelected}
                            wrap="truncate"
                          >
                            {'    '}
                            {session.messageCount} messages | {provider} |{' '}
                            {timeAgo}
                          </Text>
                        </Box>
                      </Box>,
                    ];
                  })}
              </Box>
              {/* Scrollbar - each session is 2 lines, so scrollbar needs 2x height */}
              {availableSessions.length > resumeVisibleHeight && (
                <Box flexDirection="column" marginLeft={1}>
                  {Array.from({ length: resumeVisibleHeight * 2 }).map(
                    (_, i) => {
                      const scrollbarHeight = resumeVisibleHeight * 2;
                      const thumbHeight = Math.max(
                        2,
                        Math.floor(
                          (resumeVisibleHeight / availableSessions.length) *
                            scrollbarHeight
                        )
                      );
                      const thumbPos = Math.floor(
                        (resumeScrollOffset / availableSessions.length) *
                          scrollbarHeight
                      );
                      const isThumb =
                        i >= thumbPos && i < thumbPos + thumbHeight;
                      return (
                        <Text key={i} dimColor>
                          {isThumb ? '■' : '│'}
                        </Text>
                      );
                    }
                  )}
                </Box>
              )}
            </Box>
            <Box marginTop={1}>
              <Text dimColor>
                Enter Select | ↑↓ Navigate | D Delete | Esc Cancel
              </Text>
            </Box>
          </Box>
        </Box>
        {/* TUI-040: Delete session confirmation dialog */}
        {showSessionDeleteDialog && (
          <ThreeButtonDialog
            message={`Delete session "${availableSessions[resumeSessionIndex]?.name || 'Unknown'}"?`}
            options={['Delete This Session', 'Delete ALL Sessions', 'Cancel']}
            onSelect={handleSessionDeleteSelect}
            onCancel={handleSessionDeleteCancel}
          />
        )}
        {/* TUI-046: Exit confirmation dialog (shown in resume mode too) */}
        {showExitConfirmation && (
          <ThreeButtonDialog
            message="Exit Session?"
            description={
              displayIsLoading
                ? 'The agent is currently running. Choose how to exit.'
                : 'Choose how to exit the session.'
            }
            options={['Detach', 'Close Session', 'Cancel']}
            defaultSelectedIndex={0}
            onSelect={handleExitChoice}
            onCancel={() => setShowExitConfirmation(false)}
          />
        )}
      </Box>
    );
  }

  // BLOCK-004: Blocklist management overlay
  if (isBlocklistMode) {
    return (
      <BlocklistListView
        rules={blocklistRules}
        disabledRules={disabledBlocklistRules}
        terminalWidth={terminalWidth}
        terminalHeight={terminalHeight}
        onToggleRule={handleToggleBlocklistRule}
        onClose={() => {
          setIsBlocklistMode(false);
        }}
      />
    );
  }

  // Main agent view (full-screen)
  // Remove position="absolute" since FullScreenWrapper handles positioning
  // Removed outer border to maximize usable space and reduce rendering overhead
  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* TUI-034: Shared session header with model info, capabilities, and token usage */}
      <SessionHeader
        modelId={displayModelId}
        hasReasoning={displayReasoning}
        hasVision={displayHasVision}
        contextWindow={displayContextWindow}
        compactionThreshold={displayCompactionThreshold}
        isDebugEnabled={displayIsDebugEnabled}
        isSelectMode={isTurnSelectMode}
        thinkingLevel={detectedThinkingLevel}
        baseThinkingLevel={rustSnapshot.baseThinkingLevel as JsThinkingLevel}
        isLoading={displayIsLoading}
        tokensPerSecond={displayedTokPerSec}
        tokenUsage={tokenUsage}
        rustTokens={rustTokens}
        contextFillPercentage={contextFillPercentage}
        compactionReduction={compactionReduction}
        sessionNumber={sessionNumber}
        isIsolated={isIsolated}
      />

      {/* TUI-081: Display active role below SessionHeader */}
      <RoleBanner roleText={(() => {
        if (!currentSessionId) { return null; }
        try {
          const role = sessionGetRole(currentSessionId);
          return role?.name ?? null;
        } catch {
          return null;
        }
      })()}
      />

      {/* Conversation area using VirtualList for proper scrolling - matches FileDiffViewer pattern */}
      <Box flexGrow={1} flexBasis={0}>
        <VirtualList
          items={conversationLines}
          renderItem={(line, index, isSelected, selectedIndex) => {
            // TUI-038: Check for diff color markers and render with background colors
            const content = line.content;

            // Parse diff color markers: [R] for removed (red), [A] for added (green)
            // Changed lines: line numbers WHITE, +/- content colored
            // Context lines (no marker): gray
            // Diff line pattern: starts with "L " or spaces, followed by digits and spaces
            const isDiffContextLine = (text: string): boolean => {
              // Match: "L  123   content" or "   123   content" (tree connector + line number + spaces + content)
              return /^[L ]?\s*\d+\s{3}/.test(text);
            };

            // TUI-042: Render separator lines with selection indicator using shared utility
            const separatorType = getSelectionSeparatorType(
              line,
              index,
              conversationLines,
              selectedIndex,
              isTurnSelectMode
            );
            if (separatorType) {
              const lineWidth = terminalWidth - 4;
              const arrowBar = generateArrowBar(lineWidth, separatorType);
              return (
                <Box flexGrow={1}>
                  <Text backgroundColor="gray" color="white">
                    {arrowBar}
                  </Text>
                </Box>
              );
            }

            if (line.role === 'tool') {
              const rIdx = content.indexOf('[R]');
              const aIdx = content.indexOf('[A]');

              // Changed line with [R] or [A] marker - entire line gets colored background
              if (rIdx >= 0 || aIdx >= 0) {
                const markerIdx = rIdx >= 0 ? rIdx : aIdx;
                const markerType = rIdx >= 0 ? 'R' : 'A';
                // Remove the [R] or [A] marker, keep everything else
                const lineWithoutMarker =
                  content.slice(0, markerIdx) + content.slice(markerIdx + 3);

                return (
                  <Box flexGrow={1}>
                    <Text
                      backgroundColor={
                        markerType === 'R'
                          ? DIFF_COLORS.removed
                          : DIFF_COLORS.added
                      }
                      color="white"
                    >
                      {lineWithoutMarker}
                    </Text>
                  </Box>
                );
              }

              // Context line (diff line without marker) - line number gray, content white
              if (isDiffContextLine(content)) {
                // Split at the 3 spaces after line number to separate line num from content
                const match = content.match(/^([L ]?\s*\d+\s{3})(.*)$/);
                if (match) {
                  const [, lineNumPart, contentPart] = match;
                  return (
                    <Box flexGrow={1}>
                      <Text color="gray">{lineNumPart}</Text>
                      <Text>{contentPart}</Text>
                    </Box>
                  );
                }
                return (
                  <Box flexGrow={1}>
                    <Text color="gray">{content}</Text>
                  </Box>
                );
              }

              // Stderr marker constant - used for error and stderr rendering
              const STDERR_MARKER = '⚠stderr⚠';

              // Error output (isError=true from tool result) - render in red
              // Also strip stderr marker since errors from bash tool include marked stderr
              if (line.isError) {
                const cleanContent = content.replace(
                  new RegExp(STDERR_MARKER, 'g'),
                  ''
                );
                return (
                  <Box flexGrow={1}>
                    <Text color="red">{cleanContent}</Text>
                  </Box>
                );
              }

              // Stderr output (marked with ⚠stderr⚠ prefix during streaming) - render in red
              if (content.includes(STDERR_MARKER)) {
                // Remove the marker and render in red
                const cleanContent = content.replace(
                  new RegExp(STDERR_MARKER, 'g'),
                  ''
                );
                return (
                  <Box flexGrow={1}>
                    <Text color="red">{cleanContent}</Text>
                  </Box>
                );
              }
            }

            // Thinking content - render in yellow (using isThinking flag)
            // SOLID: This check is OUTSIDE the tool role block so it applies to assistant messages too
            if (line.isThinking) {
              return (
                <Box flexGrow={1}>
                  <Text color="yellow">{content}</Text>
                </Box>
              );
            }

            // Default rendering for non-diff content
            // Tool output is white (not yellow), user input is green, supervisor input is magenta (WATCH-012)
            const baseColor =
              line.role === 'user'
                ? 'green'
                : line.role === 'supervisor'
                  ? 'magenta'
                  : 'white';
            return (
              <Box flexGrow={1}>
                <Text color={baseColor}>{content}</Text>
              </Box>
            );
          }}
          keyExtractor={(_line, index) => `line-${index}`}
          emptyMessage=""
          showScrollbar={true}
          isFocused={
            !showProviderSelector &&
            !showModelSelector &&
            !showSettingsTab &&
            !isResumeMode &&
            !isSearchMode &&
            !showTurnModal
          }
          scrollToEnd={true}
          selectionMode={isTurnSelectMode ? 'item' : 'scroll'}
          // TUI-042/044: Group-based selection for turn navigation
          // Groups lines by messageIndex, navigates between groups
          groupBy={isTurnSelectMode ? line => line.messageIndex : undefined}
          groupPaddingBefore={isTurnSelectMode ? 1 : 0}
          // TUI-045: onSelect opens modal when Enter is pressed in select mode
          onSelect={
            isTurnSelectMode
              ? line => {
                  setModalMessageIndex(line.messageIndex);
                  setShowTurnModal(true);
                }
              : undefined
          }
          // TUI-043: Expose selection state to parent
          selectionRef={virtualListSelectionRef}
        />
      </Box>

      {/* TUI-091: Session footer with CWD and git branch info */}
      <SessionFooter sessionId={currentSessionId} />

      {/* Input area */}
      <Box
        paddingX={1}
      >
        <Text color="green">&gt; </Text>
        <Box flexGrow={1}>
          <InputTransition
            isLoading={displayIsLoading}
            isPaused={displayIsPaused}
            pauseInfo={displayPauseInfo}
            triplePauseSelection={triplePauseSelection}
            hitlRequest={displayHitlRequest}
            hitlQuestionIndex={hitlInput.state.questionIndex}
            hitlSelectedOption={hitlInput.state.selectedOption}
            hitlFreeformActive={hitlInput.isCurrentQuestionFreeform}
            hitlOtherActive={hitlInput.isOtherActive}
            hitlShowEmptyHint={hitlInput.showEmptyHint}
            isCompacting={compaction.state.isActive}
            compactionProgress={compaction.state.progress}
            actionPrompt={actionPrompt}
            clearActionPrompt={() => setActionPrompt(null)}
            value={inputValue}
            onChange={handleInputChange}
            onSubmit={handleSubmit}
            placeholder="Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)"
            onHistoryPrev={handleHistoryPrev}
            onHistoryNext={handleHistoryNext}
            maxVisibleLines={5}
            skipAnimation={skipInputAnimation}
            isActive={!showCreateSessionDialog}
            suppressEnter={
              slashCommand.isVisible ||
              fileSearch.isVisible ||
              isTurnSelectMode ||
              compaction.state.isActive
            }
          />
        </Box>
      </Box>

      {/* TUI-050: Slash command autocomplete palette */}
      {slashCommand.isVisible && (
        <SlashCommandPalette
          isVisible={slashCommand.isVisible}
          filter={slashCommand.filter}
          commands={slashCommand.filteredCommands}
          selectedIndex={slashCommand.selectedIndex}
          dialogWidth={slashCommand.dialogWidth}
          maxVisibleItems={8}
        />
      )}

      {/* TUI-055: File search popup (exact same architecture as slash commands) */}
      {fileSearch.isVisible && (
        <FileSearchPopup
          isVisible={fileSearch.isVisible}
          filter={fileSearch.filter}
          files={fileSearch.files}
          selectedIndex={fileSearch.selectedIndex}
          dialogWidth={fileSearch.dialogWidth}
        />
      )}

      {/* TUI-045: Full turn content modal */}
      {showTurnModal &&
        modalMessageIndex !== null &&
        conversation[modalMessageIndex] && (
          <TurnContentModal
            content={
              conversation[modalMessageIndex].fullContent ||
              conversation[modalMessageIndex].content
            }
            role={conversation[modalMessageIndex].role}
            terminalWidth={terminalWidth}
            terminalHeight={terminalHeight}
            isFocused={showTurnModal}
          />
        )}

      {/* TUI-046: Exit confirmation dialog */}
      {showExitConfirmation && (
        <ThreeButtonDialog
          message="Exit Session?"
          description={
            displayIsLoading
              ? 'The agent is currently running. Choose how to exit.'
              : 'Choose how to exit the session.'
          }
          options={['Detach', 'Close Session', 'Cancel']}
          defaultSelectedIndex={0}
          onSelect={handleExitChoice}
          onCancel={() => setShowExitConfirmation(false)}
        />
      )}

      {/* PERF-002: Compaction retry dialog */}
      {compaction.retryState.isVisible && (
        <ThreeButtonDialog
          message={`Compaction Failed: ${compaction.retryState.error}`}
          description="Choose how to proceed after compaction failure:"
          options={['Retry', 'Continue without compacting', 'Cancel']}
          defaultSelectedIndex={0}
          onSelect={(index, _option) => {
            const optionKey = ['retry', 'continue', 'cancel'][index] as
              | 'retry'
              | 'continue'
              | 'cancel';
            compaction.handleRetryOption(optionKey);
            if (optionKey === 'retry' && currentSessionId) {
              // Retry compaction
              compaction
                .performManualCompaction(currentSessionId)
                .then(result => {
                  // UX-002: Don't add success message to conversation on retry
                  setCompactionReduction(result.compressionRatio);
                  setTokenUsage(prev => ({
                    ...prev,
                    inputTokens: result.compactedTokens,
                  }));
                })
                .catch(err => {
                  if (!compaction.retryState.isVisible) {
                    const errorMessage =
                      err instanceof Error ? err.message : 'Failed to compact';
                    setConversation(prev => [
                      ...prev,
                      {
                        type: 'status',
                        content: `Compaction failed: ${errorMessage}`,
                      },
                    ]);
                  }
                });
            }
          }}
          onCancel={() => compaction.clearRetryState()}
        />
      )}

      {/* VIEWNV-001: Create session dialog (shown when navigating past right edge) */}
      {showCreateSessionDialog && (
        <CreateSessionDialog
          onConfirm={handleCreateSessionConfirm}
          onCancel={closeCreateSessionDialog}
        />
      )}

      {/* AMGR-012: Role dialog */}
      {showRoleDialog && currentSessionId && (
        <RoleDialog
          initialRole={(() => {
            try {
              const role = sessionGetRole(currentSessionId);
              return role?.name ?? '';
            } catch {
              return '';
            }
          })()}
          onSubmit={(role: string) => {
            try {
              if (role.trim()) {
                sessionSetRole(currentSessionId, role.trim(), null, null);
              } else {
                sessionSetRole(currentSessionId, '', null, null);
              }
            } catch (err) {
              const errorMessage = err instanceof Error ? err.message : 'Failed to set role';
              setConversation(prev => [
                ...prev,
                { type: 'status', content: `Role error: ${errorMessage}` },
              ]);
            }
            setShowRoleDialog(false);
          }}
          onClose={() => setShowRoleDialog(false)}
        />
      )}

      {/* TUI-054: Thinking level dialog */}
      {showThinkingLevelDialog && currentSessionId && (
        <ThinkingLevelDialog
          currentLevel={rustSnapshot.baseThinkingLevel as JsThinkingLevel}
          defaultLevel={defaultThinkingLevel}
          onSelect={level => {
            // Update base thinking level in Rust
            getRustStateSource().setBaseThinkingLevel(currentSessionId, level);
            // Refresh snapshot to pick up the change
            refreshRustState();
            setShowThinkingLevelDialog(false);
          }}
          onSetDefault={async level => {
            // TUI-075: Hook handles persist + apply to current session
            await setDefaultThinkingLevel(level);
          }}
          onClose={() => setShowThinkingLevelDialog(false)}
        />
      )}

      {/* Error dialog for API/model errors */}
      {error && <ErrorDialog message={error} onClose={() => setError(null)} />}
    </Box>
  );
};

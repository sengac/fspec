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
 * Implements NAPI-006: Session Persistence with Fork and Merge
 * - Shift+Arrow-Up/Down for command history navigation
 * - /search command for history search
 * - Session commands: /resume, /fork, /merge, /switch, /rename, /cherry-pick, /sessions, /search
 */

import React, {
  useState,
  useEffect,
  useCallback,
  useRef,
  useMemo,
  useDeferredValue,
} from 'react';
import { Box, Text, useInput, useStdout } from 'ink';
import { VirtualList } from './VirtualList';
import { getFspecUserDir, loadConfig, writeConfig } from '../../utils/config';
import { logger } from '../../utils/logger';
import { normalizeEmojiWidth, getVisualWidth } from '../utils/stringWidth';
import {
  persistenceStoreMessageEnvelope,
  getThinkingConfig,
  JsThinkingLevel,
  modelsListAll,
  modelsRefreshCache,
  type NapiProviderModels,
  type NapiModelInfo,
} from '@sengac/codelet-napi';
import {
  detectThinkingLevel,
  getThinkingLevelLabel,
} from '../../utils/thinkingLevel';
import {
  saveCredential,
  deleteCredential,
  getProviderConfig,
  maskApiKey,
} from '../../utils/credentials';
import {
  SUPPORTED_PROVIDERS,
  getProviderRegistryEntry,
  type ProviderRegistryEntry,
} from '../../utils/provider-config';
import {
  computeLineDiff,
  changesToDiffLines,
  type DiffLine,
} from '../../git/diff-parser';

// TUI-034: Model selection types
interface ModelSelection {
  providerId: string; // "anthropic"
  modelId: string; // "claude-sonnet-4"
  apiModelId: string; // "claude-sonnet-4-20250514" (for API calls)
  displayName: string; // "Claude Sonnet 4"
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number; // 200000
  maxOutput: number; // 16000
}

interface ProviderSection {
  providerId: string; // "anthropic"
  providerName: string; // "Anthropic"
  internalName: string; // "claude" (for provider manager)
  models: NapiModelInfo[]; // Filtered to tool_call=true
  hasCredentials: boolean; // From availableProviders check
}

// Flattened item type for VirtualList-based model selector scrolling
type ModelSelectorItem =
  | {
      type: 'section';
      sectionIdx: number;
      section: ProviderSection;
      isExpanded: boolean;
    }
  | {
      type: 'model';
      sectionIdx: number;
      modelIdx: number;
      section: ProviderSection;
      model: NapiModelInfo;
    };

// Build flattened list from sections and expanded state
const buildFlatModelList = (
  sections: ProviderSection[],
  expandedProviders: Set<string>
): ModelSelectorItem[] => {
  const items: ModelSelectorItem[] = [];
  sections.forEach((section, sectionIdx) => {
    const isExpanded = expandedProviders.has(section.providerId);
    items.push({ type: 'section', sectionIdx, section, isExpanded });
    if (isExpanded) {
      section.models.forEach((model, modelIdx) => {
        items.push({ type: 'model', sectionIdx, modelIdx, section, model });
      });
    }
  });
  return items;
};

// Convert flat index to (sectionIdx, modelIdx) - modelIdx is -1 for section headers
const flatIndexToSectionModel = (
  flatIndex: number,
  items: ModelSelectorItem[]
): { sectionIdx: number; modelIdx: number } => {
  const item = items[flatIndex];
  if (!item) return { sectionIdx: 0, modelIdx: -1 };
  if (item.type === 'section') {
    return { sectionIdx: item.sectionIdx, modelIdx: -1 };
  }
  return { sectionIdx: item.sectionIdx, modelIdx: item.modelIdx };
};

// Convert (sectionIdx, modelIdx) to flat index
const sectionModelToFlatIndex = (
  sectionIdx: number,
  modelIdx: number,
  items: ModelSelectorItem[]
): number => {
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (
      item.type === 'section' &&
      item.sectionIdx === sectionIdx &&
      modelIdx === -1
    ) {
      return i;
    }
    if (
      item.type === 'model' &&
      item.sectionIdx === sectionIdx &&
      item.modelIdx === modelIdx
    ) {
      return i;
    }
  }
  return 0;
};

// TUI-034: Provider ID mapping (models.dev to internal)
const mapProviderIdToInternal = (providerId: string): string => {
  switch (providerId) {
    case 'anthropic':
      return 'claude';
    case 'google':
      return 'gemini';
    default:
      return providerId;
  }
};

const mapInternalToProviderId = (internalName: string): string => {
  switch (internalName) {
    case 'claude':
      return 'anthropic';
    case 'gemini':
      return 'google';
    default:
      return internalName;
  }
};

// CONFIG-004: Map models.dev provider IDs to our registry/credentials provider IDs
// models.dev uses "google" but our registry/credentials uses "gemini"
const mapModelsDevToRegistryId = (modelsDevProviderId: string): string => {
  switch (modelsDevProviderId) {
    case 'google':
      return 'gemini';
    default:
      return modelsDevProviderId;
  }
};

// TUI-034: Format context window size for display
const formatContextWindow = (contextWindow: number): string => {
  if (contextWindow >= 1000000) {
    return `${(contextWindow / 1000000).toFixed(0)}M`;
  }
  return `${Math.round(contextWindow / 1000)}k`;
};

// NAPI-006: Callbacks for history navigation
interface SafeTextInputCallbacks {
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
}

// Custom TextInput that ignores mouse escape sequences
const SafeTextInput: React.FC<
  {
    value: string;
    onChange: (value: string) => void;
    onSubmit: () => void;
    placeholder?: string;
    isActive?: boolean;
  } & SafeTextInputCallbacks
> = ({
  value,
  onChange,
  onSubmit,
  placeholder = '',
  isActive = true,
  onHistoryPrev,
  onHistoryNext,
}) => {
  // Use ref to avoid stale closure issues with rapid typing
  const valueRef = useRef(value);
  valueRef.current = value;

  useInput(
    (input, key) => {
      // Ignore mouse escape sequences
      if (key.mouse || input.includes('[M') || input.includes('[<')) {
        return;
      }

      if (key.return) {
        onSubmit();
        return;
      }

      if (key.backspace || key.delete) {
        const newValue = valueRef.current.slice(0, -1);
        valueRef.current = newValue; // Update immediately to handle rapid keystrokes
        onChange(newValue);
        return;
      }

      // NAPI-006: Shift+Arrow for history navigation (check before ignoring arrow keys)

      // Check raw escape sequences first (most reliable for Shift+Arrow)
      if (input.includes('[1;2A') || input.includes('\x1b[1;2A')) {
        onHistoryPrev?.();
        return;
      }
      if (input.includes('[1;2B') || input.includes('\x1b[1;2B')) {
        onHistoryNext?.();
        return;
      }
      // ink may set key.shift when shift is held
      if (key.shift && key.upArrow) {
        onHistoryPrev?.();
        return;
      }
      if (key.shift && key.downArrow) {
        onHistoryNext?.();
        return;
      }

      // Ignore navigation keys (handled by other components)
      if (
        key.escape ||
        key.tab ||
        key.upArrow ||
        key.downArrow ||
        key.pageUp ||
        key.pageDown
      ) {
        return;
      }

      // Filter to only printable characters, removing any escape sequence remnants
      const clean = input
        .split('')
        .filter(ch => {
          const code = ch.charCodeAt(0);
          // Only allow printable ASCII (space through tilde)
          return code >= 32 && code <= 126;
        })
        .join('');

      if (clean) {
        const newValue = valueRef.current + clean;
        valueRef.current = newValue; // Update immediately to handle rapid keystrokes
        onChange(newValue);
      }
    },
    { isActive }
  );

  return (
    <Text>
      {value || <Text dimColor>{placeholder}</Text>}
      <Text inverse> </Text>
    </Text>
  );
};

// Types from codelet-napi
interface TokenTracker {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens?: number;
  cacheCreationInputTokens?: number;
}

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
}

interface Message {
  role: string;
  content: string;
}

// AGENT-021: Debug command result from toggleDebug()
interface DebugCommandResult {
  enabled: boolean;
  sessionFile: string | null;
  message: string;
}

// NAPI-005: Compaction result from compact()
interface CompactionResult {
  originalTokens: number;
  compactedTokens: number;
  compressionRatio: number;
  turnsSummarized: number;
  turnsKept: number;
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

interface CodeletSessionType {
  currentProviderName: string;
  availableProviders: string[];
  tokenTracker: TokenTracker;
  messages: Message[];
  prompt: (
    input: string,
    thinkingConfig: string | null, // TOOL-010: Thinking config JSON
    callback: (chunk: StreamChunk) => void
  ) => Promise<void>;
  switchProvider: (providerName: string) => Promise<void>;
  clearHistory: () => void;
  interrupt: () => void;
  resetInterrupt: () => void;
  toggleDebug: (debugDir?: string) => DebugCommandResult; // AGENT-021
  compact: () => Promise<CompactionResult>; // NAPI-005
  getContextFillInfo: () => { fillPercentage: number };
}

export interface AgentViewProps {
  onExit: () => void;
}

// Conversation message type for display
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  isStreaming?: boolean;
}

// Line type for VirtualList (flattened from messages)
interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  messageIndex: number;
}

/**
 * Extract model ID from API model ID for registry matching.
 * IMPORTANT: Do NOT use model.family - it may be a generic family name (e.g., "gemini-pro")
 * that doesn't match registry keys. Instead, extract from the full API ID by stripping suffixes.
 *
 * Examples:
 *   "claude-sonnet-4-20250514" -> "claude-sonnet-4" (strip date suffix)
 *   "gemini-2.5-pro-preview-06-05" -> "gemini-2.5-pro" (strip preview suffix)
 *   "gpt-4o" -> "gpt-4o" (no change)
 */
const extractModelIdForRegistry = (apiModelId: string): string => {
  return apiModelId
    .replace(/-preview-\d{2}-\d{2}$/, '') // Remove Gemini preview suffix
    .replace(/-\d{8}$/, ''); // Remove date suffix
};

// TUI-037: Claude Code style tool display helpers
const STREAMING_WINDOW_SIZE = 10; // Number of lines visible during streaming
const COLLAPSED_LINES = 4; // Number of lines visible when collapsed for normal output
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
  const collapsedContent = `${visible.join('\n')}\n... +${remaining} lines (ctrl+o to expand)`;
  return formatWithTreeConnectors(collapsedContent);
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
  return lines.slice(-windowSize).join('\n');
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
}

const formatEditDiff = (
  oldString: string,
  newString: string
): DiffOutputLine[] => {
  const changes = computeLineDiff(oldString, newString);
  const diffLines = changesToDiffLines(changes);
  return diffLines.map(line => ({
    content: line.content,
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
    color: DIFF_COLORS.added,
  }));
};

/**
 * TUI-038: Convert diff output lines to display format with tree connectors
 * Matches Claude Code format exactly:
 * - Line numbers are ALWAYS dim/gray (outside color marker)
 * - Only +/- and content have colored background
 * - Format: "2513 [R]- content" for removed, "2513 [A]+ content" for added
 * - Context lines: "2535   content" (all dim)
 * - Shows ~25 lines before collapsing
 */
const formatDiffForDisplay = (
  diffLines: DiffOutputLine[],
  visibleLines: number = DIFF_COLLAPSED_LINES
): string => {
  // Calculate line number width for padding (based on total lines)
  const maxLineNum = diffLines.length;
  const lineNumWidth = Math.max(String(maxLineNum).length, 3); // Min 3 chars for alignment

  // Format: line number is OUTSIDE color marker (always dim), only +/- and content inside
  // Color markers: [R] for removed (red), [A] for added (green)
  const formattedLines = diffLines.map((line, idx) => {
    const lineNum = String(idx + 1).padStart(lineNumWidth, ' ');
    // First char is the diff prefix: + (added), - (removed), or space (context)
    const restOfLine = line.content.slice(1);

    if (line.color === DIFF_COLORS.removed) {
      // Format: "2513 [R]- content" - linenum outside (dim), minus+content inside (colored)
      return `${lineNum} [R]- ${restOfLine}`;
    } else if (line.color === DIFF_COLORS.added) {
      // Format: "2513 [A]+ content" - linenum outside (dim), plus+content inside (colored)
      return `${lineNum} [A]+ ${restOfLine}`;
    }
    // Context lines - all dim, 3 spaces to align with " X " pattern
    return `${lineNum}   ${restOfLine}`;
  });

  // Apply collapse logic
  if (formattedLines.length <= visibleLines) {
    return formatWithTreeConnectors(formattedLines.join('\n'));
  }

  const visible = formattedLines.slice(0, visibleLines);
  const remaining = formattedLines.length - visibleLines;
  const collapsedContent = `${visible.join('\n')}\n... +${remaining} lines (ctrl+o to expand)`;
  return formatWithTreeConnectors(collapsedContent);
};

export const AgentView: React.FC<AgentViewProps> = ({ onExit }) => {
  const { stdout } = useStdout();
  const [session, setSession] = useState<CodeletSessionType | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [inputValue, setInputValue] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const [conversation, setConversation] = useState<ConversationMessage[]>([]);
  const [tokenUsage, setTokenUsage] = useState<TokenTracker>({
    inputTokens: 0,
    outputTokens: 0,
  });
  const [currentProvider, setCurrentProvider] = useState<string>('');
  const [availableProviders, setAvailableProviders] = useState<string[]>([]);
  const [showProviderSelector, setShowProviderSelector] = useState(false);
  const [selectedProviderIndex, setSelectedProviderIndex] = useState(0);
  const [isDebugEnabled, setIsDebugEnabled] = useState(false); // AGENT-021
  const sessionRef = useRef<CodeletSessionType | null>(null);

  // TUI-038: Store pending Edit/Write tool inputs for diff display
  interface PendingToolDiff {
    toolName: string;
    toolCallId: string;
    oldString?: string; // For Edit tool
    newString?: string; // For Edit tool
    content?: string; // For Write tool
  }
  const pendingToolDiffsRef = useRef<Map<string, PendingToolDiff>>(new Map());

  // TUI-034: Model selection state
  const [currentModel, setCurrentModel] = useState<ModelSelection | null>(null);
  const [providerSections, setProviderSections] = useState<ProviderSection[]>(
    []
  );
  const [showModelSelector, setShowModelSelector] = useState(false);
  const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
  const [selectedModelIdx, setSelectedModelIdx] = useState(-1); // -1 = on section header
  const [expandedProviders, setExpandedProviders] = useState<Set<string>>(
    new Set()
  );
  const [modelSelectorScrollOffset, setModelSelectorScrollOffset] = useState(0);
  const [modelSelectorFilter, setModelSelectorFilter] = useState('');
  const [isModelSelectorFilterMode, setIsModelSelectorFilterMode] =
    useState(false);

  // CONFIG-004: Settings tab state
  const [showSettingsTab, setShowSettingsTab] = useState(false);
  const [selectedSettingsIdx, setSelectedSettingsIdx] = useState(0);
  const [settingsScrollOffset, setSettingsScrollOffset] = useState(0);
  const [settingsFilter, setSettingsFilter] = useState('');
  const [isSettingsFilterMode, setIsSettingsFilterMode] = useState(false);
  const [editingProviderId, setEditingProviderId] = useState<string | null>(
    null
  );
  const [editingApiKey, setEditingApiKey] = useState('');
  const [providerStatuses, setProviderStatuses] = useState<
    Record<string, { hasKey: boolean; maskedKey?: string }>
  >({});
  const [connectionTestResult, setConnectionTestResult] = useState<{
    providerId: string;
    success: boolean;
    message: string;
  } | null>(null);
  const [isRefreshingModels, setIsRefreshingModels] = useState(false);

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
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  // Track if first message has been sent (for auto-renaming session)
  const isFirstMessageRef = useRef(true);
  const currentProjectRef = useRef<string>(process.cwd());

  // NAPI-003: Resume mode state (session selection overlay)
  const [isResumeMode, setIsResumeMode] = useState(false);
  const [availableSessions, setAvailableSessions] = useState<SessionManifest[]>(
    []
  );
  const [resumeSessionIndex, setResumeSessionIndex] = useState(0);

  // TUI-031: Tok/s display (calculated in Rust, just displayed here)
  const [displayedTokPerSec, setDisplayedTokPerSec] = useState<number | null>(
    null
  );
  const [lastChunkTime, setLastChunkTime] = useState<number | null>(null);

  // TUI-033: Context window fill percentage (received from Rust via ContextFillUpdate event)
  const [contextFillPercentage, setContextFillPercentage] = useState<number>(0);

  // TOOL-010: Detected thinking level (for UI indicator)
  const [detectedThinkingLevel, setDetectedThinkingLevel] = useState<
    number | null
  >(null);

  // PERF-002: Incremental line computation cache
  // Cache wrapped lines per message to avoid recomputing entire conversation
  // This is the main optimization - line wrapping is expensive (getVisualWidth for each char)
  interface CachedMessageLines {
    content: string;
    isStreaming: boolean;
    terminalWidth: number;
    lines: ConversationLine[];
  }
  const lineCacheRef = useRef<Map<number, CachedMessageLines>>(new Map());

  // PERF-003: Use deferred value for conversation to prioritize user input
  // This tells React that conversation updates are lower priority than user interactions
  const deferredConversation = useDeferredValue(conversation);

  // TUI-033: Get color based on fill percentage
  const getContextFillColor = (percentage: number): string => {
    if (percentage < 50) return 'green';
    if (percentage < 70) return 'yellow';
    if (percentage < 85) return 'magenta';
    return 'red';
  };

  // Get terminal dimensions for full-screen layout
  const terminalWidth = stdout?.columns ?? 80;
  const terminalHeight = stdout?.rows ?? 24;

  // Model selector scrolling: build flat list and manage scroll offset
  const flatModelItems = useMemo(
    () => buildFlatModelList(providerSections, expandedProviders),
    [providerSections, expandedProviders]
  );

  // Filter model items by search string (matches provider name or model name/id)
  const filteredFlatModelItems = useMemo(() => {
    if (!modelSelectorFilter) return flatModelItems;
    const filterLower = modelSelectorFilter.toLowerCase();
    return flatModelItems.filter(item => {
      if (item.type === 'section') {
        return (
          item.section.providerId.toLowerCase().includes(filterLower) ||
          item.section.providerName.toLowerCase().includes(filterLower)
        );
      } else {
        return (
          item.model.id.toLowerCase().includes(filterLower) ||
          item.model.name.toLowerCase().includes(filterLower) ||
          item.section.providerId.toLowerCase().includes(filterLower)
        );
      }
    });
  }, [flatModelItems, modelSelectorFilter]);

  const modelSelectorVisibleHeight = Math.max(
    1,
    terminalHeight - (isModelSelectorFilterMode ? 11 : 10)
  ); // Extra line for filter input
  const selectedFlatIdx = useMemo(
    () =>
      sectionModelToFlatIndex(
        selectedSectionIdx,
        selectedModelIdx,
        filteredFlatModelItems
      ),
    [selectedSectionIdx, selectedModelIdx, filteredFlatModelItems]
  );

  // Keep selected item visible by adjusting scroll offset
  useEffect(() => {
    if (!showModelSelector) return;
    if (selectedFlatIdx < modelSelectorScrollOffset) {
      setModelSelectorScrollOffset(selectedFlatIdx);
    } else if (
      selectedFlatIdx >=
      modelSelectorScrollOffset + modelSelectorVisibleHeight
    ) {
      setModelSelectorScrollOffset(
        selectedFlatIdx - modelSelectorVisibleHeight + 1
      );
    }
  }, [
    selectedFlatIdx,
    modelSelectorScrollOffset,
    modelSelectorVisibleHeight,
    showModelSelector,
  ]);

  // Reset scroll/filter when model selector opens
  useEffect(() => {
    if (showModelSelector) {
      setModelSelectorScrollOffset(0);
      setModelSelectorFilter('');
      setIsModelSelectorFilterMode(false);
    }
  }, [showModelSelector]);

  // Reset selection when filter changes
  useEffect(() => {
    if (filteredFlatModelItems.length > 0) {
      const firstItem = filteredFlatModelItems[0];
      if (firstItem.type === 'section') {
        setSelectedSectionIdx(firstItem.sectionIdx);
        setSelectedModelIdx(-1);
      } else {
        setSelectedSectionIdx(firstItem.sectionIdx);
        setSelectedModelIdx(firstItem.modelIdx);
      }
      setModelSelectorScrollOffset(0);
    }
  }, [modelSelectorFilter]);

  // Settings tab scrolling
  const settingsVisibleHeight = Math.max(
    1,
    terminalHeight - (isSettingsFilterMode ? 11 : 10)
  ); // Extra line for filter input

  // Filter settings providers by search string
  const filteredSettingsProviders = useMemo(() => {
    if (!settingsFilter) return SUPPORTED_PROVIDERS;
    const filterLower = settingsFilter.toLowerCase();
    return SUPPORTED_PROVIDERS.filter(providerId => {
      const registryEntry = getProviderRegistryEntry(providerId);
      return (
        providerId.toLowerCase().includes(filterLower) ||
        (registryEntry?.name?.toLowerCase().includes(filterLower) ?? false)
      );
    });
  }, [settingsFilter]);

  // Keep selected settings item visible by adjusting scroll offset
  useEffect(() => {
    if (!showSettingsTab) return;
    if (selectedSettingsIdx < settingsScrollOffset) {
      setSettingsScrollOffset(selectedSettingsIdx);
    } else if (
      selectedSettingsIdx >=
      settingsScrollOffset + settingsVisibleHeight
    ) {
      setSettingsScrollOffset(selectedSettingsIdx - settingsVisibleHeight + 1);
    }
  }, [
    selectedSettingsIdx,
    settingsScrollOffset,
    settingsVisibleHeight,
    showSettingsTab,
  ]);

  // Reset scroll/filter when settings tab opens
  useEffect(() => {
    if (showSettingsTab) {
      setSettingsScrollOffset(0);
      setSettingsFilter('');
      setIsSettingsFilterMode(false);
    }
  }, [showSettingsTab]);

  // Reset selection when settings filter changes
  useEffect(() => {
    if (filteredSettingsProviders.length > 0) {
      setSelectedSettingsIdx(0);
      setSettingsScrollOffset(0);
    }
  }, [settingsFilter]);

  // Enable mouse tracking for model selector and settings tab scrolling
  useEffect(() => {
    if (showModelSelector || showSettingsTab) {
      // Enable mouse button event tracking (clicks and scroll wheel)
      process.stdout.write('\x1b[?1000h');
      return () => {
        // Disable mouse tracking on unmount or when screens close
        process.stdout.write('\x1b[?1000l');
      };
    }
  }, [showModelSelector, showSettingsTab]);

  // TUI-031: Hide tok/s after 10 seconds of no chunks
  useEffect(() => {
    if (!isLoading || lastChunkTime === null) return;
    const timeout = setTimeout(() => {
      setDisplayedTokPerSec(null);
    }, 10000);
    return () => clearTimeout(timeout);
  }, [isLoading, lastChunkTime]);

  // Initialize session when view opens
  useEffect(() => {
    const initSession = async () => {
      try {
        // Dynamic import to handle ESM
        const codeletNapi = await import('@sengac/codelet-napi');
        const {
          CodeletSession,
          persistenceSetDataDirectory,
          persistenceGetHistory,
          modelsListAll: loadModels,
          modelsSetCacheDirectory,
        } = codeletNapi;

        // NAPI-006: Set up persistence data directory
        const fspecDir = getFspecUserDir();
        try {
          persistenceSetDataDirectory(fspecDir);
          // TUI-034: Set up model cache directory
          modelsSetCacheDirectory(`${fspecDir}/cache`);
        } catch {
          // Ignore if already set
        }

        // Wire up Rust tracing to TypeScript logger
        try {
          const { setRustLogCallback } = await import('@sengac/codelet-napi');
          setRustLogCallback((msg: string) => {
            // Route Rust logs through TypeScript logger
            if (msg.includes('[RUST:ERROR]')) {
              logger.error(msg);
            } else if (msg.includes('[RUST:WARN]')) {
              logger.warn(msg);
            } else if (msg.includes('[RUST:DEBUG]')) {
              logger.debug(msg);
            } else {
              logger.info(msg);
            }
          });
        } catch (err) {
          logger.warn('Failed to set up Rust log callback', { error: err });
        }

        // TUI-034: Load models and build provider sections
        let allModels: NapiProviderModels[] = [];
        try {
          allModels = await loadModels();
          logger.debug(`Loaded ${allModels.length} providers from models.dev`);
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          logger.error(`Failed to load models from models.dev: ${errorMsg}`);
        }

        // Build provider sections: filter by credentials, keep even if no compatible models
        // TUI-034: Show all providers with credentials so we can display "No compatible models" message
        // CONFIG-004: TypeScript-side credential check for all 19 providers (including .env loading)
        const sectionsWithCreds = await Promise.all(
          allModels.map(async pm => {
            const internalName = mapProviderIdToInternal(pm.providerId);
            const registryId = mapModelsDevToRegistryId(pm.providerId);
            const registryEntry = getProviderRegistryEntry(registryId);
            const providerConfig = await getProviderConfig(registryId);
            const hasCredentials =
              registryEntry?.requiresApiKey === false ||
              !!providerConfig.apiKey;
            const toolCallModels = pm.models.filter(m => m.toolCall);
            logger.debug(
              `Provider ${pm.providerId}: registryId=${registryId}, hasApiKey=${!!providerConfig.apiKey}, source=${providerConfig.source}, hasCredentials=${hasCredentials}`
            );
            return {
              providerId: pm.providerId,
              providerName: pm.providerName,
              internalName,
              models: toolCallModels,
              hasCredentials,
            };
          })
        );
        const sections: ProviderSection[] = sectionsWithCreds.filter(
          s => s.hasCredentials
        );
        logger.debug(
          `Found ${sections.length} providers with credentials (from ${sectionsWithCreds.length} total)`
        );

        setProviderSections(sections);
        // TUI-034: Only include providers with compatible models in availableProviders for actual use
        setAvailableProviders(
          sections.filter(s => s.models.length > 0).map(s => s.internalName)
        );

        // TUI-035: Load persisted model selection from config
        let persistedModelString: string | null = null;
        try {
          const config = await loadConfig();
          persistedModelString = config?.tui?.lastUsedModel || null;
          if (persistedModelString) {
            logger.debug(
              `Found persisted model selection: ${persistedModelString}`
            );
          }
        } catch (err) {
          logger.warn(
            'Failed to load config for persisted model, using default',
            { error: err }
          );
        }

        // Find default model (first available with tool_call=true)
        let defaultModelString = 'anthropic/claude-sonnet-4'; // Fallback
        let defaultModelInfo: NapiModelInfo | null = null;
        let defaultSection: ProviderSection | null = null;

        // TUI-035: Check if persisted model is available and has credentials
        if (persistedModelString && persistedModelString.includes('/')) {
          const [persistedProviderId, persistedModelId] =
            persistedModelString.split('/');
          const persistedSection = sections.find(
            s => s.providerId === persistedProviderId
          );

          if (persistedSection && persistedSection.hasCredentials) {
            // Find the model in the section
            const persistedModel = persistedSection.models.find(
              m => extractModelIdForRegistry(m.id) === persistedModelId
            );

            if (persistedModel) {
              // Use persisted model
              defaultModelString = persistedModelString;
              defaultModelInfo = persistedModel;
              defaultSection = persistedSection;
              logger.info(`Restored persisted model: ${persistedModelString}`);
            } else {
              logger.info(
                `Persisted model ${persistedModelId} not found in ${persistedProviderId}, using default`
              );
            }
          } else if (persistedSection && !persistedSection.hasCredentials) {
            logger.info(
              `Persisted provider ${persistedProviderId} has no credentials, using default`
            );
          } else {
            logger.info(
              `Persisted provider ${persistedProviderId} not available, using default`
            );
          }
        }

        // Use first available model if no persisted model was restored
        if (!defaultModelInfo && sections.length > 0) {
          defaultSection = sections[0];
          if (defaultSection.models.length > 0) {
            defaultModelInfo = defaultSection.models[0];
            // Extract model-id from the API ID (e.g., "claude-sonnet-4-20250514" -> "claude-sonnet-4")
            const modelId = extractModelIdForRegistry(defaultModelInfo.id);
            defaultModelString = `${defaultSection.providerId}/${modelId}`;
          }
        }

        // TUI-034: Create session with model using newWithModel factory
        // Must use newWithModel() to enable mid-session model switching via selectModel()
        let newSession: CodeletSessionType;
        let sessionHasModelSupport = false;

        // Try creating session with the default model
        try {
          newSession = await CodeletSession.newWithModel(defaultModelString);
          sessionHasModelSupport = true;
        } catch (err) {
          logger.warn(
            `Failed to create session with model ${defaultModelString}, trying fallbacks`,
            { error: err }
          );

          // TUI-034 FIX: Try fallback models to maintain model support
          // Without model support, Tab key model switching won't work
          const fallbackModels = [
            'anthropic/claude-sonnet-4',
            'google/gemini-2.0-flash',
            'openai/gpt-4o',
          ];

          for (const fallbackModel of fallbackModels) {
            if (fallbackModel === defaultModelString) continue; // Skip if same as already tried
            try {
              newSession = await CodeletSession.newWithModel(fallbackModel);
              sessionHasModelSupport = true;
              logger.info(
                `Successfully created session with fallback model: ${fallbackModel}`
              );
              // Update defaultModelInfo to match what we actually created
              const [providerId, modelId] = fallbackModel.split('/');
              const section = sections.find(s => s.providerId === providerId);
              if (section) {
                const model = section.models.find(
                  m => extractModelIdForRegistry(m.id) === modelId
                );
                if (model) {
                  defaultModelInfo = model;
                  defaultSection = section;
                }
              }
              break;
            } catch (fallbackErr) {
              logger.warn(`Fallback model ${fallbackModel} also failed`, {
                error: fallbackErr,
              });
            }
          }

          // Last resort: basic session without model support
          if (!sessionHasModelSupport) {
            logger.error(
              'All newWithModel attempts failed, falling back to basic session (model switching disabled)'
            );
            newSession = new CodeletSession('claude');
            // Clear model info since model switching won't work
            defaultModelInfo = null;
            defaultSection = null;
          }
        }

        setSession(newSession);
        sessionRef.current = newSession;
        setCurrentProvider(newSession.currentProviderName);
        setTokenUsage(newSession.tokenTracker);

        // TUI-034: Set current model info (only if session has model support)
        if (sessionHasModelSupport && defaultModelInfo && defaultSection) {
          const modelId = extractModelIdForRegistry(defaultModelInfo.id);
          setCurrentModel({
            providerId: defaultSection.providerId,
            modelId,
            apiModelId: defaultModelInfo.id,
            displayName: defaultModelInfo.name,
            reasoning: defaultModelInfo.reasoning,
            hasVision: defaultModelInfo.hasVision,
            contextWindow: defaultModelInfo.contextWindow,
            maxOutput: defaultModelInfo.maxOutput,
          });
          // Expand current provider's section by default
          setExpandedProviders(new Set([defaultSection.providerId]));
        } else if (!sessionHasModelSupport) {
          // Clear model info - model switching won't work without model support
          setCurrentModel(null);
        }

        // NAPI-006: Session creation is deferred until first message is sent
        // This prevents empty sessions from being persisted when user opens
        // the modal but doesn't send any messages. See handleSubmit() for
        // the actual session creation logic.

        // NAPI-006: Load history for current project
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

        setError(null);
      } catch (err) {
        const errorMessage =
          err instanceof Error
            ? err.message
            : 'Failed to initialize AI session';
        setError(errorMessage);
        setSession(null);
        sessionRef.current = null;
      }
    };

    void initSession();
  }, []);

  // Handle sending a prompt
  const handleSubmit = useCallback(async () => {
    if (!sessionRef.current || !inputValue.trim() || isLoading) return;

    const userMessage = inputValue.trim();

    // AGENT-021: Handle /debug command - toggle debug capture mode
    if (userMessage === '/debug') {
      setInputValue('');
      try {
        // Pass ~/.fspec as the debug directory
        const result = sessionRef.current.toggleDebug(getFspecUserDir());
        setIsDebugEnabled(result.enabled);
        // Add the result message to conversation
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: result.message },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to toggle debug mode';
        setError(errorMessage);
      }
      return;
    }

    // NAPI-006: Handle /search command - enter history search mode
    if (userMessage === '/search') {
      setInputValue('');
      handleSearchMode();
      return;
    }

    // AGENT-003: Handle /clear command - clear context and reset session
    if (userMessage === '/clear') {
      setInputValue('');
      try {
        // Clear history in the Rust session (includes reinjecting context reminders)
        sessionRef.current.clearHistory();
        // Reset React state
        setConversation([]);
        setTokenUsage({ inputTokens: 0, outputTokens: 0 });
        setContextFillPercentage(0);
        // Note: currentProvider, isDebugEnabled, and historyEntries are preserved
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to clear session';
        setError(errorMessage);
      }
      return;
    }

    // NAPI-006: Handle /history command - show command history
    if (userMessage === '/history' || userMessage.startsWith('/history ')) {
      setInputValue('');
      const allProjects = userMessage.includes('--all-projects');
      try {
        const { persistenceGetHistory } = await import('@sengac/codelet-napi');
        const history = persistenceGetHistory(
          allProjects ? null : currentProjectRef.current,
          20
        );
        if (history.length === 0) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: 'No history entries found' },
          ]);
        } else {
          const historyList = history
            .map(
              (h: { display: string; timestamp: string }) => `- ${h.display}`
            )
            .join('\n');
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Command history:\n${historyList}` },
          ]);
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to get history';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `History failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-003: Handle /resume command - show session selection overlay
    if (userMessage === '/resume') {
      setInputValue('');
      void handleResumeMode();
      return;
    }

    // NAPI-006: Handle /sessions command - list all sessions
    if (userMessage === '/sessions') {
      setInputValue('');
      try {
        const { persistenceListSessions } = await import(
          '@sengac/codelet-napi'
        );
        const sessions = persistenceListSessions(currentProjectRef.current);
        if (sessions.length === 0) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: 'No sessions found for this project' },
          ]);
        } else {
          const sessionList = sessions
            .map(
              (s: SessionManifest) =>
                `- ${s.name} (${s.messageCount} messages, ${s.id.slice(0, 8)}...)`
            )
            .join('\n');
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Sessions:\n${sessionList}` },
          ]);
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to list sessions';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `List sessions failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /switch <name> command - switch to another session
    if (userMessage.startsWith('/switch ')) {
      setInputValue('');
      const targetName = userMessage.slice(8).trim();
      try {
        const { persistenceListSessions, persistenceLoadSession } =
          await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        const target = sessions.find(
          (s: SessionManifest) => s.name === targetName
        );
        if (target) {
          setCurrentSessionId(target.id);
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Switched to session: "${target.name}"` },
          ]);
        } else {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Session not found: "${targetName}"` },
          ]);
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to switch session';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Switch failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /rename <new-name> command - rename current session
    if (userMessage.startsWith('/rename ')) {
      setInputValue('');
      const newName = userMessage.slice(8).trim();
      if (!currentSessionId) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'No active session to rename' },
        ]);
        return;
      }
      try {
        const { persistenceRenameSession } = await import(
          '@sengac/codelet-napi'
        );
        persistenceRenameSession(currentSessionId, newName);
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Session renamed to: "${newName}"` },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to rename session';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Rename failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /fork <index> <name> command - fork session at index
    if (userMessage.startsWith('/fork ')) {
      setInputValue('');
      const parts = userMessage.slice(6).trim().split(/\s+/);
      const index = parseInt(parts[0], 10);
      const name = parts.slice(1).join(' ');
      if (!currentSessionId) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'No active session to fork' },
        ]);
        return;
      }
      if (isNaN(index) || !name) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'Usage: /fork <index> <name>' },
        ]);
        return;
      }
      try {
        const { persistenceForkSession } = await import('@sengac/codelet-napi');
        const forkedSession = persistenceForkSession(
          currentSessionId,
          index,
          name
        );
        setCurrentSessionId(forkedSession.id);
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `Session forked at index ${index}: "${name}"`,
          },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to fork session';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Fork failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /merge <session> <indices> command - merge messages from another session
    if (userMessage.startsWith('/merge ')) {
      setInputValue('');
      const parts = userMessage.slice(7).trim().split(/\s+/);
      const sourceName = parts[0];
      const indicesStr = parts[1];
      if (!currentSessionId) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'No active session to merge into' },
        ]);
        return;
      }
      if (!sourceName || !indicesStr) {
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content:
              'Usage: /merge <session-name> <indices> (e.g., /merge session-b 3,4)',
          },
        ]);
        return;
      }
      try {
        const { persistenceListSessions, persistenceMergeMessages } =
          await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        const source = sessions.find(
          (s: SessionManifest) => s.name === sourceName || s.id === sourceName
        );
        if (!source) {
          setConversation(prev => [
            ...prev,
            {
              role: 'tool',
              content: `Source session not found: "${sourceName}"`,
            },
          ]);
          return;
        }
        const indices = indicesStr
          .split(',')
          .map((s: string) => parseInt(s.trim(), 10));
        const result = persistenceMergeMessages(
          currentSessionId,
          source.id,
          indices
        );
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `Merged ${indices.length} messages from "${source.name}"`,
          },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to merge messages';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Merge failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-006: Handle /cherry-pick <session> <index> --context <n> command
    if (userMessage.startsWith('/cherry-pick ')) {
      setInputValue('');
      const args = userMessage.slice(13).trim();
      const contextMatch = args.match(/--context\s+(\d+)/);
      const context = contextMatch ? parseInt(contextMatch[1], 10) : 0;
      const cleanArgs = args.replace(/--context\s+\d+/, '').trim();
      const parts = cleanArgs.split(/\s+/);
      const sourceName = parts[0];
      const index = parseInt(parts[1], 10);
      if (!currentSessionId) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'No active session for cherry-pick' },
        ]);
        return;
      }
      if (!sourceName || isNaN(index)) {
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: 'Usage: /cherry-pick <session> <index> [--context N]',
          },
        ]);
        return;
      }
      try {
        const { persistenceListSessions, persistenceCherryPick } = await import(
          '@sengac/codelet-napi'
        );
        const sessions = persistenceListSessions(currentProjectRef.current);
        const source = sessions.find(
          (s: SessionManifest) => s.name === sourceName || s.id === sourceName
        );
        if (!source) {
          setConversation(prev => [
            ...prev,
            {
              role: 'tool',
              content: `Source session not found: "${sourceName}"`,
            },
          ]);
          return;
        }
        const result = persistenceCherryPick(
          currentSessionId,
          source.id,
          index,
          context
        );
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `Cherry-picked message ${index} with ${context} context messages from "${source.name}"`,
          },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to cherry-pick';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Cherry-pick failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    // NAPI-005: Handle /compact command - manual context compaction
    if (userMessage === '/compact') {
      setInputValue('');

      // Check if there's anything to compact - use session's messages, not React state
      if (sessionRef.current.messages.length === 0) {
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: 'Nothing to compact - no messages yet' },
        ]);
        return;
      }

      // Show compacting message
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: '[Compacting context...]' },
      ]);

      try {
        const result = await sessionRef.current.compact();
        // Show success message with metrics
        const compressionPct = result.compressionRatio.toFixed(0);
        const message = `[Context compacted: ${result.originalTokens}→${result.compactedTokens} tokens, ${compressionPct}% compression]\n[Summarized ${result.turnsSummarized} turns, kept ${result.turnsKept} turns]`;
        setConversation(prev => [...prev, { role: 'tool', content: message }]);
        // Update token tracker and context fill to reflect reduced context
        const finalTokens = sessionRef.current.tokenTracker;
        setTokenUsage(finalTokens);
        const contextFillInfo = sessionRef.current.getContextFillInfo();
        setContextFillPercentage(contextFillInfo.fillPercentage);

        // Persist compaction state and token usage
        if (currentSessionId) {
          try {
            const {
              persistenceSetCompactionState,
              persistenceSetSessionTokens,
            } = await import('@sengac/codelet-napi');
            // Create summary for persistence (includes key metrics)
            const summary = `Compacted ${result.turnsSummarized} turns (${result.originalTokens}→${result.compactedTokens} tokens, ${compressionPct}% compression)`;
            // compacted_before_index = turnsSummarized (messages 0 to turnsSummarized-1 were compacted)
            persistenceSetCompactionState(
              currentSessionId,
              summary,
              result.turnsSummarized
            );
            persistenceSetSessionTokens(
              currentSessionId,
              finalTokens.inputTokens,
              finalTokens.outputTokens,
              finalTokens.cacheReadInputTokens ?? 0,
              finalTokens.cacheCreationInputTokens ?? 0,
              finalTokens.cumulativeBilledInput ?? finalTokens.inputTokens,
              finalTokens.cumulativeBilledOutput ?? finalTokens.outputTokens
            );
          } catch {
            // Compaction state persistence failed - continue
          }
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to compact context';
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Compaction failed: ${errorMessage}` },
        ]);
      }
      return;
    }

    setInputValue('');
    setHistoryIndex(-1); // Reset history navigation
    setSavedInput('');
    setIsLoading(true);
    // TUI-031: Reset tok/s display for new prompt (Rust will send new values)
    setDisplayedTokPerSec(null);
    setLastChunkTime(null);

    // NAPI-006: Deferred session creation - only create session on first message
    // This prevents empty sessions from being persisted when user opens modal
    // but doesn't send any messages
    // TUI-034: Store full model path (provider/model-id) for proper restore
    let activeSessionId = currentSessionId;
    if (!activeSessionId && isFirstMessageRef.current) {
      try {
        const { persistenceCreateSessionWithProvider } = await import(
          '@sengac/codelet-napi'
        );
        const project = currentProjectRef.current;
        // Use first message as session name (truncated to 50 chars)
        const sessionName =
          userMessage.slice(0, 50) + (userMessage.length > 50 ? '...' : '');

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
        setCurrentSessionId(activeSessionId);
        // Mark first message as processed (session already named with message content)
        isFirstMessageRef.current = false;
      } catch {
        // Session creation failed - continue without persistence
      }
    }

    // NAPI-006: Save command to history
    if (activeSessionId) {
      try {
        const { persistenceAddHistory } = await import('@sengac/codelet-napi');
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
      logger.warn('No activeSessionId - history will not be saved');
    }

    // Add user message to conversation
    setConversation(prev => [...prev, { role: 'user', content: userMessage }]);

    // Persist user message as full envelope
    if (activeSessionId) {
      try {
        // Create proper user message envelope
        // Note: "type" field matches Rust's #[serde(rename = "type")] for message_type
        const userEnvelope = {
          uuid: crypto.randomUUID(),
          timestamp: new Date().toISOString(),
          type: 'user',
          provider: currentProvider,
          message: {
            role: 'user',
            content: [{ type: 'text', text: userMessage }],
          },
        };
        const envelopeJson = JSON.stringify(userEnvelope);
        persistenceStoreMessageEnvelope(activeSessionId, envelopeJson);

        // Note: Session naming now happens at creation time (deferred session creation above)
        // so we don't need to rename here
      } catch {
        // User message persistence failed - continue
      }
    }

    // Add streaming assistant message placeholder
    setConversation(prev => [
      ...prev,
      { role: 'assistant', content: '', isStreaming: true },
    ]);

    try {
      // TOOL-010: Detect thinking level from prompt keywords
      const thinkingLevel = detectThinkingLevel(userMessage);
      setDetectedThinkingLevel(thinkingLevel);

      // Get thinking config JSON if level is not Off
      let thinkingConfig: string | null = null;
      if (thinkingLevel !== JsThinkingLevel.Off) {
        thinkingConfig = getThinkingConfig(currentProvider, thinkingLevel);
        const label = getThinkingLevelLabel(thinkingLevel);
        if (label) {
          logger.debug(`Thinking level detected: ${label}`);
        }
      }

      // Track current text segment (resets after tool calls)
      let currentSegment = '';
      // Track full assistant response for persistence (includes ALL content blocks)
      let fullAssistantResponse = '';
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
      await sessionRef.current.prompt(
        userMessage,
        thinkingConfig,
        (chunk: StreamChunk) => {
          if (!chunk) return;

          if (chunk.type === 'Text' && chunk.text) {
            // Text chunks are batched in Rust for efficiency
            currentSegment += chunk.text;
            fullAssistantResponse += chunk.text; // Accumulate for display persistence
            // Add to content blocks for envelope storage
            const lastBlock =
              assistantContentBlocks[assistantContentBlocks.length - 1];
            if (lastBlock && lastBlock.type === 'text') {
              lastBlock.text = (lastBlock.text || '') + chunk.text;
            } else {
              assistantContentBlocks.push({ type: 'text', text: chunk.text });
            }
            // Update streaming message content
            // Note: Ink already throttles terminal output to 30fps, so direct state
            // updates are fine here. The expensive part (line wrapping) is cached by PERF-002.
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
            // TOOL-010: Handle thinking/reasoning content from extended thinking
            // Store thinking block for envelope persistence
            assistantContentBlocks.push({
              type: 'thinking',
              thinking: chunk.thinking,
            });
            // Display thinking content in a distinct way (could be collapsible in future)
            setConversation(prev => {
              const updated = [...prev];
              // Add thinking as a separate tool-style message with distinct formatting
              updated.push({
                role: 'tool',
                content: `[Thinking]\n${chunk.thinking}`,
              });
              return updated;
            });
          } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
            // Finalize current streaming message and add tool call (match CLI format)
            const toolCall = chunk.toolCall;

            // Add tool_use block to content blocks for envelope storage
            let parsedInput: unknown;
            try {
              parsedInput = JSON.parse(toolCall.input);
            } catch {
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
                pendingToolDiffsRef.current.set(toolCall.id, {
                  toolName: 'Edit',
                  toolCallId: toolCall.id,
                  oldString: inputObj.old_string,
                  newString: inputObj.new_string,
                });
                // Handle both Claude (write) and Gemini (write_file) tool names
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

            // TUI-037: Format tool header in Claude Code style: ● ToolName(args)
            let argsDisplay = '';
            if (typeof parsedInput === 'object' && parsedInput !== null) {
              const inputObj = parsedInput as Record<string, unknown>;
              // For Bash tool, show command; for others, show first arg or summary
              if (inputObj.command) {
                argsDisplay = String(inputObj.command);
              } else if (inputObj.file_path) {
                argsDisplay = String(inputObj.file_path);
              } else if (inputObj.pattern) {
                argsDisplay = String(inputObj.pattern);
              } else {
                // Show first key-value or JSON summary
                const entries = Object.entries(inputObj);
                if (entries.length > 0) {
                  const [key, value] = entries[0];
                  argsDisplay =
                    typeof value === 'string'
                      ? value
                      : `${key}: ${JSON.stringify(value).slice(0, 50)}`;
                }
              }
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
                updated[updated.length - 1].role === 'assistant' &&
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
                role: 'tool',
                content: toolContentSnapshot,
              });
              return updated;
            });
          } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
            // Show tool result in CLI format, then start new streaming message
            const result = chunk.toolResult;

            // NAPI-008: Store current assistant envelope BEFORE the tool_result
            // This ensures the assistant message with tool_use is separate from the continuation
            if (activeSessionId && assistantContentBlocks.length > 0) {
              try {
                const assistantEnvelope = {
                  uuid: crypto.randomUUID(),
                  timestamp: new Date().toISOString(),
                  type: 'assistant',
                  provider: currentProvider,
                  message: {
                    role: 'assistant',
                    content: [...assistantContentBlocks], // Clone before clearing
                  },
                };
                persistenceStoreMessageEnvelope(
                  activeSessionId,
                  JSON.stringify(assistantEnvelope)
                );
                // Clear for continuation after tool_result
                assistantContentBlocks.length = 0;
              } catch {
                // Persistence failed - continue
              }
            }

            // Store tool_result as user message immediately
            if (activeSessionId) {
              try {
                const toolResultEnvelope = {
                  uuid: crypto.randomUUID(),
                  timestamp: new Date().toISOString(),
                  type: 'user',
                  provider: currentProvider,
                  message: {
                    role: 'user',
                    content: [
                      {
                        type: 'tool_result',
                        tool_use_id: result.toolCallId,
                        content: result.content,
                        is_error: result.isError,
                      },
                    ],
                  },
                };
                persistenceStoreMessageEnvelope(
                  activeSessionId,
                  JSON.stringify(toolResultEnvelope)
                );
              } catch {
                // Persistence failed - continue
              }
            }

            // TUI-037 + TUI-038: Sanitize and format with collapsed output style
            // Check for Edit/Write tool diff display
            const pendingDiff = pendingToolDiffsRef.current.get(
              result.toolCallId
            );
            let toolResultContent: string;

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
                toolResultContent = formatDiffForDisplay(diffLines);
              } else if (
                pendingDiff.toolName === 'Write' &&
                pendingDiff.content !== undefined
              ) {
                const diffLines = formatWriteDiff(pendingDiff.content);
                toolResultContent = formatDiffForDisplay(diffLines);
              } else {
                // Fallback to normal formatting
                const sanitizedContent = result.content.replace(/\t/g, '  ');
                toolResultContent = formatCollapsedOutput(sanitizedContent);
              }
            } else {
              // Normal tool result formatting
              const sanitizedContent = result.content.replace(/\t/g, '  ');
              toolResultContent = formatCollapsedOutput(sanitizedContent);
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
                    msg.role === 'tool' &&
                    msg.content.includes('[Tool output]')
                  ) {
                    updated.splice(i, 1);
                    continue;
                  }
                  // TUI-037: Combine tool header with formatted result
                  if (msg.role === 'tool' && msg.content.startsWith('●')) {
                    const headerLine = msg.content.split('\n')[0];
                    updated[i] = {
                      ...msg,
                      content: `${headerLine}\n${toolResultContent}`,
                    };
                    break;
                  }
                }
                return [
                  ...updated,
                  // Add new streaming placeholder for AI continuation
                  {
                    role: 'assistant' as const,
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
                  if (msg.role === 'tool' && msg.content.startsWith('●')) {
                    const headerLine = msg.content.split('\n')[0];
                    updated[i] = {
                      ...msg,
                      content: `${headerLine}\n${toolResultContent}`,
                    };
                    break;
                  }
                }
                return [
                  ...updated,
                  // Add new streaming placeholder for AI continuation
                  {
                    role: 'assistant' as const,
                    content: '',
                    isStreaming: true,
                  },
                ];
              });
            }
          } else if (chunk.type === 'Done') {
            // Mark streaming complete and remove empty trailing assistant messages
            setConversation(prev => {
              const updated = [...prev];
              // Remove empty streaming assistant messages at the end
              while (
                updated.length > 0 &&
                updated[updated.length - 1].role === 'assistant' &&
                updated[updated.length - 1].isStreaming &&
                !updated[updated.length - 1].content
              ) {
                updated.pop();
              }
              // Mark any remaining streaming message as complete
              const lastAssistantIdx = updated.findLastIndex(
                m => m.role === 'assistant' && m.isStreaming
              );
              if (lastAssistantIdx >= 0) {
                updated[lastAssistantIdx] = {
                  ...updated[lastAssistantIdx],
                  isStreaming: false,
                };
              }
              return updated;
            });
          } else if (chunk.type === 'Status' && chunk.status) {
            const statusMessage = chunk.status;
            // Status messages (e.g., compaction notifications)
            setConversation(prev => [
              ...prev,
              {
                role: 'tool',
                content: statusMessage,
              },
            ]);
          } else if (chunk.type === 'Interrupted') {
            // Agent was interrupted by user
            // TUI-037: Only append to tool if it's still streaming (no collapse indicator)
            // If tool has collapse indicator, it completed - interrupt is for AI continuation
            setConversation(prev => {
              const updated = [...prev];

              // First, remove empty streaming assistant messages
              while (
                updated.length > 0 &&
                updated[updated.length - 1].role === 'assistant' &&
                updated[updated.length - 1].isStreaming &&
                !updated[updated.length - 1].content
              ) {
                updated.pop();
              }

              // Find the last tool message
              let handledInterrupt = false;
              for (let i = updated.length - 1; i >= 0; i--) {
                const msg = updated[i];
                if (msg.role === 'tool' && msg.content.startsWith('●')) {
                  // Only append if tool is still streaming (no collapse indicator = no ToolResult yet)
                  if (!msg.content.includes('(ctrl+o to expand)')) {
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
                  role: 'tool' as const,
                  content: '⚠ Interrupted',
                });
              }

              // Mark any remaining streaming message as complete
              const lastAssistantIdx = updated.findLastIndex(
                m => m.role === 'assistant' && m.isStreaming
              );
              if (lastAssistantIdx >= 0) {
                updated[lastAssistantIdx] = {
                  ...updated[lastAssistantIdx],
                  isStreaming: false,
                };
              }
              return updated;
            });
          } else if (chunk.type === 'TokenUpdate' && chunk.tokens) {
            // TUI-031: Display token counts and tok/s from Rust
            setTokenUsage(chunk.tokens);
            if (
              chunk.tokens.tokensPerSecond !== undefined &&
              chunk.tokens.tokensPerSecond !== null
            ) {
              setDisplayedTokPerSec(chunk.tokens.tokensPerSecond);
              setLastChunkTime(Date.now());
            }
          } else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
            // TUI-033: Display context fill percentage from Rust
            setContextFillPercentage(chunk.contextFill.fillPercentage);
          } else if (chunk.type === 'ToolProgress' && chunk.toolProgress) {
            // TOOL-011 + TUI-037: Stream tool execution progress with rolling window
            // Display the output chunk in a fixed-height window (last N lines)
            hasStreamedToolProgress = true;
            const outputChunk = chunk.toolProgress.outputChunk;
            setConversation(prev => {
              const updated = [...prev];
              const lastIdx = updated.length - 1;
              if (lastIdx >= 0) {
                const lastMsg = updated[lastIdx];
                // TUI-037: If last message is a tool header (●), append streaming output with tree connectors
                if (
                  lastMsg.role === 'tool' &&
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
                  lastMsg.role === 'tool' &&
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
                    role: 'tool',
                    content: `[Tool output]\n${outputChunk}`,
                  });
                }
              }
              return updated;
            });
          } else if (chunk.type === 'Error' && chunk.error) {
            // API error occurred - clean up streaming placeholder and show error in conversation
            setConversation(prev => {
              const updated = [...prev];
              // Remove empty streaming assistant messages at the end
              while (
                updated.length > 0 &&
                updated[updated.length - 1].role === 'assistant' &&
                updated[updated.length - 1].isStreaming &&
                !updated[updated.length - 1].content
              ) {
                updated.pop();
              }
              // Add error as tool message so it's visible in conversation
              updated.push({
                role: 'tool',
                content: `API Error: ${chunk.error}`,
              });
              return updated;
            });
          }
        }
      );

      // Persist full envelopes to session (includes tool calls and results)
      if (activeSessionId) {
        try {
          // Store assistant message with ALL content blocks (text + tool_use)
          // Note: "type" field matches Rust's #[serde(rename = "type")] for message_type
          if (assistantContentBlocks.length > 0) {
            // Per-message cumulative token usage for analytics/debugging (NAPI-008)
            // Note: This is cumulative session usage at time of message, stored for
            // historical analysis. Not used during restore (session totals are in manifest).
            const currentTokens = sessionRef.current?.tokenTracker;
            const assistantEnvelope = {
              uuid: crypto.randomUUID(),
              timestamp: new Date().toISOString(),
              type: 'assistant',
              provider: currentProvider,
              message: {
                role: 'assistant',
                content: assistantContentBlocks,
              },
              usage: currentTokens
                ? {
                    input_tokens: currentTokens.inputTokens,
                    output_tokens: currentTokens.outputTokens,
                    cache_read_input_tokens:
                      currentTokens.cacheReadInputTokens ?? 0,
                    cache_creation_input_tokens:
                      currentTokens.cacheCreationInputTokens ?? 0,
                  }
                : undefined,
            };
            const assistantJson = JSON.stringify(assistantEnvelope);
            persistenceStoreMessageEnvelope(activeSessionId, assistantJson);
          }
          // Note: Tool results are stored immediately in ToolResult handler (NAPI-008)
        } catch {
          // Message persistence failed - continue
        }
      }

      // Update token usage after prompt completes (safe to access now - session unlocked)
      if (sessionRef.current) {
        const finalTokens = sessionRef.current.tokenTracker;
        setTokenUsage(finalTokens);

        // Persist token usage to session manifest (for restore)
        if (activeSessionId) {
          try {
            const { persistenceSetSessionTokens } = await import(
              '@sengac/codelet-napi'
            );
            persistenceSetSessionTokens(
              activeSessionId,
              finalTokens.inputTokens,
              finalTokens.outputTokens,
              finalTokens.cacheReadInputTokens ?? 0,
              finalTokens.cacheCreationInputTokens ?? 0,
              finalTokens.cumulativeBilledInput ?? finalTokens.inputTokens,
              finalTokens.cumulativeBilledOutput ?? finalTokens.outputTokens
            );
          } catch {
            // Token usage persistence failed - continue
          }
        }
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to send prompt';
      // Clean up streaming placeholder and show error in conversation
      setConversation(prev => {
        const updated = [...prev];
        // Remove empty streaming assistant messages at the end
        while (
          updated.length > 0 &&
          updated[updated.length - 1].role === 'assistant' &&
          updated[updated.length - 1].isStreaming &&
          !updated[updated.length - 1].content
        ) {
          updated.pop();
        }
        // Add error as tool message so it's visible in conversation
        updated.push({ role: 'tool', content: `Error: ${errorMessage}` });
        return updated;
      });
    } finally {
      setIsLoading(false);
    }
  }, [inputValue, isLoading]);

  // Handle provider switching
  const handleSwitchProvider = useCallback(async (providerName: string) => {
    if (!sessionRef.current) return;

    try {
      setIsLoading(true);
      await sessionRef.current.switchProvider(providerName);
      setCurrentProvider(providerName);
      setShowProviderSelector(false);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to switch provider';
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // CONFIG-004: Load provider statuses (API key presence and masked display)
  const loadProviderStatuses = useCallback(async () => {
    const statuses: Record<string, { hasKey: boolean; maskedKey?: string }> =
      {};
    for (const providerId of SUPPORTED_PROVIDERS) {
      try {
        const config = await getProviderConfig(providerId);
        if (config.apiKey) {
          statuses[providerId] = {
            hasKey: true,
            maskedKey: maskApiKey(config.apiKey),
          };
        } else {
          statuses[providerId] = { hasKey: false };
        }
      } catch {
        statuses[providerId] = { hasKey: false };
      }
    }
    setProviderStatuses(statuses);
  }, []);

  // CONFIG-004: Handle saving API key for a provider
  const handleSaveApiKey = useCallback(
    async (providerId: string, apiKey: string) => {
      try {
        await saveCredential(providerId, apiKey);
        setEditingProviderId(null);
        setEditingApiKey('');
        setConnectionTestResult(null);
        // Reload statuses to reflect the change
        await loadProviderStatuses();
        // Show success message briefly
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `✓ API key saved for ${providerId}. Refreshing models...`,
          },
        ]);
        // Refresh models to pick up newly available providers
        // Use setTimeout to allow the success message to appear first
        setTimeout(async () => {
          await modelsRefreshCache();
          // Rebuild provider sections with new credentials
          const codeletNapi = await import('@sengac/codelet-napi');
          const { modelsListAll: loadModels } = codeletNapi;

          let allModels: NapiProviderModels[] = [];
          try {
            allModels = await loadModels();
          } catch {
            // Ignore
          }

          // CONFIG-004: Use TypeScript-side credential check for all 19 providers
          const sectionsWithCreds = await Promise.all(
            allModels.map(async pm => {
              const internalName = mapProviderIdToInternal(pm.providerId);
              const registryId = mapModelsDevToRegistryId(pm.providerId);
              const registryEntry = getProviderRegistryEntry(registryId);
              const providerConfig = await getProviderConfig(registryId);
              const hasCredentials =
                registryEntry?.requiresApiKey === false ||
                !!providerConfig.apiKey;
              const toolCallModels = pm.models.filter(m => m.toolCall);
              return {
                providerId: pm.providerId,
                providerName: pm.providerName,
                internalName,
                models: toolCallModels,
                hasCredentials,
              };
            })
          );
          const sections: ProviderSection[] = sectionsWithCreds.filter(
            s => s.hasCredentials
          );

          setProviderSections(sections);
          setAvailableProviders(
            sections.filter(s => s.models.length > 0).map(s => s.internalName)
          );

          setConversation(prev => [
            ...prev,
            {
              role: 'tool',
              content: `✓ Models refreshed - ${sections.length} providers available`,
            },
          ]);
        }, 100);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to save API key';
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `✗ Failed to save API key: ${errorMessage}`,
          },
        ]);
      }
    },
    [loadProviderStatuses]
  );

  // CONFIG-004: Handle deleting API key for a provider
  const handleDeleteApiKey = useCallback(
    async (providerId: string) => {
      try {
        await deleteCredential(providerId);
        setConnectionTestResult(null);
        await loadProviderStatuses();
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `✓ API key deleted for ${providerId}` },
        ]);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to delete API key';
        setConversation(prev => [
          ...prev,
          {
            role: 'tool',
            content: `✗ Failed to delete API key: ${errorMessage}`,
          },
        ]);
      }
    },
    [loadProviderStatuses]
  );

  // CONFIG-004: Test provider connection with a lightweight API call
  const handleTestConnection = useCallback(async (providerId: string) => {
    setConnectionTestResult({
      providerId,
      success: false,
      message: 'Testing...',
    });
    try {
      // Get the internal name for the provider
      const internalName = mapProviderIdToInternal(providerId);

      // Try to create a session with this provider to test the connection
      const codeletNapi = await import('@sengac/codelet-napi');
      const { CodeletSession } = codeletNapi;

      // Attempt to create a session - this will fail if credentials are invalid
      const testSession = new CodeletSession(internalName);

      if (testSession) {
        setConnectionTestResult({
          providerId,
          success: true,
          message: '✓ Connection successful',
        });
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Connection failed';
      setConnectionTestResult({
        providerId,
        success: false,
        message: `✗ ${errorMessage}`,
      });
    }
  }, []);

  // CONFIG-004: Refresh models from models.dev and rebuild provider sections
  const refreshModels = useCallback(async () => {
    setIsRefreshingModels(true);
    try {
      // Refresh the cache from models.dev
      await modelsRefreshCache();

      // Fetch the updated models
      const codeletNapi = await import('@sengac/codelet-napi');
      const { modelsListAll: loadModels } = codeletNapi;

      let allModels: NapiProviderModels[] = [];
      try {
        allModels = await loadModels();
      } catch (err) {
        logger.warn('Failed to load models from models.dev', { error: err });
      }

      // Rebuild provider sections
      // CONFIG-004: Use TypeScript-side credential check for all 19 providers
      const sectionsWithCreds = await Promise.all(
        allModels.map(async pm => {
          const internalName = mapProviderIdToInternal(pm.providerId);
          const registryId = mapModelsDevToRegistryId(pm.providerId);
          // Get registry entry to check if API key is required
          const registryEntry = getProviderRegistryEntry(registryId);
          // Check credentials using TypeScript side (supports all 19 providers)
          const providerConfig = await getProviderConfig(registryId);
          // Provider is configured if: no API key required (e.g., Ollama) OR has API key
          const hasCredentials =
            registryEntry?.requiresApiKey === false || !!providerConfig.apiKey;
          const toolCallModels = pm.models.filter(m => m.toolCall);
          return {
            providerId: pm.providerId,
            providerName: pm.providerName,
            internalName,
            models: toolCallModels,
            hasCredentials,
          };
        })
      );
      const sections: ProviderSection[] = sectionsWithCreds.filter(
        s => s.hasCredentials
      );

      setProviderSections(sections);
      setAvailableProviders(
        sections.filter(s => s.models.length > 0).map(s => s.internalName)
      );

      // Show success message
      setConversation(prev => [
        ...prev,
        {
          role: 'tool',
          content: `✓ Models refreshed (${allModels.reduce((acc, pm) => acc + pm.models.length, 0)} models from ${allModels.length} providers)`,
        },
      ]);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to refresh models';
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: `✗ Refresh failed: ${errorMessage}` },
      ]);
    } finally {
      setIsRefreshingModels(false);
    }
  }, []);

  // TUI-034: Handle model selection
  const handleSelectModel = useCallback(
    async (section: ProviderSection, model: NapiModelInfo) => {
      if (!sessionRef.current) return;

      try {
        setIsLoading(true);
        // Extract model-id from the API ID for registry matching
        // IMPORTANT: Do NOT use model.family - it may be a generic family name (e.g., "gemini-pro")
        // that doesn't match registry keys. Instead, extract from model.id by stripping suffixes.
        // Examples:
        //   "claude-sonnet-4-20250514" -> "claude-sonnet-4" (strip date suffix)
        //   "gemini-2.5-pro-preview-06-05" -> "gemini-2.5-pro" (strip preview suffix)
        //   "gpt-4o" -> "gpt-4o" (no change)
        const modelId = extractModelIdForRegistry(model.id);
        const modelString = `${section.providerId}/${modelId}`;

        // Use selectModel to switch the model
        await sessionRef.current.selectModel(modelString);

        // Update state
        setCurrentModel({
          providerId: section.providerId,
          modelId,
          apiModelId: model.id,
          displayName: model.name,
          reasoning: model.reasoning,
          hasVision: model.hasVision,
          contextWindow: model.contextWindow,
          maxOutput: model.maxOutput,
        });
        setCurrentProvider(section.internalName);
        setShowModelSelector(false);

        // TUI-035: Persist model selection to user config
        try {
          const existingConfig = await loadConfig();
          const updatedConfig = {
            ...existingConfig,
            tui: {
              ...existingConfig?.tui,
              lastUsedModel: modelString,
            },
          };
          await writeConfig('user', updatedConfig);
          logger.debug(`Persisted model selection: ${modelString}`);
        } catch (persistErr) {
          // Log but don't fail - persistence is not critical
          logger.warn('Failed to persist model selection', {
            error: persistErr,
          });
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Failed to switch model';
        // TUI-034: Display error in conversation instead of global error state
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Model selection failed: ${errorMessage}` },
        ]);
        setShowModelSelector(false);
      } finally {
        setIsLoading(false);
      }
    },
    []
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
      const { persistenceSearchHistory } = await import('@sengac/codelet-napi');
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
    } catch {
      // Search is optional - continue without it
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

  // NAPI-003: Enter resume mode (show session selection overlay)
  const handleResumeMode = useCallback(async () => {
    try {
      const { persistenceListSessions } = await import('@sengac/codelet-napi');
      const sessions = persistenceListSessions(currentProjectRef.current);

      // Sort by updatedAt descending (most recent first)
      const sorted = [...sessions].sort(
        (a: SessionManifest, b: SessionManifest) =>
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
        { role: 'tool', content: `Resume failed: ${errorMessage}` },
      ]);
    }
  }, []);

  // NAPI-003: Select session and restore conversation
  const handleResumeSelect = useCallback(async () => {
    if (
      availableSessions.length === 0 ||
      resumeSessionIndex >= availableSessions.length
    ) {
      return;
    }

    const selectedSession = availableSessions[resumeSessionIndex];

    try {
      const {
        persistenceGetSessionMessages,
        persistenceGetSessionMessageEnvelopes,
      } = await import('@sengac/codelet-napi');
      const messages = persistenceGetSessionMessages(selectedSession.id);

      // Get FULL envelopes with all content blocks (ToolUse, ToolResult, Text, etc.)
      const envelopes: string[] = persistenceGetSessionMessageEnvelopes(
        selectedSession.id
      );

      // Convert full envelopes to conversation format for UI display
      // This properly restores tool calls, tool results, thinking, etc.
      //
      // CRITICAL: Tool results are stored in separate user envelopes after assistant
      // messages, but we need to interleave them correctly by matching tool_use_id.
      const restored: ConversationMessage[] = [];

      // First pass: collect all tool results by their tool_use_id
      const toolResultsByUseId = new Map<
        string,
        { content: string; isError: boolean }
      >();
      for (const envelopeJson of envelopes) {
        try {
          const envelope = JSON.parse(envelopeJson);
          const messageType =
            envelope.type || envelope.message_type || envelope.messageType;
          const message = envelope.message;
          if (!message) continue;

          if (messageType === 'user') {
            const contents = message.content || [];
            for (const content of contents) {
              if (content.type === 'tool_result' && content.tool_use_id) {
                toolResultsByUseId.set(content.tool_use_id, {
                  content: content.content || '',
                  isError: content.is_error || false,
                });
              }
            }
          }
        } catch {
          // Skip malformed envelopes in first pass
        }
      }

      // Second pass: process envelopes and interleave tool results
      for (const envelopeJson of envelopes) {
        try {
          const envelope = JSON.parse(envelopeJson);
          const messageType =
            envelope.type || envelope.message_type || envelope.messageType;
          const message = envelope.message;

          if (!message) continue;

          if (messageType === 'user') {
            // User messages - extract text only (tool results handled via interleaving)
            const contents = message.content || [];
            for (const content of contents) {
              if (content.type === 'text' && content.text) {
                restored.push({
                  role: 'user',
                  content: `${content.text}`,
                  isStreaming: false,
                });
              }
              // Skip tool_result here - they're interleaved with tool_use below
            }
          } else if (messageType === 'assistant') {
            // Assistant messages - extract text, tool use, and thinking
            // Interleave tool results immediately after their corresponding tool_use
            const contents = message.content || [];
            let textContent = '';

            for (const content of contents) {
              if (content.type === 'text' && content.text) {
                textContent += content.text;
              } else if (content.type === 'tool_use') {
                // Flush accumulated text first
                if (textContent) {
                  restored.push({
                    role: 'assistant',
                    content: textContent,
                    isStreaming: false,
                  });
                  textContent = '';
                }
                // TUI-037: Tool call in Claude Code style: ● ToolName(args)
                const input = content.input;
                let argsDisplay = '';
                if (typeof input === 'object' && input !== null) {
                  const inputObj = input as Record<string, unknown>;
                  if (inputObj.command) {
                    argsDisplay = String(inputObj.command);
                  } else if (inputObj.file_path) {
                    argsDisplay = String(inputObj.file_path);
                  } else if (inputObj.pattern) {
                    argsDisplay = String(inputObj.pattern);
                  } else {
                    const entries = Object.entries(inputObj);
                    if (entries.length > 0) {
                      const [key, value] = entries[0];
                      argsDisplay =
                        typeof value === 'string'
                          ? value
                          : `${key}: ${JSON.stringify(value).slice(0, 50)}`;
                    }
                  }
                }
                const toolHeader = formatToolHeader(content.name, argsDisplay);

                // TUI-037 + TUI-038: Combine tool header with result as ONE message
                // Check if this is Edit/Write tool to regenerate diff formatting
                const toolResult = toolResultsByUseId.get(content.id);
                if (toolResult) {
                  let resultContent: string;
                  const toolNameLower = content.name?.toLowerCase() || '';
                  const inputObj =
                    typeof input === 'object' && input !== null
                      ? (input as Record<string, unknown>)
                      : {};

                  // TUI-038: Regenerate diff for Edit/Write tools on restore
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
                  } else if (
                    (toolNameLower === 'write' ||
                      toolNameLower === 'write_file') &&
                    typeof inputObj.content === 'string'
                  ) {
                    // Write tool - generate diff (all additions)
                    const diffLines = formatWriteDiff(inputObj.content);
                    resultContent = formatDiffForDisplay(diffLines);
                  } else {
                    // Normal tool - use collapsed output
                    const sanitizedContent = toolResult.content.replace(
                      /\t/g,
                      '  '
                    );
                    resultContent = formatCollapsedOutput(sanitizedContent);
                  }

                  restored.push({
                    role: 'tool',
                    content: `${toolHeader}\n${resultContent}`,
                    isStreaming: false,
                  });
                } else {
                  // No result yet, just show header
                  restored.push({
                    role: 'tool',
                    content: toolHeader,
                    isStreaming: false,
                  });
                }
              } else if (content.type === 'thinking' && content.thinking) {
                // Thinking block (could show or hide based on preference)
                // For now, skip thinking blocks in restore (like Claude Code does)
              }
            }

            // Flush remaining text
            if (textContent) {
              restored.push({
                role: 'assistant',
                content: textContent,
                isStreaming: false,
              });
            }
          }
        } catch {
          // If envelope parsing fails, fall back to simple format
          logger.warn(
            'Failed to parse envelope, falling back to simple format'
          );
        }
      }

      // If envelope parsing yielded nothing, fall back to simple messages
      if (restored.length === 0) {
        for (const m of messages) {
          restored.push({
            role: m.role === 'user' ? 'user' : 'assistant',
            content: m.content,
            isStreaming: false,
          });
        }
      }

      // NAPI-008: Restore messages to CodeletSession for LLM context using full envelopes
      // This preserves structured tool_use/tool_result blocks (not just text summaries)
      // and rebuilds turn boundaries for proper compaction after restore
      if (sessionRef.current) {
        sessionRef.current.restoreMessagesFromEnvelopes(envelopes);

        // TUI-034: Handle full model path (provider/model-id) or legacy provider-only format
        if (selectedSession.provider) {
          const storedProvider = selectedSession.provider;

          if (storedProvider.includes('/')) {
            // Full model path format: "anthropic/claude-sonnet-4"
            try {
              await sessionRef.current.selectModel(storedProvider);
              // Parse model info from stored path
              const [providerId, modelId] = storedProvider.split('/');
              const internalName = mapProviderIdToInternal(providerId);
              setCurrentProvider(internalName);
              // Find matching model info from provider sections
              const section = providerSections.find(
                s => s.providerId === providerId
              );
              const model = section?.models.find(
                m => extractModelIdForRegistry(m.id) === modelId
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
                });
              }
            } catch (modelErr) {
              logger.warn(
                `Failed to select model ${storedProvider}: ${modelErr instanceof Error ? modelErr.message : String(modelErr)}`
              );
              // Fallback: try switching provider only
              const [providerId, modelId] = storedProvider.split('/');
              const internalName = mapProviderIdToInternal(providerId);
              try {
                await sessionRef.current.switchProvider(internalName);
                setCurrentProvider(internalName);
                // TUI-034: Show informational message about deprecated model fallback
                restored.push({
                  role: 'tool',
                  content: `Note: Model "${modelId}" is no longer available. Using provider default for ${internalName}.`,
                });
              } catch {
                // Provider switch also failed - continue with current
                restored.push({
                  role: 'tool',
                  content: `Note: Model "${storedProvider}" is no longer available and provider switch failed. Using current provider.`,
                });
              }
            }
          } else if (
            storedProvider !== sessionRef.current.currentProviderName
          ) {
            // Legacy provider-only format
            try {
              await sessionRef.current.switchProvider(storedProvider);
              setCurrentProvider(storedProvider);
            } catch (providerErr) {
              // Provider switch failed - continue with current provider
              logger.warn(
                `Failed to switch to session provider ${storedProvider}: ${providerErr instanceof Error ? providerErr.message : String(providerErr)}`
              );
            }
          }
        }

        // Restore token state from persisted session (including cache tokens for TUI-033)
        // CTX-003: Restore current context, output, cache tokens, and cumulative billing fields
        if (selectedSession.tokenUsage) {
          sessionRef.current.restoreTokenState(
            selectedSession.tokenUsage.currentContextTokens, // current input context
            selectedSession.tokenUsage.cumulativeBilledOutput, // output tokens (use cumulative as we don't store current separately)
            selectedSession.tokenUsage.cacheReadTokens ?? 0, // cache read
            selectedSession.tokenUsage.cacheCreationTokens ?? 0, // cache creation
            selectedSession.tokenUsage.cumulativeBilledInput ?? 0, // cumulative billed input
            selectedSession.tokenUsage.cumulativeBilledOutput ?? 0 // cumulative billed output
          );
        }

        // Update context fill percentage after restoring messages and tokens
        const contextFillInfo = sessionRef.current.getContextFillInfo();
        setContextFillPercentage(contextFillInfo.fillPercentage);
      }

      // Update state - replace current conversation entirely
      setCurrentSessionId(selectedSession.id);
      setConversation(restored);
      setIsResumeMode(false);
      setAvailableSessions([]);
      setResumeSessionIndex(0);
      // Don't rename resumed sessions with their first new message
      isFirstMessageRef.current = false;

      // Restore token usage from session manifest (including cache tokens)
      // CTX-003: Use currentContextTokens for display, cumulativeBilledOutput for output
      if (selectedSession.tokenUsage) {
        setTokenUsage({
          inputTokens: selectedSession.tokenUsage.currentContextTokens,
          outputTokens: selectedSession.tokenUsage.cumulativeBilledOutput,
          cacheReadInputTokens: selectedSession.tokenUsage.cacheReadTokens,
          cacheCreationInputTokens:
            selectedSession.tokenUsage.cacheCreationTokens,
        });
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to restore session';
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: `Resume failed: ${errorMessage}` },
      ]);
      setIsResumeMode(false);
      setAvailableSessions([]);
      setResumeSessionIndex(0);
    }
  }, [availableSessions, resumeSessionIndex]);

  // NAPI-003: Cancel resume mode
  const handleResumeCancel = useCallback(() => {
    setIsResumeMode(false);
    setAvailableSessions([]);
    setResumeSessionIndex(0);
  }, []);

  // Mouse scroll acceleration state (like VirtualList)
  const modelSelectorLastScrollTime = useRef<number>(0);
  const modelSelectorScrollVelocity = useRef<number>(1);
  const settingsLastScrollTime = useRef<number>(0);
  const settingsScrollVelocity = useRef<number>(1);

  // Mouse scroll navigation helper for model selector (navigates through filtered flat list)
  const navigateModelSelectorByDelta = useCallback(
    (delta: number) => {
      if (filteredFlatModelItems.length === 0) return;

      // Acceleration: scroll faster when scrolling rapidly
      const now = Date.now();
      const timeDelta = now - modelSelectorLastScrollTime.current;
      if (timeDelta < 150) {
        modelSelectorScrollVelocity.current = Math.min(
          modelSelectorScrollVelocity.current + 1,
          5
        );
      } else {
        modelSelectorScrollVelocity.current = 1;
      }
      modelSelectorLastScrollTime.current = now;
      const scrollAmount = modelSelectorScrollVelocity.current * delta;

      const currentFlatIdx = sectionModelToFlatIndex(
        selectedSectionIdx,
        selectedModelIdx,
        filteredFlatModelItems
      );
      const newFlatIdx = Math.max(
        0,
        Math.min(
          filteredFlatModelItems.length - 1,
          currentFlatIdx + scrollAmount
        )
      );
      const { sectionIdx, modelIdx } = flatIndexToSectionModel(
        newFlatIdx,
        filteredFlatModelItems
      );
      setSelectedSectionIdx(sectionIdx);
      setSelectedModelIdx(modelIdx);
    },
    [filteredFlatModelItems, selectedSectionIdx, selectedModelIdx]
  );

  // Mouse scroll navigation helper for settings tab (navigates through filtered list)
  const navigateSettingsByDelta = useCallback(
    (delta: number) => {
      // Acceleration: scroll faster when scrolling rapidly
      const now = Date.now();
      const timeDelta = now - settingsLastScrollTime.current;
      if (timeDelta < 150) {
        settingsScrollVelocity.current = Math.min(
          settingsScrollVelocity.current + 1,
          5
        );
      } else {
        settingsScrollVelocity.current = 1;
      }
      settingsLastScrollTime.current = now;
      const scrollAmount = settingsScrollVelocity.current * delta;

      setSelectedSettingsIdx(prev =>
        Math.max(
          0,
          Math.min(filteredSettingsProviders.length - 1, prev + scrollAmount)
        )
      );
    },
    [filteredSettingsProviders.length]
  );

  // Handle keyboard input
  useInput(
    (input, key) => {
      // Handle mouse scroll for model selector and settings tab
      // Mouse scroll moves the SELECTION (like VirtualList item mode), scroll offset auto-adjusts
      if (input.startsWith('[M') || key.mouse) {
        // Parse raw mouse escape sequences for scroll wheel
        if (input.startsWith('[M')) {
          const buttonByte = input.charCodeAt(2);
          // Button codes: 96 = scroll up, 97 = scroll down (xterm encoding)
          if (showModelSelector) {
            if (buttonByte === 96) {
              navigateModelSelectorByDelta(-1);
              return;
            } else if (buttonByte === 97) {
              navigateModelSelectorByDelta(1);
              return;
            }
          }
          if (showSettingsTab) {
            if (buttonByte === 96) {
              navigateSettingsByDelta(-1);
              return;
            } else if (buttonByte === 97) {
              navigateSettingsByDelta(1);
              return;
            }
          }
        }
        // Handle parsed mouse events from Ink
        if (key.mouse) {
          if (showModelSelector) {
            if (key.mouse.button === 'wheelUp') {
              navigateModelSelectorByDelta(-1);
              return;
            } else if (key.mouse.button === 'wheelDown') {
              navigateModelSelectorByDelta(1);
              return;
            }
          }
          if (showSettingsTab) {
            if (key.mouse.button === 'wheelUp') {
              navigateSettingsByDelta(-1);
              return;
            } else if (key.mouse.button === 'wheelDown') {
              navigateSettingsByDelta(1);
              return;
            }
          }
        }
        // Skip other mouse events (handled by VirtualList elsewhere)
        return;
      }

      // NAPI-006: Search mode keyboard handling
      if (isSearchMode) {
        if (key.escape) {
          handleSearchCancel();
          return;
        }
        if (key.return) {
          handleSearchSelect();
          return;
        }
        if (key.upArrow) {
          setSearchResultIndex(prev => Math.max(0, prev - 1));
          return;
        }
        if (key.downArrow) {
          setSearchResultIndex(prev =>
            Math.min(searchResults.length - 1, prev + 1)
          );
          return;
        }
        if (key.backspace || key.delete) {
          void handleSearchInput(searchQuery.slice(0, -1));
          return;
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
        return;
      }

      // NAPI-003: Resume mode keyboard handling
      if (isResumeMode) {
        if (key.escape) {
          handleResumeCancel();
          return;
        }
        if (key.return) {
          void handleResumeSelect();
          return;
        }
        if (key.upArrow) {
          setResumeSessionIndex(prev => Math.max(0, prev - 1));
          return;
        }
        if (key.downArrow) {
          setResumeSessionIndex(prev =>
            Math.min(availableSessions.length - 1, prev + 1)
          );
          return;
        }
        // No text input in resume mode - just navigation
        return;
      }

      if (showProviderSelector) {
        if (key.escape) {
          setShowProviderSelector(false);
          return;
        }
        if (key.upArrow) {
          setSelectedProviderIndex(prev =>
            prev > 0 ? prev - 1 : availableProviders.length - 1
          );
          return;
        }
        if (key.downArrow) {
          setSelectedProviderIndex(prev =>
            prev < availableProviders.length - 1 ? prev + 1 : 0
          );
          return;
        }
        if (key.return) {
          void handleSwitchProvider(availableProviders[selectedProviderIndex]);
          return;
        }
        return;
      }

      // TUI-034: Model selector keyboard handling
      if (showModelSelector) {
        // Filter mode handling
        if (isModelSelectorFilterMode) {
          if (key.escape) {
            setIsModelSelectorFilterMode(false);
            setModelSelectorFilter('');
            return;
          }
          if (key.return) {
            setIsModelSelectorFilterMode(false);
            return;
          }
          if (key.backspace || key.delete) {
            setModelSelectorFilter(prev => prev.slice(0, -1));
            return;
          }
          // Accept printable characters for filter
          const clean = input
            .split('')
            .filter(ch => {
              const code = ch.charCodeAt(0);
              return code >= 32 && code <= 126;
            })
            .join('');
          if (clean) {
            setModelSelectorFilter(prev => prev + clean);
          }
          return;
        }

        if (key.escape) {
          if (modelSelectorFilter) {
            setModelSelectorFilter('');
            return;
          }
          setShowModelSelector(false);
          return;
        }

        // '/' to enter filter mode
        if (input === '/') {
          setIsModelSelectorFilterMode(true);
          return;
        }

        // Left arrow: collapse current section
        if (key.leftArrow) {
          const currentSection = providerSections[selectedSectionIdx];
          if (
            currentSection &&
            expandedProviders.has(currentSection.providerId)
          ) {
            setExpandedProviders(prev => {
              const next = new Set(prev);
              next.delete(currentSection.providerId);
              return next;
            });
            setSelectedModelIdx(-1); // Move back to section header
          }
          return;
        }

        // Right arrow: expand current section
        if (key.rightArrow) {
          const currentSection = providerSections[selectedSectionIdx];
          if (
            currentSection &&
            !expandedProviders.has(currentSection.providerId)
          ) {
            setExpandedProviders(
              prev => new Set([...prev, currentSection.providerId])
            );
          }
          return;
        }

        // Up arrow: navigate up through models and sections
        if (key.upArrow) {
          const currentSection = providerSections[selectedSectionIdx];
          const isExpanded =
            currentSection && expandedProviders.has(currentSection.providerId);

          if (selectedModelIdx > 0 && isExpanded) {
            // Move up within models
            setSelectedModelIdx(prev => prev - 1);
          } else if (selectedModelIdx === 0 && isExpanded) {
            // Move from first model to section header
            setSelectedModelIdx(-1);
          } else if (selectedSectionIdx > 0) {
            // Move to previous section
            const prevSection = providerSections[selectedSectionIdx - 1];
            const prevExpanded = expandedProviders.has(prevSection.providerId);
            setSelectedSectionIdx(prev => prev - 1);
            if (prevExpanded && prevSection.models.length > 0) {
              // Move to last model of previous section
              setSelectedModelIdx(prevSection.models.length - 1);
            } else {
              setSelectedModelIdx(-1);
            }
          }
          return;
        }

        // Down arrow: navigate down through models and sections
        if (key.downArrow) {
          const currentSection = providerSections[selectedSectionIdx];
          const isExpanded =
            currentSection && expandedProviders.has(currentSection.providerId);
          const modelCount = currentSection?.models.length ?? 0;

          if (selectedModelIdx === -1 && isExpanded && modelCount > 0) {
            // Move from section header to first model
            setSelectedModelIdx(0);
          } else if (selectedModelIdx < modelCount - 1 && isExpanded) {
            // Move down within models
            setSelectedModelIdx(prev => prev + 1);
          } else if (selectedSectionIdx < providerSections.length - 1) {
            // Move to next section
            setSelectedSectionIdx(prev => prev + 1);
            setSelectedModelIdx(-1);
          }
          return;
        }

        // Enter: select model or toggle section
        if (key.return) {
          const currentSection = providerSections[selectedSectionIdx];
          if (selectedModelIdx === -1) {
            // On section header - toggle expand/collapse
            if (expandedProviders.has(currentSection.providerId)) {
              setExpandedProviders(prev => {
                const next = new Set(prev);
                next.delete(currentSection.providerId);
                return next;
              });
            } else {
              setExpandedProviders(
                prev => new Set([...prev, currentSection.providerId])
              );
            }
          } else {
            // On model - select it
            const model = currentSection.models[selectedModelIdx];
            if (model) {
              void handleSelectModel(currentSection, model);
            }
          }
          return;
        }

        // CONFIG-004: 'r' to refresh models from models.dev
        if ((input === 'r' || input === 'R') && !isRefreshingModels) {
          void refreshModels();
          return;
        }

        // CONFIG-004: Tab to switch to Settings view
        if (key.tab) {
          setShowModelSelector(false);
          setShowSettingsTab(true);
          setSelectedSettingsIdx(0);
          setEditingProviderId(null);
          setEditingApiKey('');
          void loadProviderStatuses();
          return;
        }
        return;
      }

      // CONFIG-004: Settings tab keyboard handling
      if (showSettingsTab) {
        // Filter mode handling
        if (isSettingsFilterMode) {
          if (key.escape) {
            setIsSettingsFilterMode(false);
            setSettingsFilter('');
            return;
          }
          if (key.return) {
            setIsSettingsFilterMode(false);
            return;
          }
          if (key.backspace || key.delete) {
            setSettingsFilter(prev => prev.slice(0, -1));
            return;
          }
          // Accept printable characters for filter
          const clean = input
            .split('')
            .filter(ch => {
              const code = ch.charCodeAt(0);
              return code >= 32 && code <= 126;
            })
            .join('');
          if (clean) {
            setSettingsFilter(prev => prev + clean);
          }
          return;
        }

        if (key.escape) {
          if (settingsFilter) {
            setSettingsFilter('');
            return;
          }
          if (editingProviderId) {
            // Cancel editing
            setEditingProviderId(null);
            setEditingApiKey('');
          } else {
            setShowSettingsTab(false);
          }
          return;
        }

        // '/' to enter filter mode (when not editing)
        if (input === '/' && !editingProviderId) {
          setIsSettingsFilterMode(true);
          return;
        }

        // Tab to switch back to Model selector
        if (key.tab && !editingProviderId) {
          setShowSettingsTab(false);
          setShowModelSelector(true);
          return;
        }

        // Up/Down arrow to navigate providers (when not editing)
        if (!editingProviderId) {
          if (key.upArrow && selectedSettingsIdx > 0) {
            setSelectedSettingsIdx(prev => prev - 1);
            setConnectionTestResult(null);
            return;
          }
          if (
            key.downArrow &&
            selectedSettingsIdx < filteredSettingsProviders.length - 1
          ) {
            setSelectedSettingsIdx(prev => prev + 1);
            setConnectionTestResult(null);
            return;
          }
        }

        // Enter to start editing or save
        if (key.return) {
          if (editingProviderId) {
            // Save the key
            if (editingApiKey.trim()) {
              void handleSaveApiKey(editingProviderId, editingApiKey.trim());
            } else {
              // Cancel if empty
              setEditingProviderId(null);
              setEditingApiKey('');
            }
          } else {
            // Start editing the selected provider
            const providerId = filteredSettingsProviders[selectedSettingsIdx];
            setEditingProviderId(providerId);
            setEditingApiKey('');
          }
          return;
        }

        // 't' to test connection (when not editing)
        if (!editingProviderId && (input === 't' || input === 'T')) {
          const providerId = filteredSettingsProviders[selectedSettingsIdx];
          void handleTestConnection(providerId);
          return;
        }

        // 'd' to delete API key (when not editing)
        if (!editingProviderId && (input === 'd' || input === 'D')) {
          const providerId = filteredSettingsProviders[selectedSettingsIdx];
          if (providerStatuses[providerId]?.hasKey) {
            void handleDeleteApiKey(providerId);
          }
          return;
        }

        // Handle text input when editing
        if (editingProviderId) {
          if (key.backspace || key.delete) {
            setEditingApiKey(prev => prev.slice(0, -1));
            return;
          }
          // Filter to printable characters
          const clean = input
            .split('')
            .filter(ch => {
              const code = ch.charCodeAt(0);
              return code >= 32 && code <= 126;
            })
            .join('');
          if (clean) {
            setEditingApiKey(prev => prev + clean);
          }
          return;
        }
        return;
      }

      // Esc key handling - interrupt if loading, exit if not
      if (key.escape) {
        if (isLoading && sessionRef.current) {
          // Interrupt the agent execution
          sessionRef.current.interrupt();
        } else {
          // Exit the agent view
          onExit();
        }
        return;
      }

      // TUI-034: Tab to toggle model selector (replaces provider-only selector)
      if (key.tab && providerSections.length > 0) {
        setShowModelSelector(true);
        // Find current section and expand it
        const currentSectionIdx = providerSections.findIndex(
          s => s.providerId === currentModel?.providerId
        );
        setSelectedSectionIdx(currentSectionIdx >= 0 ? currentSectionIdx : 0);
        setSelectedModelIdx(-1); // Start on section header
        // Expand current provider's section
        if (currentModel?.providerId) {
          setExpandedProviders(new Set([currentModel.providerId]));
        }
        return;
      }
    },
    { isActive: true }
  );

  // PERF-002: Helper function to wrap a single message into lines
  // Extracted to be reusable for incremental caching
  const wrapMessageToLines = (
    msg: ConversationMessage,
    msgIndex: number,
    maxWidth: number
  ): ConversationLine[] => {
    const lines: ConversationLine[] = [];
    // Add role prefix to first line
    const prefix =
      msg.role === 'user' ? 'You: ' : msg.role === 'assistant' ? '● ' : '';
    // Normalize emoji variation selectors for consistent width calculation
    const normalizedContent = normalizeEmojiWidth(msg.content);
    const contentLines = normalizedContent.split('\n');

    contentLines.forEach((lineContent, lineIndex) => {
      let displayContent =
        lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;
      // Add streaming indicator to last line of streaming message
      const isLastLine = lineIndex === contentLines.length - 1;
      if (msg.isStreaming && isLastLine) {
        displayContent += '...';
      }

      // Wrap long lines manually to fit terminal width (using visual width for Unicode)
      if (getVisualWidth(displayContent) === 0) {
        lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });
      } else {
        // Split into words, keeping whitespace
        const words = displayContent.split(/(\s+)/);
        let currentLine = '';
        let currentWidth = 0;

        for (const word of words) {
          const wordWidth = getVisualWidth(word);

          if (wordWidth === 0) continue;

          // If word alone exceeds max width, force break it character by character
          if (wordWidth > maxWidth) {
            // Flush current line first
            if (currentLine) {
              lines.push({
                role: msg.role,
                content: currentLine,
                messageIndex: msgIndex,
              });
              currentLine = '';
              currentWidth = 0;
            }
            // Break long word by visual width
            let chunk = '';
            let chunkWidth = 0;
            for (const char of word) {
              const charWidth = getVisualWidth(char);
              if (chunkWidth + charWidth > maxWidth && chunk) {
                lines.push({
                  role: msg.role,
                  content: chunk,
                  messageIndex: msgIndex,
                });
                chunk = char;
                chunkWidth = charWidth;
              } else {
                chunk += char;
                chunkWidth += charWidth;
              }
            }
            if (chunk) {
              currentLine = chunk;
              currentWidth = chunkWidth;
            }
            continue;
          }

          // Check if word fits on current line
          if (currentWidth + wordWidth > maxWidth) {
            // Flush current line and start new one
            if (currentLine.trim()) {
              lines.push({
                role: msg.role,
                content: currentLine.trimEnd(),
                messageIndex: msgIndex,
              });
            }
            // Don't start line with whitespace
            currentLine = word.trim() ? word : '';
            currentWidth = word.trim() ? wordWidth : 0;
          } else {
            currentLine += word;
            currentWidth += wordWidth;
          }
        }

        // Flush remaining content
        if (currentLine.trim()) {
          lines.push({
            role: msg.role,
            content: currentLine.trimEnd(),
            messageIndex: msgIndex,
          });
        } else if (lines.length === 0) {
          // Ensure at least one line per content section
          lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });
        }
      }
    });

    // Add empty line after each message for spacing (use space to ensure line renders)
    lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });

    return lines;
  };

  // PERF-002: Incremental line computation with caching
  // Only recompute lines for messages that changed, reuse cached lines for unchanged messages
  // PERF-003: Uses deferredConversation to prioritize user input over streaming updates
  const conversationLines = useMemo((): ConversationLine[] => {
    const maxWidth = terminalWidth - 6; // Account for borders and padding
    const lines: ConversationLine[] = [];
    const cache = lineCacheRef.current;

    // Track which message indices are still valid
    const validIndices = new Set<number>();

    deferredConversation.forEach((msg, msgIndex) => {
      validIndices.add(msgIndex);

      // Check cache for this message
      const cached = cache.get(msgIndex);
      if (
        cached &&
        cached.content === msg.content &&
        cached.isStreaming === msg.isStreaming &&
        cached.terminalWidth === terminalWidth
      ) {
        // Cache hit - reuse cached lines
        lines.push(...cached.lines);
      } else {
        // Cache miss - compute lines and cache them
        const messageLines = wrapMessageToLines(msg, msgIndex, maxWidth);
        cache.set(msgIndex, {
          content: msg.content,
          isStreaming: msg.isStreaming ?? false,
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

  // Error state - show setup instructions (full-screen overlay)
  if (error && !session) {
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
              <Text color="cyan"> OPENAI_API_KEY</Text>
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
              <Box key={provider}>
                <Text
                  backgroundColor={
                    idx === selectedProviderIndex ? 'cyan' : undefined
                  }
                  color={idx === selectedProviderIndex ? 'black' : 'white'}
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

  // TUI-034: Model selector overlay (hierarchical with collapsible sections)
  if (showModelSelector) {
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
          <Box flexDirection="column" padding={2} flexGrow={1}>
            <Box marginBottom={1}>
              <Text bold color="cyan">
                Select Model
              </Text>
              {isRefreshingModels && (
                <Text color="yellow"> (refreshing...)</Text>
              )}
              <Text dimColor>
                {' '}
                ({filteredFlatModelItems.length} items, showing{' '}
                {modelSelectorScrollOffset + 1}-
                {Math.min(
                  modelSelectorScrollOffset + modelSelectorVisibleHeight,
                  filteredFlatModelItems.length
                )}
                )
              </Text>
            </Box>
            {/* Filter input box */}
            {(isModelSelectorFilterMode || modelSelectorFilter) && (
              <Box marginBottom={1}>
                <Text color="yellow">Filter: </Text>
                <Text>{modelSelectorFilter}</Text>
                {isModelSelectorFilterMode && <Text inverse> </Text>}
              </Box>
            )}
            {/* Scrollable list with viewport */}
            <Box flexDirection="row" flexGrow={1}>
              <Box flexDirection="column" flexGrow={1}>
                {filteredFlatModelItems
                  .slice(
                    modelSelectorScrollOffset,
                    modelSelectorScrollOffset + modelSelectorVisibleHeight
                  )
                  .map(item => {
                    if (item.type === 'section') {
                      const isSectionSelected =
                        item.sectionIdx === selectedSectionIdx &&
                        selectedModelIdx === -1;
                      const sectionIcon = item.isExpanded ? '▼' : '▶';
                      return (
                        <Box key={`section-${item.section.providerId}`}>
                          <Text
                            backgroundColor={
                              isSectionSelected ? 'cyan' : undefined
                            }
                            color={isSectionSelected ? 'black' : 'white'}
                          >
                            {isSectionSelected ? '> ' : '  '}
                            {sectionIcon} [{item.section.providerId}] (
                            {item.section.models.length} models)
                          </Text>
                        </Box>
                      );
                    } else {
                      const isModelSelected =
                        item.sectionIdx === selectedSectionIdx &&
                        item.modelIdx === selectedModelIdx;
                      const isCurrent =
                        currentModel?.apiModelId === item.model.id;
                      const modelId = extractModelIdForRegistry(item.model.id);
                      return (
                        <Box key={`model-${item.model.id}`}>
                          <Text
                            backgroundColor={
                              isModelSelected ? 'cyan' : undefined
                            }
                            color={isModelSelected ? 'black' : 'white'}
                          >
                            {isModelSelected ? '  > ' : '    '}
                            {modelId} ({item.model.name})
                            {item.model.reasoning && (
                              <Text
                                color={isModelSelected ? 'black' : 'magenta'}
                              >
                                {' '}
                                [R]
                              </Text>
                            )}
                            {item.model.hasVision && (
                              <Text color={isModelSelected ? 'black' : 'blue'}>
                                {' '}
                                [V]
                              </Text>
                            )}
                            <Text color={isModelSelected ? 'black' : 'gray'}>
                              {' '}
                              [{formatContextWindow(item.model.contextWindow)}]
                            </Text>
                            {isCurrent && (
                              <Text color={isModelSelected ? 'black' : 'green'}>
                                {' '}
                                (current)
                              </Text>
                            )}
                          </Text>
                        </Box>
                      );
                    }
                  })}
              </Box>
              {/* Scrollbar */}
              {filteredFlatModelItems.length > modelSelectorVisibleHeight && (
                <Box flexDirection="column" marginLeft={1}>
                  {Array.from({ length: modelSelectorVisibleHeight }).map(
                    (_, i) => {
                      const thumbHeight = Math.max(
                        1,
                        Math.floor(
                          (modelSelectorVisibleHeight /
                            filteredFlatModelItems.length) *
                            modelSelectorVisibleHeight
                        )
                      );
                      const thumbPos = Math.floor(
                        (modelSelectorScrollOffset /
                          filteredFlatModelItems.length) *
                          modelSelectorVisibleHeight
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
                Enter Select | ←→ Expand/Collapse | ↑↓ Navigate | / Filter | r
                Refresh | Tab Settings | Esc Cancel
              </Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  // CONFIG-004: Settings tab overlay (provider API key management)
  if (showSettingsTab) {
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
          borderColor="yellow"
          backgroundColor="black"
        >
          <Box flexDirection="column" padding={2} flexGrow={1}>
            <Box marginBottom={1}>
              <Text bold color="yellow">
                Provider Settings
              </Text>
              <Text dimColor>
                {' '}
                ({filteredSettingsProviders.length} providers, showing{' '}
                {settingsScrollOffset + 1}-
                {Math.min(
                  settingsScrollOffset + settingsVisibleHeight,
                  filteredSettingsProviders.length
                )}
                )
              </Text>
            </Box>
            {/* Filter input box */}
            {(isSettingsFilterMode || settingsFilter) && (
              <Box marginBottom={1}>
                <Text color="yellow">Filter: </Text>
                <Text>{settingsFilter}</Text>
                {isSettingsFilterMode && <Text inverse> </Text>}
              </Box>
            )}

            {/* Scrollable list of providers with viewport */}
            <Box flexDirection="row" flexGrow={1}>
              <Box flexDirection="column" flexGrow={1}>
                {filteredSettingsProviders
                  .slice(
                    settingsScrollOffset,
                    settingsScrollOffset + settingsVisibleHeight
                  )
                  .map((providerId, visibleIdx) => {
                    const actualIdx = settingsScrollOffset + visibleIdx;
                    const isSelected = actualIdx === selectedSettingsIdx;
                    const status = providerStatuses[providerId];
                    const registryEntry = getProviderRegistryEntry(providerId);
                    const isEditing = editingProviderId === providerId;
                    const testResult =
                      connectionTestResult?.providerId === providerId
                        ? connectionTestResult
                        : null;

                    return (
                      <Box
                        key={providerId}
                        flexDirection="column"
                        marginBottom={0}
                      >
                        <Box>
                          <Text
                            backgroundColor={
                              isSelected && !isEditing ? 'yellow' : undefined
                            }
                            color={isSelected && !isEditing ? 'black' : 'white'}
                          >
                            {isSelected ? '> ' : '  '}
                            {registryEntry?.name || providerId}
                          </Text>
                          {status?.hasKey ? (
                            <Text color="green"> ✓ {status.maskedKey}</Text>
                          ) : (
                            <Text color="gray"> (not configured)</Text>
                          )}
                          {testResult && (
                            <Text color={testResult.success ? 'green' : 'red'}>
                              {' '}
                              {testResult.message}
                            </Text>
                          )}
                        </Box>

                        {/* Editing input */}
                        {isEditing && (
                          <Box marginLeft={4}>
                            <Text color="yellow">API Key: </Text>
                            <Text>
                              {editingApiKey
                                ? '•'.repeat(editingApiKey.length)
                                : ''}
                              <Text inverse> </Text>
                            </Text>
                          </Box>
                        )}
                      </Box>
                    );
                  })}
              </Box>
              {/* Scrollbar */}
              {filteredSettingsProviders.length > settingsVisibleHeight && (
                <Box flexDirection="column" marginLeft={1}>
                  {Array.from({ length: settingsVisibleHeight }).map((_, i) => {
                    const thumbHeight = Math.max(
                      1,
                      Math.floor(
                        (settingsVisibleHeight /
                          filteredSettingsProviders.length) *
                          settingsVisibleHeight
                      )
                    );
                    const thumbPos = Math.floor(
                      (settingsScrollOffset /
                        filteredSettingsProviders.length) *
                        settingsVisibleHeight
                    );
                    const isThumb = i >= thumbPos && i < thumbPos + thumbHeight;
                    return (
                      <Text key={i} dimColor>
                        {isThumb ? '■' : '│'}
                      </Text>
                    );
                  })}
                </Box>
              )}
            </Box>

            <Box marginTop={1}>
              <Text dimColor>
                {editingProviderId
                  ? 'Type API key | Enter Save | Esc Cancel'
                  : 'Enter Edit | t Test | d Delete | / Filter | Tab Models | Esc Close'}
              </Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  // NAPI-006: Search mode overlay (Ctrl+R history search)
  if (isSearchMode) {
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
          borderColor="magenta"
          backgroundColor="black"
        >
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
              <Box key={`${entry.sessionId}-${entry.timestamp}`}>
                <Text
                  backgroundColor={
                    idx === searchResultIndex ? 'magenta' : undefined
                  }
                  color={idx === searchResultIndex ? 'black' : 'white'}
                >
                  {idx === searchResultIndex ? '> ' : '  '}
                  {entry.display.slice(0, terminalWidth - 10)}
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
        <Box
          flexDirection="column"
          flexGrow={1}
          borderStyle="double"
          borderColor="blue"
          backgroundColor="black"
        >
          <Box flexDirection="column" padding={2} flexGrow={1}>
            <Box marginBottom={1}>
              <Text bold color="blue">
                Resume Session ({availableSessions.length} available)
              </Text>
            </Box>
            {availableSessions.length === 0 && (
              <Box>
                <Text dimColor>No sessions found for this project</Text>
              </Box>
            )}
            {availableSessions.slice(0, 15).map((session, idx) => {
              const isSelected = idx === resumeSessionIndex;
              const updatedAt = new Date(session.updatedAt);
              const timeAgo = formatTimeAgo(updatedAt);
              const provider = session.provider || 'unknown';
              return (
                <Box key={session.id} flexDirection="column">
                  <Text
                    backgroundColor={isSelected ? 'blue' : undefined}
                    color={isSelected ? 'black' : 'white'}
                  >
                    {isSelected ? '> ' : '  '}
                    {session.name}
                  </Text>
                  <Text
                    backgroundColor={isSelected ? 'blue' : undefined}
                    color={isSelected ? 'black' : 'gray'}
                    dimColor={!isSelected}
                  >
                    {'    '}
                    {session.messageCount} messages | {provider} | {timeAgo}
                  </Text>
                </Box>
              );
            })}
            <Box marginTop={1}>
              <Text dimColor>Enter Select | Arrow Navigate | Esc Cancel</Text>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  }

  // Main agent view (full-screen)
  // Remove position="absolute" since FullScreenWrapper handles positioning
  // Removed outer border to maximize usable space and reduce rendering overhead
  return (
    <Box
      flexDirection="column"
      flexGrow={1}
    >
      {/* TUI-034: Header with model name, capability indicators, and token usage */}
      <Box
        borderStyle="single"
        borderBottom={true}
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        paddingX={1}
      >
        <Box flexGrow={1}>
          <Text bold color="cyan">
            Agent: {currentModel?.modelId || currentProvider}
          </Text>
          {/* TUI-034: Model capability indicators */}
          {currentModel?.reasoning && <Text color="magenta"> [R]</Text>}
          {currentModel?.hasVision && <Text color="blue"> [V]</Text>}
          {currentModel?.contextWindow && (
            <Text dimColor>
              {' '}
              [{formatContextWindow(currentModel.contextWindow)}]
            </Text>
          )}
          {isLoading && <Text color="yellow"> (streaming...)</Text>}
          {/* AGENT-021: DEBUG indicator when debug capture is enabled */}
          {isDebugEnabled && (
            <Text color="red" bold>
              {' '}
              [DEBUG]
            </Text>
          )}
          {/* TOOL-010: Thinking level indicator - only show while streaming */}
          {isLoading &&
            detectedThinkingLevel !== null &&
            detectedThinkingLevel !== JsThinkingLevel.Off && (
              <Text color="magenta" bold>
                {' '}
                {getThinkingLevelLabel(detectedThinkingLevel)}
              </Text>
            )}
        </Box>
        {/* TUI-031: Tokens per second display during streaming */}
        {isLoading && displayedTokPerSec !== null && (
          <Box marginRight={2}>
            <Text color="magenta">{displayedTokPerSec.toFixed(1)} tok/s</Text>
          </Box>
        )}
        <Box>
          <Text dimColor>
            tokens: {tokenUsage.inputTokens}↓ {tokenUsage.outputTokens}↑
          </Text>
        </Box>
        {/* TUI-033: Context window fill percentage indicator */}
        <Box marginLeft={2}>
          <Text color={getContextFillColor(contextFillPercentage)}>
            [{contextFillPercentage}%]
          </Text>
        </Box>
        {/* TUI-034: Tab to switch model */}
        {providerSections.length > 0 && (
          <Box marginLeft={2}>
            <Text dimColor>[Tab]</Text>
          </Box>
        )}
      </Box>

      {/* Conversation area using VirtualList for proper scrolling - matches FileDiffViewer pattern */}
      <Box flexGrow={1} flexBasis={0}>
        <VirtualList
          items={conversationLines}
          renderItem={line => {
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
            }

            // Default rendering for non-diff content
            // Tool output is white (not yellow), user input is green
            const color = line.role === 'user' ? 'green' : 'white';
            return (
              <Box flexGrow={1}>
                <Text color={color}>{content}</Text>
              </Box>
            );
          }}
          keyExtractor={(_line, index) => `line-${index}`}
          emptyMessage="Type a message to start..."
          showScrollbar={!isLoading}
          isFocused={!isLoading && !showProviderSelector}
          scrollToEnd={true}
          selectionMode="scroll"
        />
      </Box>

      {/* Input area */}
      <Box
        borderStyle="single"
        borderTop={true}
        borderBottom={false}
        borderLeft={false}
        borderRight={false}
        paddingX={1}
      >
        <Text color="green">&gt; </Text>
        <Box flexGrow={1}>
          {isLoading ? (
            <Text dimColor>Thinking... (Esc to stop)</Text>
          ) : (
            <SafeTextInput
              value={inputValue}
              onChange={setInputValue}
              onSubmit={handleSubmit}
              placeholder="Type your message... (Shift+↑↓ history)"
              onHistoryPrev={handleHistoryPrev}
              onHistoryNext={handleHistoryNext}
            />
          )}
        </Box>
      </Box>
    </Box>
  );
};

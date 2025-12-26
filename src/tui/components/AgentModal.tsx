/**
 * AgentModal - Full-screen modal for AI agent interactions
 *
 * Integrates codelet-napi native module into fspec's TUI to enable
 * AI-powered conversations within the terminal interface.
 *
 * Implements NAPI-003: Proper TUI Integration Using Existing Codelet Rust Infrastructure
 * - Uses the same streaming loop as codelet-cli (run_agent_stream)
 * - Supports Esc key interruption via session.interrupt()
 * - Full-screen modal for maximum conversation space
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
  useReducer,
} from 'react';
import { Box, Text, useInput, useStdout } from 'ink';
import { VirtualList } from './VirtualList';
import { getFspecUserDir } from '../../utils/config';
import { logger } from '../../utils/logger';
import { normalizeEmojiWidth, getVisualWidth } from '../utils/stringWidth';
import { persistenceStoreMessageEnvelope } from '@sengac/codelet-napi';

// NAPI-006: Callbacks for history navigation
interface SafeTextInputCallbacks {
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
}

// Custom TextInput that ignores mouse escape sequences
const SafeTextInput: React.FC<{
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
} & SafeTextInputCallbacks> = ({
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
        .filter((ch) => {
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
  toolCall?: { id: string; name: string; input: string };
  toolResult?: { toolCallId: string; content: string; isError: boolean };
  status?: string;
  queuedInputs?: string[];
  tokens?: TokenTracker;
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
    callback: (chunk: StreamChunk) => void
  ) => Promise<void>;
  switchProvider: (providerName: string) => Promise<void>;
  clearHistory: () => void;
  interrupt: () => void;
  resetInterrupt: () => void;
  toggleDebug: (debugDir?: string) => DebugCommandResult; // AGENT-021
  compact: () => Promise<CompactionResult>; // NAPI-005
}

export interface AgentModalProps {
  isOpen: boolean;
  onClose: () => void;
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

export const AgentModal: React.FC<AgentModalProps> = ({ isOpen, onClose }) => {
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
  const [availableSessions, setAvailableSessions] = useState<SessionManifest[]>([]);
  const [resumeSessionIndex, setResumeSessionIndex] = useState(0);

  // TUI-031: Tok/s display (calculated in Rust, just displayed here)
  const [displayedTokPerSec, setDisplayedTokPerSec] = useState<number | null>(null);
  const [lastChunkTime, setLastChunkTime] = useState<number | null>(null);

  // TUI-033: Context window fill percentage (received from Rust via ContextFillUpdate event)
  const [contextFillPercentage, setContextFillPercentage] = useState<number>(0);

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

  // TUI-031: Hide tok/s after 10 seconds of no chunks
  useEffect(() => {
    if (!isLoading || lastChunkTime === null) return;
    const timeout = setTimeout(() => {
      setDisplayedTokPerSec(null);
    }, 10000);
    return () => clearTimeout(timeout);
  }, [isLoading, lastChunkTime]);

  // Initialize session when modal opens
  useEffect(() => {
    if (!isOpen) {
      // Reset state when modal closes (fresh session each time)
      setSession(null);
      setConversation([]);
      setTokenUsage({ inputTokens: 0, outputTokens: 0 });
      setError(null);
      setInputValue('');
      setIsDebugEnabled(false); // AGENT-021: Reset debug state on modal close
      // TUI-031: Reset tok/s display
      setDisplayedTokPerSec(null);
      setLastChunkTime(null);
      sessionRef.current = null;
      // NAPI-006: Reset history and search state
      setHistoryEntries([]);
      setHistoryIndex(-1);
      setSavedInput('');
      setIsSearchMode(false);
      setSearchQuery('');
      setSearchResults([]);
      setSearchResultIndex(0);
      setCurrentSessionId(null);
      // NAPI-003: Reset resume mode state
      setIsResumeMode(false);
      setAvailableSessions([]);
      setResumeSessionIndex(0);
      return;
    }

    const initSession = async () => {
      try {
        // Dynamic import to handle ESM
        const codeletNapi = await import('@sengac/codelet-napi');
        const { CodeletSession, persistenceSetDataDirectory, persistenceGetHistory } = codeletNapi;

        // NAPI-006: Set up persistence data directory
        const fspecDir = getFspecUserDir();
        try {
          persistenceSetDataDirectory(fspecDir);
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

        // Default to Claude as the primary AI provider
        const newSession = new CodeletSession('claude');
        setSession(newSession);
        sessionRef.current = newSession;
        setCurrentProvider(newSession.currentProviderName);
        setAvailableProviders(newSession.availableProviders);
        setTokenUsage(newSession.tokenTracker);

        // NAPI-006: Session creation is deferred until first message is sent
        // This prevents empty sessions from being persisted when user opens
        // the modal but doesn't send any messages. See handleSubmit() for
        // the actual session creation logic.

        // NAPI-006: Load history for current project
        try {

          const history = persistenceGetHistory(currentProjectRef.current, 100);

          // Convert NAPI history entries (camelCase from NAPI-RS) to our interface
          const entries: HistoryEntry[] = history.map((h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({
            display: h.display,
            timestamp: h.timestamp,
            project: h.project,
            sessionId: h.sessionId,
            hasPastedContent: h.hasPastedContent ?? false,
          }));
          setHistoryEntries(entries);
        } catch (err) {
          logger.error(`Failed to load history: ${err instanceof Error ? err.message : String(err)}`);
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
  }, [isOpen]);

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
        const errorMessage = err instanceof Error ? err.message : 'Failed to toggle debug mode';
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
        const errorMessage = err instanceof Error ? err.message : 'Failed to clear session';
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
        const history = persistenceGetHistory(allProjects ? null : currentProjectRef.current, 20);
        if (history.length === 0) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: 'No history entries found' },
          ]);
        } else {
          const historyList = history.map((h: { display: string; timestamp: string }) =>
            `- ${h.display}`
          ).join('\n');
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Command history:\n${historyList}` },
          ]);
        }
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to get history';
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
        const { persistenceListSessions } = await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        if (sessions.length === 0) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: 'No sessions found for this project' },
          ]);
        } else {
          const sessionList = sessions.map((s: SessionManifest) =>
            `- ${s.name} (${s.messageCount} messages, ${s.id.slice(0, 8)}...)`
          ).join('\n');
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Sessions:\n${sessionList}` },
          ]);
        }
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to list sessions';
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
        const { persistenceListSessions, persistenceLoadSession } = await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        const target = sessions.find((s: SessionManifest) => s.name === targetName);
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
        const errorMessage = err instanceof Error ? err.message : 'Failed to switch session';
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
        const { persistenceRenameSession } = await import('@sengac/codelet-napi');
        persistenceRenameSession(currentSessionId, newName);
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Session renamed to: "${newName}"` },
        ]);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to rename session';
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
        const forkedSession = persistenceForkSession(currentSessionId, index, name);
        setCurrentSessionId(forkedSession.id);
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Session forked at index ${index}: "${name}"` },
        ]);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to fork session';
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
          { role: 'tool', content: 'Usage: /merge <session-name> <indices> (e.g., /merge session-b 3,4)' },
        ]);
        return;
      }
      try {
        const { persistenceListSessions, persistenceMergeMessages } = await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        const source = sessions.find((s: SessionManifest) => s.name === sourceName || s.id === sourceName);
        if (!source) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Source session not found: "${sourceName}"` },
          ]);
          return;
        }
        const indices = indicesStr.split(',').map((s: string) => parseInt(s.trim(), 10));
        const result = persistenceMergeMessages(currentSessionId, source.id, indices);
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Merged ${indices.length} messages from "${source.name}"` },
        ]);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to merge messages';
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
          { role: 'tool', content: 'Usage: /cherry-pick <session> <index> [--context N]' },
        ]);
        return;
      }
      try {
        const { persistenceListSessions, persistenceCherryPick } = await import('@sengac/codelet-napi');
        const sessions = persistenceListSessions(currentProjectRef.current);
        const source = sessions.find((s: SessionManifest) => s.name === sourceName || s.id === sourceName);
        if (!source) {
          setConversation(prev => [
            ...prev,
            { role: 'tool', content: `Source session not found: "${sourceName}"` },
          ]);
          return;
        }
        const result = persistenceCherryPick(currentSessionId, source.id, index, context);
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: `Cherry-picked message ${index} with ${context} context messages from "${source.name}"` },
        ]);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to cherry-pick';
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
        setConversation(prev => [
          ...prev,
          { role: 'tool', content: message },
        ]);
        // Update token tracker and context fill to reflect reduced context
        const finalTokens = sessionRef.current.tokenTracker;
        setTokenUsage(finalTokens);
        const contextFillInfo = sessionRef.current.getContextFillInfo();
        setContextFillPercentage(contextFillInfo.fillPercentage);

        // Persist compaction state and token usage
        if (currentSessionId) {
          try {
            const { persistenceSetCompactionState, persistenceSetSessionTokens } = await import('@sengac/codelet-napi');
            // Create summary for persistence (includes key metrics)
            const summary = `Compacted ${result.turnsSummarized} turns (${result.originalTokens}→${result.compactedTokens} tokens, ${compressionPct}% compression)`;
            // compacted_before_index = turnsSummarized (messages 0 to turnsSummarized-1 were compacted)
            persistenceSetCompactionState(currentSessionId, summary, result.turnsSummarized);
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
        const errorMessage = err instanceof Error ? err.message : 'Failed to compact context';
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
    let activeSessionId = currentSessionId;
    if (!activeSessionId && isFirstMessageRef.current) {
      try {
        const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
        const project = currentProjectRef.current;
        // Use first message as session name (truncated to 50 chars)
        const sessionName = userMessage.slice(0, 50) + (userMessage.length > 50 ? '...' : '');

        const persistedSession = persistenceCreateSessionWithProvider(
          sessionName,
          project,
          currentProvider
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
        persistenceAddHistory(userMessage, currentProjectRef.current, activeSessionId);
        // Update local history entries
        setHistoryEntries(prev => [{
          display: userMessage,
          timestamp: new Date().toISOString(),
          project: currentProjectRef.current,
          sessionId: activeSessionId,
          hasPastedContent: false,
        }, ...prev]);
      } catch (err) {
        logger.error(`Failed to save history: ${err instanceof Error ? err.message : String(err)}`);
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
      // Track current text segment (resets after tool calls)
      let currentSegment = '';
      // Track full assistant response for persistence (includes ALL content blocks)
      let fullAssistantResponse = '';
      // Track assistant message content blocks for envelope storage
      const assistantContentBlocks: Array<{ type: string; text?: string; id?: string; name?: string; input?: unknown }> = [];
      await sessionRef.current.prompt(userMessage, (chunk: StreamChunk) => {
        if (!chunk) return;

        if (chunk.type === 'Text' && chunk.text) {
          // Text chunks are batched in Rust for efficiency
          currentSegment += chunk.text;
          fullAssistantResponse += chunk.text; // Accumulate for display persistence
          // Add to content blocks for envelope storage
          const lastBlock = assistantContentBlocks[assistantContentBlocks.length - 1];
          if (lastBlock && lastBlock.type === 'text') {
            lastBlock.text = (lastBlock.text || '') + chunk.text;
          } else {
            assistantContentBlocks.push({ type: 'text', text: chunk.text });
          }
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

          let toolContent = `[Planning to use tool: ${toolCall.name}]`;
          // Parse and display arguments
          if (typeof parsedInput === 'object' && parsedInput !== null) {
            for (const [key, value] of Object.entries(parsedInput as Record<string, unknown>)) {
              const displayValue =
                typeof value === 'string' ? value : JSON.stringify(value);
              toolContent += `\n  ${key}: ${displayValue}`;
            }
          } else if (toolCall.input) {
            toolContent += `\n  ${toolCall.input}`;
          }
          const toolContentSnapshot = toolContent;
          setConversation(prev => {
            const updated = [...prev];
            const streamingIdx = updated.findLastIndex(m => m.isStreaming);
            if (streamingIdx >= 0) {
              // Mark current segment as complete
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
              persistenceStoreMessageEnvelope(activeSessionId, JSON.stringify(assistantEnvelope));
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
                  content: [{
                    type: 'tool_result',
                    tool_use_id: result.toolCallId,
                    content: result.content,
                    is_error: result.isError,
                  }],
                },
              };
              persistenceStoreMessageEnvelope(activeSessionId, JSON.stringify(toolResultEnvelope));
            } catch {
              // Persistence failed - continue
            }
          }

          // Sanitize content: replace tabs with spaces (Ink can't render tabs)
          const sanitizedContent = result.content.replace(/\t/g, '  ');
          const preview = sanitizedContent.slice(0, 500);
          const truncated = sanitizedContent.length > 500;
          // Format like CLI: indented with separators
          const indentedPreview = preview
            .split('\n')
            .map(line => `  ${line}`)
            .join('\n');
          const toolResultContent = `[Tool result preview]\n-------\n${indentedPreview}${truncated ? '...' : ''}\n-------`;
          currentSegment = ''; // Reset for next text segment
          setConversation(prev => [
            ...prev,
            { role: 'tool' as const, content: toolResultContent },
            // Add new streaming placeholder for AI continuation
            { role: 'assistant' as const, content: '', isStreaming: true },
          ]);
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
          // Use ⚠ (U+26A0) without emoji selector - width 1 in both string-width and terminal
          setConversation(prev => {
            const updated = [
              ...prev,
              { role: 'tool' as const, content: '⚠ Agent interrupted' },
            ];
            // Mark any streaming message as complete
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
          if (chunk.tokens.tokensPerSecond !== undefined) {
            setDisplayedTokPerSec(chunk.tokens.tokensPerSecond);
            if (chunk.tokens.tokensPerSecond !== null) {
              setLastChunkTime(Date.now());
            }
          }
        } else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
          // TUI-033: Display context fill percentage from Rust
          setContextFillPercentage(chunk.contextFill.fillPercentage);
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
            updated.push({ role: 'tool', content: `API Error: ${chunk.error}` });
            return updated;
          });
        }
      });

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
              usage: currentTokens ? {
                input_tokens: currentTokens.inputTokens,
                output_tokens: currentTokens.outputTokens,
                cache_read_input_tokens: currentTokens.cacheReadInputTokens ?? 0,
                cache_creation_input_tokens: currentTokens.cacheCreationInputTokens ?? 0,
              } : undefined,
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
            const { persistenceSetSessionTokens } = await import('@sengac/codelet-napi');
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

  // NAPI-006: Navigate to previous history entry (Shift+Arrow-Up)
  const handleHistoryPrev = useCallback(() => {
    if (historyEntries.length === 0) {
      return;
    }

    // Save current input if we're starting navigation
    if (historyIndex === -1) {
      setSavedInput(inputValue);
    }

    const newIndex = historyIndex === -1 ? 0 : Math.min(historyIndex + 1, historyEntries.length - 1);
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
      const results = persistenceSearchHistory(query, currentProjectRef.current);
      const entries: HistoryEntry[] = results.map((h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({
        display: h.display,
        timestamp: h.timestamp,
        project: h.project,
        sessionId: h.sessionId,
        hasPastedContent: h.hasPastedContent ?? false,
      }));
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
    const timeStr = date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays === 1) return `yesterday ${timeStr}`;
    if (diffDays < 7) {
      const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });
      return `${dayName} ${timeStr}`;
    }
    // For older sessions, show date and time
    const monthDay = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    return `${monthDay} ${timeStr}`;
  }, []);

  // NAPI-003: Enter resume mode (show session selection overlay)
  const handleResumeMode = useCallback(async () => {
    try {
      const { persistenceListSessions } = await import('@sengac/codelet-napi');
      const sessions = persistenceListSessions(currentProjectRef.current);

      // Sort by updatedAt descending (most recent first)
      const sorted = [...sessions].sort((a: SessionManifest, b: SessionManifest) =>
        new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      );

      setAvailableSessions(sorted);
      setResumeSessionIndex(0);
      setIsResumeMode(true);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to list sessions';
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: `Resume failed: ${errorMessage}` },
      ]);
    }
  }, []);

  // NAPI-003: Select session and restore conversation
  const handleResumeSelect = useCallback(async () => {
    if (availableSessions.length === 0 || resumeSessionIndex >= availableSessions.length) {
      return;
    }

    const selectedSession = availableSessions[resumeSessionIndex];

    try {
      const { persistenceGetSessionMessages, persistenceGetSessionMessageEnvelopes } = await import('@sengac/codelet-napi');
      const messages = persistenceGetSessionMessages(selectedSession.id);

      // Get FULL envelopes with all content blocks (ToolUse, ToolResult, Text, etc.)
      const envelopes: string[] = persistenceGetSessionMessageEnvelopes(selectedSession.id);

      // Convert full envelopes to conversation format for UI display
      // This properly restores tool calls, tool results, thinking, etc.
      //
      // CRITICAL: Tool results are stored in separate user envelopes after assistant
      // messages, but we need to interleave them correctly by matching tool_use_id.
      const restored: ConversationMessage[] = [];

      // First pass: collect all tool results by their tool_use_id
      const toolResultsByUseId = new Map<string, { content: string; isError: boolean }>();
      for (const envelopeJson of envelopes) {
        try {
          const envelope = JSON.parse(envelopeJson);
          const messageType = envelope.type || envelope.message_type || envelope.messageType;
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
          const messageType = envelope.type || envelope.message_type || envelope.messageType;
          const message = envelope.message;

          if (!message) continue;

          if (messageType === 'user') {
            // User messages - extract text only (tool results handled via interleaving)
            const contents = message.content || [];
            for (const content of contents) {
              if (content.type === 'text' && content.text) {
                restored.push({ role: 'user', content: `${content.text}`, isStreaming: false });
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
                  restored.push({ role: 'assistant', content: textContent, isStreaming: false });
                  textContent = '';
                }
                // Tool call
                let toolContent = `[Planning to use tool: ${content.name}]`;
                const input = content.input;
                if (typeof input === 'object' && input !== null) {
                  for (const [key, value] of Object.entries(input)) {
                    const displayValue = typeof value === 'string' ? value : JSON.stringify(value);
                    toolContent += `\n  ${key}: ${displayValue}`;
                  }
                }
                restored.push({ role: 'tool', content: toolContent, isStreaming: false });

                // Immediately show the tool result (interleaved)
                const toolResult = toolResultsByUseId.get(content.id);
                if (toolResult) {
                  const preview = toolResult.content.slice(0, 500);
                  const truncated = toolResult.content.length > 500;
                  const indentedPreview = preview.split('\n').map((line: string) => `  ${line}`).join('\n');
                  restored.push({
                    role: 'tool',
                    content: `[Tool result preview]\n-------\n${indentedPreview}${truncated ? '...' : ''}\n-------`,
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
              restored.push({ role: 'assistant', content: textContent, isStreaming: false });
            }
          }
        } catch {
          // If envelope parsing fails, fall back to simple format
          logger.warn('Failed to parse envelope, falling back to simple format');
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

        // Switch provider to match the restored session's provider
        // This ensures API calls use the same provider as the original session
        if (selectedSession.provider && selectedSession.provider !== sessionRef.current.currentProviderName) {
          try {
            await sessionRef.current.switchProvider(selectedSession.provider);
            setCurrentProvider(selectedSession.provider);
          } catch (providerErr) {
            // Provider switch failed - continue with current provider
            logger.warn(`Failed to switch to session provider ${selectedSession.provider}: ${providerErr instanceof Error ? providerErr.message : String(providerErr)}`);
          }
        }

        // Restore token state from persisted session (including cache tokens for TUI-033)
        // CTX-003: Restore current context, output, cache tokens, and cumulative billing fields
        if (selectedSession.tokenUsage) {
          sessionRef.current.restoreTokenState(
            selectedSession.tokenUsage.currentContextTokens,           // current input context
            selectedSession.tokenUsage.cumulativeBilledOutput,         // output tokens (use cumulative as we don't store current separately)
            selectedSession.tokenUsage.cacheReadTokens ?? 0,           // cache read
            selectedSession.tokenUsage.cacheCreationTokens ?? 0,       // cache creation
            selectedSession.tokenUsage.cumulativeBilledInput ?? 0,     // cumulative billed input
            selectedSession.tokenUsage.cumulativeBilledOutput ?? 0     // cumulative billed output
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
          cacheCreationInputTokens: selectedSession.tokenUsage.cacheCreationTokens,
        });
      }

      // Build confirmation message with compaction info
      let confirmationMsg = `Session resumed: "${selectedSession.name}" (${selectedSession.messageCount} messages)`;
      if (selectedSession.compaction) {
        confirmationMsg += `\n[Compaction: ${selectedSession.compaction.summary}]`;
      }

      // Add confirmation message
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: confirmationMsg },
      ]);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to restore session';
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

  // Handle keyboard input
  useInput(
    (input, key) => {
      // Skip mouse events (handled by VirtualList)
      if (input.startsWith('[M') || key.mouse) {
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
          setSearchResultIndex(prev => Math.min(searchResults.length - 1, prev + 1));
          return;
        }
        if (key.backspace || key.delete) {
          void handleSearchInput(searchQuery.slice(0, -1));
          return;
        }
        // Accept printable characters for search query
        const clean = input
          .split('')
          .filter((ch) => {
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
          setResumeSessionIndex(prev => Math.min(availableSessions.length - 1, prev + 1));
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

      // Esc key handling - interrupt if loading, close if not
      if (key.escape) {
        if (isLoading && sessionRef.current) {
          // Interrupt the agent execution
          sessionRef.current.interrupt();
        } else {
          // Close the modal
          onClose();
        }
        return;
      }

      // Tab to toggle provider selector
      if (key.tab && availableProviders.length > 1) {
        setShowProviderSelector(true);
        const idx = availableProviders.indexOf(currentProvider);
        setSelectedProviderIndex(idx >= 0 ? idx : 0);
        return;
      }
    },
    { isActive: isOpen }
  );

  if (!isOpen) return null;

  // Flatten conversation messages into individual lines for VirtualList
  // Pre-wrap lines to fit terminal width since VirtualList expects single-line items
  const conversationLines = useMemo((): ConversationLine[] => {
    const maxWidth = terminalWidth - 6; // Account for borders and padding
    const lines: ConversationLine[] = [];

    conversation.forEach((msg, msgIndex) => {
      // Add role prefix to first line
      const prefix =
        msg.role === 'user' ? 'You: ' : msg.role === 'assistant' ? 'AI: ' : '';
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
                lines.push({ role: msg.role, content: currentLine, messageIndex: msgIndex });
                currentLine = '';
                currentWidth = 0;
              }
              // Break long word by visual width
              let chunk = '';
              let chunkWidth = 0;
              for (const char of word) {
                const charWidth = getVisualWidth(char);
                if (chunkWidth + charWidth > maxWidth && chunk) {
                  lines.push({ role: msg.role, content: chunk, messageIndex: msgIndex });
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
                lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });
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
            lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });
          } else if (lines.length === 0 || lines[lines.length - 1]?.messageIndex !== msgIndex) {
            // Ensure at least one line per content section
            lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });
          }
        }
      });

      // Add empty line after each message for spacing (use space to ensure line renders)
      lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });
    });

    return lines;
  }, [conversation, terminalWidth]);

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
          <Box
            flexDirection="column"
            padding={2}
            flexGrow={1}
          >
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
                  backgroundColor={idx === searchResultIndex ? 'magenta' : undefined}
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
          <Box
            flexDirection="column"
            padding={2}
            flexGrow={1}
          >
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

  // Main agent modal (full-screen overlay)
  // Use two-layer structure: outer box for positioning, inner box for styling
  // This prevents border rendering issues with position="absolute" + explicit dimensions
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
        {/* Header with provider and token usage */}
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
              Agent: {currentProvider}
            </Text>
            {isLoading && <Text color="yellow"> (streaming...)</Text>}
            {/* AGENT-021: DEBUG indicator when debug capture is enabled */}
            {isDebugEnabled && <Text color="red" bold> [DEBUG]</Text>}
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
          {availableProviders.length > 1 && (
            <Box marginLeft={2}>
              <Text dimColor>[Tab] Switch</Text>
            </Box>
          )}
        </Box>

        {/* Conversation area using VirtualList for proper scrolling - matches FileDiffViewer pattern */}
        <Box flexGrow={1} flexBasis={0}>
          <VirtualList
            items={conversationLines}
            renderItem={(line) => {
              const color =
                line.role === 'user'
                  ? 'green'
                  : line.role === 'tool'
                    ? 'yellow'
                    : 'white';
              return (
                <Box flexGrow={1}>
                  <Text color={color}>{line.content}</Text>
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
    </Box>
  );
};

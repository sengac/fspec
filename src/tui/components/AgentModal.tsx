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
import stringWidth from 'string-width';
import { VirtualList } from './VirtualList';

/**
 * Normalize emoji variation selectors for consistent terminal width calculation.
 *
 * Many characters have Emoji_Presentation=No in Unicode (they're text by default):
 * - ⚠ ✏ 🖥 ☁ ❤ ✨ ☆ ✱ △ ⚡ and many more
 *
 * When U+FE0F (VS16) is added, string-width correctly reports width 2.
 * BUT many terminals IGNORE U+FE0F and render as width 1, causing layout misalignment.
 *
 * FIX: Strip ALL U+FE0F variation selectors from text.
 * - For text-default chars: removes VS16, string-width reports 1, terminal renders 1 ✓
 * - For emoji-default chars: they're already emoji, VS16 is redundant, no effect
 *
 * See: border-debug.test.tsx for reproduction and explanation
 */
function normalizeEmojiWidth(text: string): string {
  // Remove ALL U+FE0F (Variation Selector-16) characters
  // This ensures string-width matches terminal rendering for text-default emojis
  return text.replace(/\uFE0F/g, '');
}

// Custom TextInput that ignores mouse escape sequences
const SafeTextInput: React.FC<{
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
}> = ({ value, onChange, onSubmit, placeholder = '', isActive = true }) => {
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
  const sessionRef = useRef<CodeletSessionType | null>(null);

  // TUI-031: Track tok/s - calculate on each TEXT chunk for real-time updates
  const streamingStartTimeRef = useRef<number | null>(null);
  const [displayedTokPerSec, setDisplayedTokPerSec] = useState<number | null>(null);
  const [lastChunkTime, setLastChunkTime] = useState<number | null>(null);
  const lastChunkTimeRef = useRef<number | null>(null);
  const rateSamplesRef = useRef<number[]>([]);
  const MAX_RATE_SAMPLES = 5; // Average last 5 samples for stability

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
      // TUI-031: Reset tok/s tracking
      streamingStartTimeRef.current = null;
      setDisplayedTokPerSec(null);
      setLastChunkTime(null);
      lastChunkTimeRef.current = null;
      rateSamplesRef.current = [];
      sessionRef.current = null;
      return;
    }

    const initSession = async () => {
      try {
        // Dynamic import to handle ESM
        const { CodeletSession } = await import('codelet-napi');
        // Default to Claude as the primary AI provider
        const newSession = new CodeletSession('claude');
        setSession(newSession);
        sessionRef.current = newSession;
        setCurrentProvider(newSession.currentProviderName);
        setAvailableProviders(newSession.availableProviders);
        setTokenUsage(newSession.tokenTracker);
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
    setInputValue('');
    setIsLoading(true);
    // TUI-031: Reset tok/s tracking for new prompt
    streamingStartTimeRef.current = Date.now();
    setDisplayedTokPerSec(null);
    setLastChunkTime(null);
    lastChunkTimeRef.current = null;
    rateSamplesRef.current = [];

    // Add user message to conversation
    setConversation(prev => [...prev, { role: 'user', content: userMessage }]);

    // Add streaming assistant message placeholder
    setConversation(prev => [
      ...prev,
      { role: 'assistant', content: '', isStreaming: true },
    ]);

    try {
      // Track current text segment (resets after tool calls)
      let currentSegment = '';

      await sessionRef.current.prompt(userMessage, (chunk: StreamChunk) => {
        if (!chunk) return;

        if (chunk.type === 'Text' && chunk.text) {
          // TUI-031: Calculate tok/s on each text chunk
          const now = Date.now();
          const chunkTokens = Math.ceil(chunk.text.length / 4); // ~4 chars per token

          if (lastChunkTimeRef.current !== null && chunkTokens > 0) {
            const deltaTime = (now - lastChunkTimeRef.current) / 1000;
            if (deltaTime > 0.01) { // At least 10ms between samples
              const instantRate = chunkTokens / deltaTime;
              // Add to samples, keep last N for smoothing
              rateSamplesRef.current.push(instantRate);
              if (rateSamplesRef.current.length > MAX_RATE_SAMPLES) {
                rateSamplesRef.current.shift();
              }
              // Display average of samples
              const avgRate = rateSamplesRef.current.reduce((a, b) => a + b, 0) / rateSamplesRef.current.length;
              setDisplayedTokPerSec(avgRate);
              setLastChunkTime(now);
            }
          }
          lastChunkTimeRef.current = now;

          // Text chunks are now batched in Rust, so we receive fewer, larger updates
          currentSegment += chunk.text;
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
          let toolContent = `[Planning to use tool: ${toolCall.name}]`;
          // Parse and display arguments
          try {
            const args = JSON.parse(toolCall.input);
            if (typeof args === 'object' && args !== null) {
              for (const [key, value] of Object.entries(args)) {
                const displayValue =
                  typeof value === 'string' ? value : JSON.stringify(value);
                toolContent += `\n  ${key}: ${displayValue}`;
              }
            }
          } catch {
            // If input isn't valid JSON, show as-is
            if (toolCall.input) {
              toolContent += `\n  ${toolCall.input}`;
            }
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
          // Update token usage display (tok/s is now calculated from Text chunks)
          setTokenUsage(chunk.tokens);
        } else if (chunk.type === 'Error' && chunk.error) {
          setError(chunk.error);
        }
      });

      // Update token usage after prompt completes (safe to access now - session unlocked)
      if (sessionRef.current) {
        setTokenUsage(sessionRef.current.tokenTracker);
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to send prompt';
      setError(errorMessage);
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

  // Handle keyboard input
  useInput(
    (input, key) => {
      // Skip mouse events (handled by VirtualList)
      if (input.startsWith('[M') || key.mouse) {
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
        if (stringWidth(displayContent) === 0) {
          lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });
        } else {
          // Split into words, keeping whitespace
          const words = displayContent.split(/(\s+)/);
          let currentLine = '';
          let currentWidth = 0;

          for (const word of words) {
            const wordWidth = stringWidth(word);

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
                const charWidth = stringWidth(char);
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
            renderItem={(line, _index, isSelected) => {
              const color =
                line.role === 'user'
                  ? 'green'
                  : line.role === 'tool'
                    ? 'yellow'
                    : 'white';
              return (
                <Box flexGrow={1}>
                  <Text color={isSelected ? 'cyan' : color}>{line.content}</Text>
                </Box>
              );
            }}
            keyExtractor={(_line, index) => `line-${index}`}
            emptyMessage="Type a message to start..."
            showScrollbar={!isLoading}
            isFocused={!isLoading && !showProviderSelector}
            scrollToEnd={true}
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
                placeholder="Type your message..."
              />
            )}
          </Box>
        </Box>
      </Box>
    </Box>
  );
};

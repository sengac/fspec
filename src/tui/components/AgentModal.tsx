/**
 * AgentModal - Modal overlay for AI agent interactions
 *
 * Integrates codelet-napi native module into fspec's TUI to enable
 * AI-powered conversations within the terminal interface.
 *
 * Implements NAPI-002: TUI Integration for Codelet AI Agent
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { TextInput } from '@inkjs/ui';
import { Dialog } from '../../components/Dialog';

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
  prompt: (input: string, callback: (chunk: StreamChunk) => void) => Promise<void>;
  switchProvider: (providerName: string) => Promise<void>;
  clearHistory: () => void;
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

export const AgentModal: React.FC<AgentModalProps> = ({ isOpen, onClose }) => {
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

  // Initialize session when modal opens
  useEffect(() => {
    if (!isOpen) {
      // Reset state when modal closes (fresh session each time)
      setSession(null);
      setConversation([]);
      setTokenUsage({ inputTokens: 0, outputTokens: 0 });
      setError(null);
      setInputValue('');
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
          err instanceof Error ? err.message : 'Failed to initialize AI session';
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

    // Add user message to conversation
    setConversation(prev => [...prev, { role: 'user', content: userMessage }]);

    // Add streaming assistant message placeholder
    setConversation(prev => [
      ...prev,
      { role: 'assistant', content: '', isStreaming: true },
    ]);

    try {
      let streamedContent = '';

      await sessionRef.current.prompt(userMessage, (chunk: StreamChunk) => {
        if (chunk.type === 'Text' && chunk.text) {
          streamedContent += chunk.text;
          // Update the streaming message
          setConversation(prev => {
            const updated = [...prev];
            const lastIdx = updated.length - 1;
            if (updated[lastIdx]?.isStreaming) {
              updated[lastIdx] = {
                ...updated[lastIdx],
                content: streamedContent,
              };
            }
            return updated;
          });
        } else if (chunk.type === 'ToolCall' && chunk.toolCall) {
          // Tool calls are auto-executed, show them in conversation
          setConversation(prev => [
            ...prev,
            {
              role: 'tool',
              content: `Calling: ${chunk.toolCall!.name}`,
            },
          ]);
        } else if (chunk.type === 'ToolResult' && chunk.toolResult) {
          // Show tool results
          const resultPreview = chunk.toolResult.content.slice(0, 100);
          setConversation(prev => [
            ...prev,
            {
              role: 'tool',
              content: `Result: ${resultPreview}${chunk.toolResult!.content.length > 100 ? '...' : ''}`,
            },
          ]);
        } else if (chunk.type === 'Done') {
          // Mark streaming complete
          setConversation(prev => {
            const updated = [...prev];
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
        } else if (chunk.type === 'Error' && chunk.error) {
          setError(chunk.error);
        }
      });

      // Update token usage after prompt completes
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

      // Tab to toggle provider selector
      if (key.tab && availableProviders.length > 1) {
        setShowProviderSelector(true);
        setSelectedProviderIndex(
          availableProviders.indexOf(currentProvider)
        );
        return;
      }
    },
    { isActive: isOpen }
  );

  if (!isOpen) return null;

  // Error state - show setup instructions
  if (error && !session) {
    return (
      <Dialog onClose={onClose} borderColor="red">
        <Box flexDirection="column" minWidth={60} padding={1}>
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
          <Box justifyContent="center">
            <Text dimColor>Press Esc to close</Text>
          </Box>
        </Box>
      </Dialog>
    );
  }

  // Provider selector overlay
  if (showProviderSelector) {
    return (
      <Dialog onClose={() => setShowProviderSelector(false)} borderColor="cyan">
        <Box flexDirection="column" minWidth={40} padding={1}>
          <Box marginBottom={1}>
            <Text bold color="cyan">
              Select Provider
            </Text>
          </Box>
          {availableProviders.map((provider, idx) => (
            <Box key={provider}>
              <Text
                backgroundColor={idx === selectedProviderIndex ? 'cyan' : undefined}
                color={idx === selectedProviderIndex ? 'black' : 'white'}
              >
                {idx === selectedProviderIndex ? '> ' : '  '}
                {provider}
                {provider === currentProvider ? ' (current)' : ''}
              </Text>
            </Box>
          ))}
          <Box marginTop={1} justifyContent="center">
            <Text dimColor>Enter Select | Esc Cancel</Text>
          </Box>
        </Box>
      </Dialog>
    );
  }

  // Main agent modal
  return (
    <Dialog onClose={onClose} borderColor="cyan">
      <Box flexDirection="column" minWidth={70} minHeight={20}>
        {/* Header with provider and token usage */}
        <Box
          borderStyle="single"
          borderBottom={true}
          borderTop={false}
          borderLeft={false}
          borderRight={false}
          paddingX={1}
          marginBottom={1}
        >
          <Box flexGrow={1}>
            <Text bold color="cyan">
              Agent: {currentProvider}
            </Text>
          </Box>
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

        {/* Conversation area */}
        <Box flexDirection="column" flexGrow={1} paddingX={1} overflowY="hidden">
          {conversation.length === 0 ? (
            <Box justifyContent="center" alignItems="center" flexGrow={1}>
              <Text dimColor>Type a message to start...</Text>
            </Box>
          ) : (
            conversation.slice(-10).map((msg, idx) => (
              <Box key={idx} marginBottom={1}>
                <Text
                  color={
                    msg.role === 'user'
                      ? 'green'
                      : msg.role === 'tool'
                        ? 'yellow'
                        : 'white'
                  }
                >
                  {msg.role === 'user' ? 'You: ' : msg.role === 'tool' ? '' : 'AI: '}
                  {msg.content}
                  {msg.isStreaming ? '...' : ''}
                </Text>
              </Box>
            ))
          )}
        </Box>

        {/* Input area */}
        <Box
          borderStyle="single"
          borderTop={true}
          borderBottom={false}
          borderLeft={false}
          borderRight={false}
          paddingX={1}
          marginTop={1}
        >
          <Text color="green">&gt; </Text>
          <Box flexGrow={1}>
            <TextInput
              value={inputValue}
              onChange={setInputValue}
              onSubmit={handleSubmit}
              placeholder={isLoading ? 'Thinking...' : 'Type your message...'}
              isDisabled={isLoading}
            />
          </Box>
        </Box>

        {/* Footer with shortcuts */}
        <Box justifyContent="center" marginTop={1}>
          <Text dimColor>Enter Send | Esc Close</Text>
        </Box>
      </Box>
    </Dialog>
  );
};

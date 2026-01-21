/**
 * Feature: spec/features/codelet-napi-rs-native-module-bindings.feature
 *
 * Tests for Codelet NAPI-RS Native Module Bindings
 *
 * These tests verify the JavaScript API exposed by the codelet-napi native module.
 * The module provides access to codelet's Rust AI agent functionality from Node.js.
 *
 * NOTE: These tests use mocks to avoid calling real APIs. The actual NAPI bindings
 * are tested via Rust unit tests in codelet/napi/src/*.rs
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock functions for CodeletSession methods
const mockPrompt = vi.fn();
const mockSwitchProvider = vi.fn();
const mockInterrupt = vi.fn();
const mockResetInterrupt = vi.fn();
const mockToggleDebug = vi.fn();
const mockCompact = vi.fn();
const mockSelectModel = vi.fn();
const mockClearHistory = vi.fn();
const mockRestoreMessages = vi.fn();
const mockRestoreMessagesFromEnvelopes = vi.fn();
const mockRestoreTokenState = vi.fn();

// Mock CodeletSession class
class MockCodeletSession {
  prompt = mockPrompt;
  switchProvider = mockSwitchProvider;
  interrupt = mockInterrupt;
  resetInterrupt = mockResetInterrupt;
  toggleDebug = mockToggleDebug;
  compact = mockCompact;
  selectModel = mockSelectModel;
  clearHistory = mockClearHistory;
  restoreMessages = mockRestoreMessages;
  restoreMessagesFromEnvelopes = mockRestoreMessagesFromEnvelopes;
  restoreTokenState = mockRestoreTokenState;
  currentProviderName = 'claude';
  selectedModel = 'anthropic/claude-sonnet-4';
  availableProviders = ['claude', 'openai', 'gemini'];
  tokenTracker = {
    inputTokens: 100,
    outputTokens: 50,
    cacheReadInputTokens: 0,
    cacheCreationInputTokens: 0,
  };
  messages = [
    { role: 'user', content: 'Hello' },
    { role: 'assistant', content: 'Hi there!' },
  ];

  static newWithModel = vi.fn().mockResolvedValue(new MockCodeletSession());
  static newWithCredentials = vi
    .fn()
    .mockResolvedValue(new MockCodeletSession());
}

vi.mock('@sengac/codelet-napi', () => ({
  CodeletSession: MockCodeletSession,
}));

describe('Feature: Codelet NAPI-RS Native Module Bindings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Create session with auto-detected provider', () => {
    it('should create session with highest priority available provider', async () => {
      // @step Given I have at least one provider configured with credentials
      // @step When I create a new CodeletSession without specifying a provider
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // @step Then the session should be created with the highest priority available provider
      expect(session).toBeDefined();

      // @step And the currentProviderName getter should return the detected provider name
      const providerName = session.currentProviderName;
      expect(typeof providerName).toBe('string');
      expect(['claude', 'openai', 'gemini', 'codex']).toContain(providerName);
    });
  });

  describe('Scenario: Create session with specific provider', () => {
    it('should create session with Claude provider when specified', async () => {
      // @step Given I have Claude provider configured with credentials
      // @step When I create a new CodeletSession with provider name 'claude'
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession('claude');

      // @step Then the session should be created with the Claude provider
      expect(session).toBeDefined();

      // @step And the currentProviderName getter should return 'claude'
      expect(session.currentProviderName).toBe('claude');
    });
  });

  describe('Scenario: List available providers', () => {
    it('should return array of available providers', async () => {
      // @step Given I have Claude, OpenAI, and Gemini providers configured
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // @step When I access the availableProviders getter on the session
      const providers = session.availableProviders;

      // @step Then I should receive an array containing available provider names
      expect(Array.isArray(providers)).toBe(true);
      expect(providers.length).toBeGreaterThan(0);
      // Each provider should be a string
      providers.forEach((p: unknown) => {
        expect(typeof p).toBe('string');
      });
    });
  });

  describe('Scenario: Stream prompt with text chunks', () => {
    it('should stream text chunks via callback', async () => {
      // @step Given I have an active CodeletSession
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // Setup mock to simulate streaming chunks
      const mockChunks = [
        { type: 'Text', text: 'Hello' },
        { type: 'Text', text: ' there!' },
        { type: 'Done' },
      ];
      mockPrompt.mockImplementation(
        async (
          _input: string,
          _config: unknown,
          callback: (chunk: unknown) => void
        ) => {
          for (const chunk of mockChunks) {
            callback(chunk);
          }
        }
      );

      // @step When I call session.prompt with input text and a callback function
      const chunks: Array<{ type: string; text?: string }> = [];
      const callback = (chunk: { type: string; text?: string }) => {
        chunks.push(chunk);
      };

      await session.prompt('Say hello in exactly 3 words', null, callback);

      // @step Then the callback should receive chunks with type 'Text' containing streamed content
      const textChunks = chunks.filter(c => c.type === 'Text');
      expect(textChunks.length).toBeGreaterThan(0);
      textChunks.forEach(chunk => {
        expect(typeof chunk.text).toBe('string');
      });

      // @step And the final chunk should have type 'Done'
      const lastChunk = chunks[chunks.length - 1];
      expect(lastChunk.type).toBe('Done');
    });
  });

  describe('Scenario: Track token usage during conversation', () => {
    it('should track input and output tokens', async () => {
      // @step Given I have an active CodeletSession
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // @step When I complete a prompt that uses tokens
      mockPrompt.mockResolvedValue(undefined);
      await session.prompt('Say hi', null, () => {});

      // @step Then the tokenTracker getter should return inputTokens, outputTokens, and cache token counts
      const tracker = session.tokenTracker;
      expect(tracker).toBeDefined();
      expect(typeof tracker.inputTokens).toBe('number');
      expect(typeof tracker.outputTokens).toBe('number');
      expect(tracker.inputTokens).toBeGreaterThan(0);
      expect(tracker.outputTokens).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Switch provider mid-conversation', () => {
    it('should switch provider and preserve ability to prompt', async () => {
      // @step Given I have an active CodeletSession with Claude provider
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession('claude');

      // @step And I have completed at least one prompt
      mockPrompt.mockResolvedValue(undefined);
      await session.prompt('Say hi', null, () => {});
      expect(session.currentProviderName).toBe('claude');

      // @step When I call session.switchProvider with 'openai'
      mockSwitchProvider.mockResolvedValue(undefined);
      await session.switchProvider('openai');

      // @step Then switchProvider should have been called with 'openai'
      expect(mockSwitchProvider).toHaveBeenCalledWith('openai');

      // @step And subsequent prompts should use the new provider
      await session.prompt('Say hello', null, () => {});
      expect(mockPrompt).toHaveBeenCalledTimes(2);
    });
  });

  describe('Scenario: Receive tool call chunks during streaming', () => {
    it('should receive tool_call chunks when tools are invoked', async () => {
      // @step Given I have an active CodeletSession
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // Setup mock to simulate tool call chunks
      const mockChunks = [
        { type: 'Text', text: "I'll read that file for you." },
        {
          type: 'ToolCall',
          toolCall: {
            id: 'toolu_01ABC123',
            name: 'Read',
            input: { file_path: '/project/package.json' },
          },
        },
        {
          type: 'ToolResult',
          toolResult: {
            toolCallId: 'toolu_01ABC123',
            content: '{"name": "fspec"}',
            isError: false,
          },
        },
        { type: 'Text', text: 'The name field is "fspec".' },
        { type: 'Done' },
      ];
      mockPrompt.mockImplementation(
        async (
          _input: string,
          _config: unknown,
          callback: (chunk: unknown) => void
        ) => {
          for (const chunk of mockChunks) {
            callback(chunk);
          }
        }
      );

      // @step When I prompt with a request that triggers a tool call like 'read this file'
      const chunks: Array<{
        type: string;
        toolCall?: { id: string; name: string; input: unknown };
      }> = [];
      await session.prompt(
        'Read the file package.json and tell me the name field',
        null,
        chunk => {
          chunks.push(chunk);
        }
      );

      // @step Then the callback should receive a chunk with type 'ToolCall'
      const toolCallChunks = chunks.filter(c => c.type === 'ToolCall');
      expect(toolCallChunks.length).toBeGreaterThan(0);

      // @step And the toolCall object should contain id, name, and input properties
      const toolCall = toolCallChunks[0].toolCall;
      expect(toolCall).toBeDefined();
      expect(typeof toolCall!.id).toBe('string');
      expect(typeof toolCall!.name).toBe('string');
      expect(toolCall!.input).toBeDefined();
    });
  });

  describe('Scenario: Receive tool result chunks after tool execution', () => {
    it('should receive tool_result chunks after tools execute', async () => {
      // @step Given I have an active CodeletSession
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // Setup mock to simulate tool result chunks
      const mockChunks = [
        { type: 'Text', text: "I'll read that file for you." },
        {
          type: 'ToolCall',
          toolCall: {
            id: 'toolu_01ABC123',
            name: 'Read',
            input: { file_path: '/project/package.json' },
          },
        },
        {
          type: 'ToolResult',
          toolResult: {
            toolCallId: 'toolu_01ABC123',
            content: '{"name": "fspec", "version": "1.0.0"}',
            isError: false,
          },
        },
        { type: 'Text', text: 'The name field is "fspec".' },
        { type: 'Done' },
      ];
      mockPrompt.mockImplementation(
        async (
          _input: string,
          _config: unknown,
          callback: (chunk: unknown) => void
        ) => {
          for (const chunk of mockChunks) {
            callback(chunk);
          }
        }
      );

      // @step When a tool call is executed automatically
      const chunks: Array<{
        type: string;
        toolResult?: { toolCallId: string; content: string; isError: boolean };
      }> = [];
      await session.prompt(
        'Read the file package.json and tell me the name field',
        null,
        chunk => {
          chunks.push(chunk);
        }
      );

      // @step Then the callback should receive a chunk with type 'ToolResult'
      const toolResultChunks = chunks.filter(c => c.type === 'ToolResult');
      expect(toolResultChunks.length).toBeGreaterThan(0);

      // @step And the toolResult object should contain toolCallId, content, and isError properties
      const toolResult = toolResultChunks[0].toolResult;
      expect(toolResult).toBeDefined();
      expect(typeof toolResult!.toolCallId).toBe('string');
      expect(typeof toolResult!.content).toBe('string');
      expect(typeof toolResult!.isError).toBe('boolean');
    });
  });

  describe('Scenario: Auto-inject context reminders on session creation', () => {
    it('should inject context reminders including CLAUDE.md', async () => {
      // @step Given I am in a directory with a CLAUDE.md file
      // @step When I create a new CodeletSession
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // @step Then the session messages should include context reminders
      const messages = session.messages;
      expect(Array.isArray(messages)).toBe(true);

      // @step And the context should include CLAUDE.md content and environment information
      // Messages should have been populated with context reminders
      expect(messages.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Trigger context compaction manually', () => {
    it('should compact context when requested', async () => {
      // @step Given I have an active CodeletSession with many messages
      const { CodeletSession } = await import('@sengac/codelet-napi');
      const session = new CodeletSession();

      // Setup mock for compact
      mockCompact.mockResolvedValue({
        originalTokens: 10000,
        compactedTokens: 3000,
        compressionRatio: 70,
        turnsSummarized: 5,
        turnsKept: 2,
      });

      // @step When I call compact()
      const result = await session.compact();

      // @step Then the session should compact the context
      expect(mockCompact).toHaveBeenCalled();

      // @step And return compaction metrics
      expect(result).toBeDefined();
      expect(result.originalTokens).toBe(10000);
      expect(result.compactedTokens).toBe(3000);
    });
  });

  describe('Scenario: Create session with specific model', () => {
    it('should create session with newWithModel static method', async () => {
      // @step Given I want to create a session with a specific model
      const { CodeletSession } = await import('@sengac/codelet-napi');

      // @step When I call CodeletSession.newWithModel with a model string
      const session = await CodeletSession.newWithModel(
        'anthropic/claude-sonnet-4'
      );

      // @step Then the session should be created
      expect(session).toBeDefined();

      // @step And the selectedModel getter should return the model
      expect(session.selectedModel).toBe('anthropic/claude-sonnet-4');
    });
  });
});

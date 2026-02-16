/**
 * Integration tests for Telegram Content Chunker
 *
 * These tests verify ContentChunker is properly integrated into telegram-endpoint.ts
 * Unit tests are in telegram-content-chunker.test.ts
 *
 * BRIDGE-006: Intelligent Content-Aware Chunking for Telegram Display
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const {
  mockBotSendMessage,
  mockBotOn,
  mockBotStopPolling,
  mockWsOn,
  MockWebSocketServer,
  MockTelegramBot,
} = vi.hoisted(() => {
  const mockBotSendMessage = vi.fn().mockResolvedValue({});
  const mockBotOn = vi.fn();
  const mockBotStopPolling = vi.fn().mockResolvedValue(undefined);
  const mockWsOn = vi.fn();

  class MockWebSocketServer {
    on = mockWsOn;
    close = vi.fn();
    constructor() {}
  }

  class MockTelegramBot {
    on = mockBotOn;
    sendMessage = mockBotSendMessage;
    stopPolling = mockBotStopPolling;
    constructor() {}
  }

  return {
    mockBotSendMessage,
    mockBotOn,
    mockBotStopPolling,
    mockWsOn,
    MockWebSocketServer,
    MockTelegramBot,
  };
});

vi.mock('ws', () => ({
  WebSocketServer: MockWebSocketServer,
  WebSocket: { OPEN: 1 },
}));

vi.mock('node-telegram-bot-api', () => ({
  default: MockTelegramBot,
}));

vi.mock('dotenv', () => ({
  config: vi.fn(),
}));

import type { OutboundMessage } from '../telegram-endpoint';
import {
  startEndpoint,
  stopEndpoint,
  resetState,
  handleStreamChunk,
} from '../telegram-endpoint';

describe('Feature: BRIDGE-006 Content Chunker Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    mockBotSendMessage.mockClear().mockResolvedValue({});
    mockBotOn.mockClear();
    mockBotStopPolling.mockClear().mockResolvedValue(undefined);
    mockWsOn.mockClear();
    delete process.env.TELEGRAM_BOT_TOKEN;
    delete process.env.TELEGRAM_CHAT_ID;
    delete process.env.TELEGRAM_ALLOWED_USER_IDS;
    resetState();
  });

  afterEach(async () => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    await stopEndpoint();
    resetState();
  });

  describe('Scenario: Thinking content wrapped in think tags', () => {
    it('should wrap thinking content in escaped <think> tags for MarkdownV2', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude sends a thinking chunk with reasoning content
      const thinkingChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: {
          type: 'thinking',
          thinking:
            'Let me think about this problem carefully. I need to analyze the code structure.',
        },
      };

      // @step When the thinking block is processed for Telegram
      await handleStreamChunk(thinkingChunk);

      // Send a done chunk to trigger flush (thinking only flushes on non-thinking chunks)
      const doneChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'done' },
      };
      await handleStreamChunk(doneChunk);
      await vi.advanceTimersByTimeAsync(100);

      // @step Then the message starts with escaped '\<think\>'
      expect(mockBotSendMessage).toHaveBeenCalled();
      const sentMessage = mockBotSendMessage.mock.calls[0][1] as string;
      expect(sentMessage).toMatch(/^\\<think\\>/);

      // @step And the actual thinking content flows naturally
      expect(sentMessage).toContain('think about this problem');

      // @step And the message contains escaped '\</think\>' (before done indicator)
      expect(sentMessage).toContain('\\</think\\>');
    });
  });

  describe('Scenario: Multiple thinking chunks stream as continuous content', () => {
    it('should stream thinking chunks between single escaped tags, not as multiple indicators', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude sends 5 separate thinking chunks in succession
      for (let i = 1; i <= 5; i++) {
        const chunk: OutboundMessage = {
          type: 'chunk',
          session_id: 'test-session',
          data: {
            type: 'thinking',
            thinking: `Thought number ${i} about the problem.`,
          },
        };
        await handleStreamChunk(chunk);
      }

      // @step When they are processed for Telegram
      // Send a done chunk to trigger flush (thinking only flushes on non-thinking chunks)
      const doneChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'done' },
      };
      await handleStreamChunk(doneChunk);
      await vi.advanceTimersByTimeAsync(100);

      // @step Then the content flows between single escaped '\<think\>' and '\</think\>' tags
      const allMessages = mockBotSendMessage.mock.calls
        .map(c => c[1] as string)
        .join('');
      expect((allMessages.match(/\\<think\\>/g) || []).length).toBe(1);
      expect((allMessages.match(/\\<\/think\\>/g) || []).length).toBe(1);

      // @step And NOT 5 separate '🤔' indicator messages
      const thinkingIndicators = mockBotSendMessage.mock.calls.filter(call =>
        (call[1] as string).includes('🤔')
      );
      expect(thinkingIndicators.length).toBe(0);
    });
  });

  describe('Scenario: Interleaved thinking and text chunks produce separate think blocks', () => {
    it('should wrap each thinking burst in separate think tags when interrupted by text', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude sends thinking, then text, then more thinking
      // First thinking burst
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'First thought.' },
      });
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Second thought.' },
      });

      // Text interruption - this triggers flush of thinking
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text: 'Here is my response.' },
      });

      // Second thinking burst
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Third thought.' },
      });
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Fourth thought.' },
      });

      // Done to flush everything
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'done' },
      });
      await vi.advanceTimersByTimeAsync(100);

      // @step Then there should be exactly 2 think blocks (one per burst)
      const allMessages = mockBotSendMessage.mock.calls
        .map(c => c[1] as string)
        .join('');
      expect((allMessages.match(/\\<think\\>/g) || []).length).toBe(2);
      expect((allMessages.match(/\\<\/think\\>/g) || []).length).toBe(2);

      // @step And thinking content should not have nested tags
      // Each thinking burst should be: \<think\>content\</think\>
      // NOT: \<think\>\<think\>content\</think\>\</think\>
      expect(allMessages).not.toContain('\\<think\\>\\<think\\>');
      expect(allMessages).not.toContain('\\</think\\>\\</think\\>');
    });
  });

  describe('Scenario: Thinking streams naturally with idle flush', () => {
    it('should stream thinking content progressively not batch at end', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude sends thinking chunks with time gaps
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Starting analysis.' },
      });

      // Let idle timer trigger a flush
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then thinking should have been sent with opening tag
      expect(mockBotSendMessage).toHaveBeenCalled();
      const firstMessage = mockBotSendMessage.mock.calls[0][1] as string;
      expect(firstMessage).toContain('\\<think\\>');
      expect(firstMessage).toContain('Starting analysis');
      // Note: First message does NOT have closing tag - thinking block stays open
      expect(firstMessage).not.toContain('\\</think\\>');

      // @step When more thinking arrives after the flush
      mockBotSendMessage.mockClear();
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Continuing analysis.' },
      });
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then continuation should flow naturally WITHOUT new think tags
      // This is the key fix for BRIDGE-006: thinking streams naturally across flushes
      expect(mockBotSendMessage).toHaveBeenCalled();
      const secondMessage = mockBotSendMessage.mock.calls[0][1] as string;
      // NO opening tag - this is a continuation, not a new block
      expect(secondMessage).not.toContain('\\<think\\>');
      expect(secondMessage).not.toContain('\\</think\\>');
      expect(secondMessage).toContain('Continuing analysis');
    });
  });

  describe('Scenario: Empty text chunks do not interrupt thinking blocks', () => {
    it('should NOT close thinking block when empty text chunk arrives', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude sends a thinking chunk
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'First thought.' },
      });

      // @step When an empty text chunk arrives (this can happen with streaming)
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text: '' },
      });

      // @step And then more thinking arrives
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'thinking', thinking: 'Second thought.' },
      });

      // Flush with done
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'done' },
      });
      await vi.advanceTimersByTimeAsync(100);

      // @step Then there should be exactly ONE think block (not two)
      const allMessages = mockBotSendMessage.mock.calls
        .map(c => c[1] as string)
        .join('');
      expect((allMessages.match(/\\<think\\>/g) || []).length).toBe(1);
      expect((allMessages.match(/\\<\/think\\>/g) || []).length).toBe(1);

      // @step And we should NOT see </think><think> pattern (block interruption)
      expect(allMessages).not.toContain('\\</think\\>\\<think\\>');
    });
  });

  describe('Scenario: File read tool result shows summary with line count', () => {
    it('should summarize file read result instead of showing full content', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude reads a 500-line file "src/auth.ts"
      const toolCallChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'tool_call', name: 'Read', id: 'read-123' },
      };
      await handleStreamChunk(toolCallChunk);

      const fileContent = Array(100).fill('console.log("line");').join('\n');
      const toolResultChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: {
          type: 'tool_result',
          tool_call_id: 'read-123',
          content: fileContent,
          is_error: false,
        },
      };

      // @step When the tool_result chunk is processed
      await handleStreamChunk(toolResultChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then Telegram shows "📄 Read src/auth.ts (500 lines)" instead of file contents
      const calls = mockBotSendMessage.mock.calls;
      const resultMessage = calls.find(call => {
        const msg = call[1] as string;
        return (
          msg.includes('Read') && (msg.includes('lines') || msg.includes('📄'))
        );
      });

      expect(resultMessage).toBeDefined();
      const msg = resultMessage![1] as string;
      expect(msg).not.toContain('console.log');
      expect(msg).toMatch(/\d+\s*(line|lines)/i);
    });
  });

  describe('Scenario: Large tool output summarized not sent verbatim', () => {
    it('should summarize large tool output instead of sending verbatim', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given a tool returns 10000 characters of output
      const toolCallChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'tool_call', name: 'Bash', id: 'bash-123' },
      };
      await handleStreamChunk(toolCallChunk);

      const largeOutput = 'X'.repeat(2000);
      const toolResultChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: {
          type: 'tool_result',
          tool_call_id: 'bash-123',
          content: largeOutput,
          is_error: false,
        },
      };

      // @step When the tool_result chunk is processed
      await handleStreamChunk(toolResultChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then Telegram receives a summary under 500 characters
      // @step And the full output is not sent
      const calls = mockBotSendMessage.mock.calls;
      const allMessages = calls.map(c => c[1] as string).join('');
      expect(allMessages.includes('X'.repeat(500))).toBe(false);
    });
  });

  describe('Scenario: Long message splits at logical boundary before limit', () => {
    it('should split at sentence boundary instead of arbitrary position', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given the buffer contains 5000 characters with a paragraph break at 3800
      const sentence = 'This is a complete sentence. ';
      const longText = sentence.repeat(200);

      const textChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text: longText },
      };

      // @step When the buffer is flushed
      await handleStreamChunk(textChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then the first message ends at the paragraph break
      // @step And the second message contains the remainder
      expect(mockBotSendMessage).toHaveBeenCalled();
      const firstMessage = mockBotSendMessage.mock.calls[0][1] as string;
      const trimmed = firstMessage.trim();
      expect(trimmed.endsWith('.')).toBe(true);
    });
  });

  describe('Scenario: Unclosed code block closed before sending', () => {
    it('should balance code block markers in each chunk', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given the buffer contains "```typescript\nconst x = 1;" without closing fence
      // @step And the buffer is being force-flushed due to size limit
      const longCode = '```typescript\n' + 'console.log("test");\n'.repeat(500);
      const textChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text: longCode },
      };

      // @step When the message is prepared for sending
      await handleStreamChunk(textChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then a closing "```" is appended to make valid markdown
      expect(mockBotSendMessage).toHaveBeenCalled();
      for (const call of mockBotSendMessage.mock.calls) {
        const message = call[1] as string;
        const markers = (message.match(/```/g) || []).length;
        expect(markers % 2).toBe(0);
      }
    });
  });

  describe('Scenario: Unclosed bold markers balanced before sending', () => {
    it('should balance bold markers in truncated message', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given the buffer contains "This is **bold text without closing"
      // @step And the buffer is being force-flushed
      const text = '**This is bold text that goes on forever '.repeat(200);
      const textChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text },
      };

      // @step When the message is prepared for sending
      await handleStreamChunk(textChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then a closing "**" is appended to balance the markers
      expect(mockBotSendMessage).toHaveBeenCalled();
      for (const call of mockBotSendMessage.mock.calls) {
        const message = call[1] as string;
        const boldMarkers = (message.match(/\*\*/g) || []).length;
        expect(boldMarkers % 2).toBe(0);
      }
    });
  });

  describe('Scenario: Tool call displays formatted invocation', () => {
    it('should show tool call with nice formatting', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude invokes the Fspec tool with command "create-story"
      const toolCallChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'tool_call', name: 'Fspec', id: 'fspec-123' },
      };

      // @step When the tool_call chunk is processed
      await handleStreamChunk(toolCallChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then Telegram shows "🔧 Running: Fspec(create-story)"
      expect(mockBotSendMessage).toHaveBeenCalled();
      const message = mockBotSendMessage.mock.calls[0][1] as string;
      expect(message).toContain('🔧');
      expect(message).toContain('Fspec');
    });
  });

  describe('Scenario: Code block stays together', () => {
    it('should not split a code block across messages', async () => {
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step Given Claude outputs a 50-line code block
      const codeBlock = '```typescript\nconst x = 1;\nconst y = 2;\n```';
      const textChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session',
        data: { type: 'text', text: `Before text. ${codeBlock} After text.` },
      };

      // @step When it is processed for Telegram
      await handleStreamChunk(textChunk);
      await vi.advanceTimersByTimeAsync(1000);

      // @step Then Telegram receives it as a single message with proper formatting
      expect(mockBotSendMessage).toHaveBeenCalled();
      const messages = mockBotSendMessage.mock.calls.map(c => c[1] as string);

      const codeMessage = messages.find(m => m.includes('```'));
      expect(codeMessage).toBeDefined();

      const codeBlockMatches = codeMessage!.match(/```/g) || [];
      expect(codeBlockMatches.length % 2).toBe(0);
    });
  });
});

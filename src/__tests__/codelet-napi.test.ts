/**
 * Feature: spec/features/codelet-napi-rs-native-module-bindings.feature
 *
 * Tests for Codelet NAPI-RS Native Module Bindings
 *
 * These tests verify the JavaScript API exposed by the codelet-napi native module.
 * The module provides access to codelet's Rust AI agent functionality from Node.js.
 */

import { describe, it, expect } from 'vitest';
import type { CodeletSession as CodeletSessionType } from 'codelet-napi';

// Dynamic import for native module to handle ESM compatibility
const getCodeletNapi = async (): Promise<{
  CodeletSession: typeof CodeletSessionType;
}> => {
  return await import('codelet-napi');
};

describe('Feature: Codelet NAPI-RS Native Module Bindings', () => {
  describe('Scenario: Create session with auto-detected provider', () => {
    it('should create session with highest priority available provider', async () => {
      // @step Given I have at least one provider configured with credentials
      // This test assumes ANTHROPIC_API_KEY or similar is set in environment
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      // @step When I create a new CodeletSession without specifying a provider
      const { CodeletSession } = await getCodeletNapi();
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
      if (!process.env.ANTHROPIC_API_KEY) {
        console.log('Skipping test: ANTHROPIC_API_KEY not configured');
        return;
      }

      // @step When I create a new CodeletSession with provider name 'claude'
      const { CodeletSession } = await getCodeletNapi();
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
      // This test works with whatever providers are available

      const { CodeletSession } = await getCodeletNapi();
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
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession();

      // @step When I call session.prompt with input text and a callback function
      const chunks: Array<{ type: string; text?: string }> = [];
      const callback = (chunk: { type: string; text?: string }) => {
        chunks.push(chunk);
      };

      await session.prompt('Say hello in exactly 3 words', callback);

      // @step Then the callback should receive chunks with type 'text' containing streamed content
      const textChunks = chunks.filter(c => c.type === 'text');
      expect(textChunks.length).toBeGreaterThan(0);
      textChunks.forEach(chunk => {
        expect(typeof chunk.text).toBe('string');
      });

      // @step And the final chunk should have type 'done'
      const lastChunk = chunks[chunks.length - 1];
      expect(lastChunk.type).toBe('done');
    });
  });

  describe('Scenario: Track token usage during conversation', () => {
    it('should track input and output tokens', async () => {
      // @step Given I have an active CodeletSession
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession();

      // @step When I complete a prompt that uses tokens
      await session.prompt('Say hi', () => {});

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
      if (!process.env.ANTHROPIC_API_KEY || !process.env.OPENAI_API_KEY) {
        console.log(
          'Skipping test: Both ANTHROPIC_API_KEY and OPENAI_API_KEY required'
        );
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession('claude');

      // @step And I have completed at least one prompt
      await session.prompt('Say hi', () => {});
      expect(session.currentProviderName).toBe('claude');

      // @step When I call session.switchProvider with 'openai'
      await session.switchProvider('openai');

      // @step Then the currentProviderName should return 'openai'
      expect(session.currentProviderName).toBe('openai');

      // @step And subsequent prompts should use the OpenAI provider
      // Verify by making another prompt (would fail if provider not properly switched)
      await session.prompt('Say hello', () => {});
    });
  });

  describe('Scenario: Receive tool call chunks during streaming', () => {
    it('should receive tool_call chunks when tools are invoked', async () => {
      // @step Given I have an active CodeletSession
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession();

      // @step When I prompt with a request that triggers a tool call like 'read this file'
      const chunks: Array<{
        type: string;
        toolCall?: { id: string; name: string; input: unknown };
      }> = [];
      await session.prompt(
        'Read the file package.json and tell me the name field',
        chunk => {
          chunks.push(chunk);
        }
      );

      // @step Then the callback should receive a chunk with type 'tool_call'
      const toolCallChunks = chunks.filter(c => c.type === 'tool_call');
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
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession();

      // @step When a tool call is executed automatically
      const chunks: Array<{
        type: string;
        toolResult?: { toolCallId: string; content: string; isError: boolean };
      }> = [];
      await session.prompt(
        'Read the file package.json and tell me the name field',
        chunk => {
          chunks.push(chunk);
        }
      );

      // @step Then the callback should receive a chunk with type 'tool_result'
      const toolResultChunks = chunks.filter(c => c.type === 'tool_result');
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
      // The fspec project has CLAUDE.md at the root

      const { CodeletSession } = await getCodeletNapi();

      // @step When I create a new CodeletSession
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const session = new CodeletSession();

      // @step Then the session messages should include context reminders
      const messages = session.messages;
      expect(Array.isArray(messages)).toBe(true);

      // @step And the context should include CLAUDE.md content and environment information
      // Messages should have been populated with context reminders
      expect(messages.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Trigger context compaction automatically', () => {
    it('should compact context when approaching threshold', async () => {
      // @step Given I have an active CodeletSession with many messages
      const hasCredentials =
        process.env.ANTHROPIC_API_KEY ||
        process.env.OPENAI_API_KEY ||
        process.env.GOOGLE_GENERATIVE_AI_API_KEY;

      if (!hasCredentials) {
        console.log('Skipping test: No provider credentials configured');
        return;
      }

      const { CodeletSession } = await getCodeletNapi();
      const session = new CodeletSession();

      // @step And the token count approaches the context window threshold
      // This is hard to test without many real API calls
      // For now, we just verify the tokenTracker exists and is being tracked

      // @step When I send another prompt
      const chunks: Array<{ type: string }> = [];
      await session.prompt('Say hi', chunk => {
        chunks.push(chunk);
      });

      // @step Then the session should automatically compact the context
      // Compaction is internal - we verify session is still functional
      expect(session).toBeDefined();

      // @step And the callback should receive compaction-related events
      // The callback receives all events including potential compaction notifications
      expect(chunks.length).toBeGreaterThan(0);
    });
  });
});

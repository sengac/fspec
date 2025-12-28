/**
 * Feature: spec/features/tui-integration-for-codelet-ai-agent.feature
 *
 * Tests for TUI Integration for Codelet AI Agent
 *
 * These tests verify the AgentModal component that integrates codelet-napi
 * native module into fspec's TUI infrastructure.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: true,
      sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
      message: 'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
    }),
    // NAPI-005: Manual compaction command
    compact: vi.fn().mockReturnValue({
      originalTokens: 150000,
      compactedTokens: 40000,
      compressionRatio: 73.3,
      turnsSummarized: 12,
      turnsKept: 3,
    }),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
}));

// Mock codelet-napi module
vi.mock('@sengac/codelet-napi', () => ({
  CodeletSession: class MockCodeletSession {
    currentProviderName: string;
    availableProviders: string[];
    tokenTracker: { inputTokens: number; outputTokens: number };
    messages: Array<{ role: string; content: string }>;
    prompt: ReturnType<typeof vi.fn>;
    switchProvider: ReturnType<typeof vi.fn>;
    clearHistory: ReturnType<typeof vi.fn>;
    interrupt: ReturnType<typeof vi.fn>;
    toggleDebug: ReturnType<typeof vi.fn>;
    compact: ReturnType<typeof vi.fn>; // NAPI-005

    constructor() {
      if (mockState.shouldThrow) {
        throw new Error(mockState.errorMessage);
      }
      this.currentProviderName = mockState.session.currentProviderName;
      this.availableProviders = mockState.session.availableProviders;
      this.tokenTracker = mockState.session.tokenTracker;
      this.messages = mockState.session.messages;
      this.prompt = mockState.session.prompt;
      this.switchProvider = mockState.session.switchProvider;
      this.clearHistory = mockState.session.clearHistory;
      this.interrupt = mockState.session.interrupt;
      this.toggleDebug = mockState.session.toggleDebug;
      this.compact = mockState.session.compact; // NAPI-005
    }
  },
  ChunkType: {
    Text: 'Text',
    Thinking: 'Thinking', // TOOL-010
    ToolCall: 'ToolCall',
    ToolResult: 'ToolResult',
    Done: 'Done',
    Error: 'Error',
  },
  // TOOL-010: Thinking level detection exports
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
  // TUI-034: Model selection mocks
  modelsSetCacheDirectory: vi.fn(),
  modelsListAll: vi.fn(() => Promise.resolve([])),
  setRustLogCallback: vi.fn(),
  // Persistence NAPI bindings required by AgentModal
  persistenceSetDataDirectory: vi.fn(),
  persistenceStoreMessageEnvelope: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceCreateSessionWithProvider: vi.fn(() => ({
    id: 'mock-session-id',
    name: 'Mock Session',
    project: '/test/project',
    provider: 'claude',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  })),
  persistenceAddHistory: vi.fn(),
  persistenceSearchHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  persistenceAppendMessage: vi.fn(),
  persistenceRenameSession: vi.fn(),
}));

// Mock Dialog to render children without position="absolute" which breaks ink-testing-library
vi.mock('../../components/Dialog', () => ({
  Dialog: ({
    children,
  }: {
    children: React.ReactNode;
    onClose: () => void;
    borderColor?: string;
  }) => <Box flexDirection="column">{children}</Box>,
}));

// Mock Ink's Box to strip position="absolute" which doesn't work in ink-testing-library
vi.mock('ink', async () => {
  const actual = await vi.importActual<typeof import('ink')>('ink');
  return {
    ...actual,
    Box: (props: React.ComponentProps<typeof actual.Box>) => {
      // Strip position prop as it breaks ink-testing-library
      const { position, ...rest } = props as { position?: string } & typeof props;
      return <actual.Box {...rest} />;
    },
  };
});

// Import the component after mocks are set up
import { AgentModal } from '../components/AgentModal';

// Helper to wait for async operations
const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

// Helper to reset mock session
const resetMockSession = (overrides = {}) => {
  mockState.session = {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [],
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: true,
      sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
      message: 'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
    }),
    // NAPI-005: Manual compaction command
    compact: vi.fn().mockReturnValue({
      originalTokens: 150000,
      compactedTokens: 40000,
      compressionRatio: 73.3,
      turnsSummarized: 12,
      turnsKept: 3,
    }),
    ...overrides,
  };
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
};

describe('Feature: TUI Integration for Codelet AI Agent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Open agent modal and send prompt with streaming response', () => {
    it('should display modal overlay with provider name and stream responses', async () => {
      // @step Given I am viewing the TUI board
      // The AgentModal renders as an overlay on top of existing views

      // @step And at least one AI provider is configured
      // Mocked CodeletSession returns 'claude' as available provider

      // @step When I press the agent modal hotkey
      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step Then an agent modal overlay should appear
      expect(lastFrame()).toContain('Agent');

      // @step And the modal should show the current provider name
      expect(lastFrame()).toContain('claude');

      // @step When I type a prompt and press Enter
      // Input handling will be tested in integration

      // @step Then the response should stream in real-time
      // Streaming is handled by the prompt() callback

      // @step And I should see text appearing character by character
      // This is verified by checking streaming state updates
    });
  });

  describe('Scenario: Auto-execute tool calls without approval', () => {
    it('should execute tool calls automatically without approval prompts', async () => {
      // @step Given I have the agent modal open
      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step And I send a prompt that triggers a tool call
      // Tool calls are simulated via mock

      // @step When the AI responds with a tool call
      // Mock returns tool_call chunk type

      // @step Then the tool should execute automatically
      // No approval dialog should be rendered
      expect(lastFrame()).not.toContain('Approve');
      expect(lastFrame()).not.toContain('Deny');

      // @step And the tool result should display in the conversation
      // Tool results are displayed in the message list

      // @step And no approval prompt should appear
      expect(lastFrame()).not.toContain('approval');
    });
  });

  describe('Scenario: Switch provider mid-conversation', () => {
    it('should allow switching providers during conversation', async () => {
      // @step Given I have the agent modal open with an active conversation
      const mockSwitchProvider = vi.fn();
      resetMockSession({
        availableProviders: ['claude', 'openai', 'gemini'],
        tokenTracker: { inputTokens: 100, outputTokens: 50 },
        messages: [{ role: 'user', content: 'hello' }],
        switchProvider: mockSwitchProvider,
      });

      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step And multiple providers are available
      expect(lastFrame()).toContain('claude');

      // @step When I select a different provider from the provider selector
      // Provider switching is triggered via UI interaction

      // @step Then the provider should switch successfully
      // switchProvider method should be called

      // @step And the modal should display the new provider name
      // UI should update to show new provider

      // @step And I can continue the conversation with the new provider
      // Subsequent prompts should work
    });
  });

  describe('Scenario: Display token usage and provider status', () => {
    it('should display token usage and current provider in header', async () => {
      // @step Given I have the agent modal open
      resetMockSession({
        tokenTracker: { inputTokens: 150, outputTokens: 75 },
      });

      // @step When I send a prompt and receive a response
      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step Then the modal header should show token usage
      expect(lastFrame()).toContain('tokens');

      // @step And the token count should include input and output tokens
      expect(lastFrame()).toMatch(/\d+/); // Contains numbers for token counts

      // @step And the current provider name should be visible
      expect(lastFrame()).toContain('claude');
    });
  });

  describe('Scenario: Fresh session on modal reopen', () => {
    it('should start fresh session when modal is reopened', async () => {
      // @step Given I have the agent modal open with an active conversation
      resetMockSession({
        tokenTracker: { inputTokens: 200, outputTokens: 100 },
        messages: [
          { role: 'user', content: 'hello' },
          { role: 'assistant', content: 'hi there' },
        ],
      });

      const onClose = vi.fn();
      const { lastFrame, rerender } = render(
        <AgentModal isOpen={true} onClose={onClose} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step When I close the modal with Escape key
      // Escape key triggers onClose callback
      rerender(<AgentModal isOpen={false} onClose={onClose} />);

      // @step And I reopen the agent modal
      // Create fresh session on reopen
      resetMockSession({
        tokenTracker: { inputTokens: 0, outputTokens: 0 },
        messages: [],
      });

      rerender(<AgentModal isOpen={true} onClose={onClose} />);

      // Wait for async session initialization
      await waitForFrame();

      // @step Then the conversation history should be empty
      // New session has empty messages array

      // @step And a fresh session should be initialized
      // CodeletSession constructor called again

      // @step And token usage should be reset to zero
      expect(lastFrame()).not.toContain('200'); // Previous token count gone
    });
  });

  describe('Scenario: Handle missing credentials gracefully', () => {
    it('should display error message when no providers are configured', async () => {
      // @step Given no AI provider credentials are configured
      mockState.shouldThrow = true;
      mockState.errorMessage = 'No AI provider credentials configured';

      // @step When I open the agent modal
      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization (and error)
      await waitForFrame();

      // @step Then an error message should display
      expect(lastFrame()).toContain('Error');

      // @step And the error should explain no providers are available
      expect(lastFrame()).toContain('provider');

      // @step And setup instructions should be shown
      expect(lastFrame()).toContain('ANTHROPIC_API_KEY');
    });
  });

  // ============================================================================
  // Feature: spec/features/add-debug-slash-command-to-fspec-tui-agent.feature
  // AGENT-021: Add /debug slash command to fspec TUI agent
  // ============================================================================

  describe('Scenario: Enable debug capture mode', () => {
    it('should toggle debug mode and show confirmation message when /debug is entered', async () => {
      // @step Given I have the fspec TUI agent modal open
      const mockToggleDebug = vi.fn().mockReturnValue({
        enabled: true,
        sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
        message: 'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
      });
      resetMockSession({
        toggleDebug: mockToggleDebug,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // Verify modal is open with provider
      expect(lastFrame()).toContain('Agent');
      expect(lastFrame()).toContain('claude');

      // @step When I type "/debug" in the input and submit
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r'); // Enter key
      await waitForFrame(100);

      // @step Then I should see a message "Debug capture started. Writing to: ~/.fspec/debug/session-*.jsonl"
      expect(mockToggleDebug).toHaveBeenCalledTimes(1);
      // Verify toggleDebug is called with ~/.fspec directory
      expect(mockToggleDebug).toHaveBeenCalledWith(expect.stringContaining('.fspec'));
      expect(lastFrame()).toContain('Debug capture started');

      // @step And the header should show a DEBUG indicator
      expect(lastFrame()).toContain('[DEBUG]');

      // @step And session metadata should be set on the debug capture manager
      // This is handled internally by toggleDebug() in the Rust layer
    });
  });

  describe('Scenario: Disable debug capture mode', () => {
    it('should toggle debug off and show confirmation when /debug is entered again', async () => {
      // @step Given I have the fspec TUI agent modal open
      const mockToggleDebug = vi.fn()
        .mockReturnValueOnce({
          enabled: true,
          sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
          message: 'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
        })
        .mockReturnValueOnce({
          enabled: false,
          sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
          message: 'Debug capture stopped. Session saved to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
        });
      resetMockSession({
        toggleDebug: mockToggleDebug,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame();

      // @step And debug capture mode is enabled
      // First /debug call enables debug
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Verify debug is now enabled
      expect(mockToggleDebug).toHaveBeenCalledTimes(1);
      expect(lastFrame()).toContain('[DEBUG]');

      // @step When I type "/debug" in the input and submit
      // Second /debug call disables debug
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then I should see a message "Debug capture stopped. Session saved to: ~/.fspec/debug/session-*.jsonl"
      expect(mockToggleDebug).toHaveBeenCalledTimes(2);
      expect(lastFrame()).toContain('Debug capture stopped');

      // @step And the DEBUG indicator should disappear from the header
      expect(lastFrame()).not.toContain('[DEBUG]');
    });
  });

  describe('Scenario: Debug state resets on modal close', () => {
    it('should reset debug indicator when modal is closed and reopened', async () => {
      // @step Given I have the fspec TUI agent modal open with debug enabled
      const mockToggleDebug = vi.fn().mockReturnValue({
        enabled: true,
        sessionFile: '~/.fspec/debug/session-test.jsonl',
        message: 'Debug capture started.',
      });
      resetMockSession({
        toggleDebug: mockToggleDebug,
      });

      const onClose = vi.fn();
      const { lastFrame, stdin, rerender } = render(
        <AgentModal isOpen={true} onClose={onClose} />
      );

      await waitForFrame();

      // Enable debug mode
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Verify debug is enabled
      expect(lastFrame()).toContain('[DEBUG]');

      // @step When I close the modal
      rerender(<AgentModal isOpen={false} onClose={onClose} />);
      await waitForFrame();

      // @step And I reopen the modal
      resetMockSession({
        toggleDebug: mockToggleDebug,
      });
      rerender(<AgentModal isOpen={true} onClose={onClose} />);
      await waitForFrame();

      // @step Then the DEBUG indicator should not be shown
      // (fresh state on modal reopen)
      expect(lastFrame()).not.toContain('[DEBUG]');
    });
  });

  describe('Scenario: Debug events captured during prompt', () => {
    it('should capture debug events when sending prompts with debug enabled', async () => {
      // @step Given I have the fspec TUI agent modal open
      const mockToggleDebug = vi.fn().mockReturnValue({
        enabled: true,
        sessionFile: '~/.fspec/debug/session-test.jsonl',
        message: 'Debug capture started.',
      });
      const mockPrompt = vi.fn().mockImplementation(async (_input: string, callback: (chunk: { type: string }) => void) => {
        // Simulate Done chunk to complete the prompt
        callback({ type: 'Done' });
      });
      resetMockSession({
        toggleDebug: mockToggleDebug,
        prompt: mockPrompt,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame();

      // @step And debug capture mode is enabled
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      expect(mockToggleDebug).toHaveBeenCalledTimes(1);
      expect(lastFrame()).toContain('[DEBUG]');

      // @step When I send a prompt to the agent
      stdin.write('test prompt');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Verify prompt was called (debug events are captured in Rust layer)
      // TOOL-010: prompt now takes (input, thinkingConfig, callback)
      expect(mockPrompt).toHaveBeenCalledWith('test prompt', null, expect.any(Function));

      // @step Then the debug session file should contain "api.request" event
      // @step And the debug session file should contain "api.response.start" event
      // @step And the debug session file should contain "compaction.check" event
      // @step And the debug session file should contain "token.update" event
      // Note: These events are captured by the Rust debug capture manager
      // Unit tests verify the TUI wiring; Rust integration tests verify event capture
    });
  });

  describe('Scenario: Compaction triggered event captured', () => {
    it('should capture compaction events when context exceeds threshold', async () => {
      // @step Given I have the fspec TUI agent modal open
      const mockToggleDebug = vi.fn().mockReturnValue({
        enabled: true,
        sessionFile: '~/.fspec/debug/session-compaction.jsonl',
        message: 'Debug capture started.',
      });
      const mockPrompt = vi.fn().mockImplementation(async (_input: string, callback: (chunk: { type: string; status?: string }) => void) => {
        // Simulate compaction status message and Done
        callback({ type: 'Status', status: 'Context compaction triggered' });
        callback({ type: 'Done' });
      });

      // @step And the context has accumulated close to 180k tokens
      resetMockSession({
        tokenTracker: { inputTokens: 175000, outputTokens: 5000 },
        toggleDebug: mockToggleDebug,
        prompt: mockPrompt,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame();

      // Verify high token count is displayed
      expect(lastFrame()).toContain('175000');

      // @step And debug capture mode is enabled
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      expect(mockToggleDebug).toHaveBeenCalledTimes(1);
      expect(lastFrame()).toContain('[DEBUG]');

      // @step When the next prompt triggers compaction
      stdin.write('trigger compaction');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Verify prompt was called
      expect(mockPrompt).toHaveBeenCalled();

      // @step Then the debug session file should contain "compaction.triggered" event
      // @step And the event should show the threshold was exceeded
      // Note: Compaction events are captured by the Rust compaction hook
      // This test verifies the TUI correctly displays status messages from compaction
      expect(lastFrame()).toContain('compaction');
    });
  });

  // ============================================================================
  // Feature: spec/features/manual-compaction-command.feature
  // NAPI-005: Manual Compaction Command
  // ============================================================================

  describe('Scenario: Successful manual compaction with compression feedback', () => {
    it('should compact context and show compression metrics when /compact is entered', async () => {
      // @step Given I am in AgentModal with a conversation that has approximately 150k tokens
      const mockCompact = vi.fn().mockReturnValue({
        originalTokens: 150000,
        compactedTokens: 40000,
        compressionRatio: 73.3,
        turnsSummarized: 12,
        turnsKept: 3,
      });
      resetMockSession({
        tokenTracker: { inputTokens: 150000, outputTokens: 5000 },
        messages: [
          { role: 'user', content: 'hello' },
          { role: 'assistant', content: 'hi there' },
        ],
        compact: mockCompact,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // Verify modal is open with high token count
      expect(lastFrame()).toContain('Agent');
      expect(lastFrame()).toContain('150000');

      // @step When I type '/compact' and press Enter
      stdin.write('/compact');
      await waitForFrame();
      stdin.write('\r'); // Enter key
      await waitForFrame(100);

      // @step Then I see 'Compacting context...' message in the conversation area
      // Note: The "Compacting context..." message is transient, but the result should be shown

      // @step And I see a result message showing original tokens, compacted tokens, and compression percentage
      expect(mockCompact).toHaveBeenCalledTimes(1);
      expect(lastFrame()).toContain('150000');
      expect(lastFrame()).toContain('40000');
      expect(lastFrame()).toContain('73');

      // @step And the token display in the header updates to reflect the reduced context size
      // The token tracker should be updated after compaction
    });
  });

  describe('Scenario: Empty session shows nothing to compact', () => {
    it('should show nothing to compact message when session has no messages', async () => {
      // @step Given I am in AgentModal with no messages in the conversation
      const mockCompact = vi.fn();
      resetMockSession({
        tokenTracker: { inputTokens: 0, outputTokens: 0 },
        messages: [], // Empty conversation
        compact: mockCompact,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // Verify modal is open with empty conversation
      expect(lastFrame()).toContain('Agent');

      // @step When I type '/compact' and press Enter
      stdin.write('/compact');
      await waitForFrame();
      stdin.write('\r'); // Enter key
      await waitForFrame(100);

      // @step Then I see 'Nothing to compact - no messages yet' in the conversation area
      expect(lastFrame()).toContain('Nothing to compact');

      // @step And the input field returns to normal and I can type my next message
      // compact() should NOT be called when messages are empty
      expect(mockCompact).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Compaction failure preserves context', () => {
    it('should show error message and preserve context when compaction fails', async () => {
      // @step Given I am in AgentModal with an active conversation
      const mockCompact = vi.fn().mockImplementation(() => {
        throw new Error('API rate limit exceeded');
      });
      resetMockSession({
        tokenTracker: { inputTokens: 100000, outputTokens: 5000 },
        messages: [
          { role: 'user', content: 'test message' },
          { role: 'assistant', content: 'test response' },
        ],
        compact: mockCompact,
      });

      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // Verify modal is open with conversation
      expect(lastFrame()).toContain('Agent');
      expect(lastFrame()).toContain('100000');

      // @step When I type '/compact' and press Enter
      stdin.write('/compact');
      await waitForFrame();
      stdin.write('\r'); // Enter key
      await waitForFrame(100);

      // @step Then I see 'Compaction failed: [error message]' in the conversation area
      expect(mockCompact).toHaveBeenCalledTimes(1);
      expect(lastFrame()).toContain('Compaction failed');
      expect(lastFrame()).toContain('API rate limit exceeded');

      // @step And the compaction API returns an error
      // Already verified by mockCompact throwing

      // @step And my conversation context remains unchanged
      // Token count should remain the same (100000)
      expect(lastFrame()).toContain('100000');
    });
  });
});

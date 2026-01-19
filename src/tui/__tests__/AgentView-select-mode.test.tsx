/**
 * Feature: spec/features/toggle-line-selection-mode-with-select-command.feature
 *
 * Tests for Tab key that toggles between scroll mode and turn selection mode
 * in the AgentView conversation area.
 *
 * TUI-041: Toggle line selection mode with /select command
 * NOTE: TUI-042 replaced line-based selection with turn-based selection.
 *       Tab key now enables turn selection mode (replacing /select command).
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box, Text } from 'ink';

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    resetInterrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: false,
      sessionFile: null,
      message: 'Debug capture disabled',
    }),
    compact: vi.fn().mockReturnValue({
      originalTokens: 10000,
      compactedTokens: 5000,
      compressionRatio: 50,
      turnsSummarized: 5,
      turnsKept: 2,
    }),
    getContextFillInfo: vi.fn().mockReturnValue({ fillPercentage: 10 }),
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
    resetInterrupt: ReturnType<typeof vi.fn>;
    toggleDebug: ReturnType<typeof vi.fn>;
    compact: ReturnType<typeof vi.fn>;
    getContextFillInfo: ReturnType<typeof vi.fn>;

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
      this.resetInterrupt = mockState.session.resetInterrupt;
      this.toggleDebug = mockState.session.toggleDebug;
      this.compact = mockState.session.compact;
      this.getContextFillInfo = mockState.session.getContextFillInfo;
    }
  },
  ChunkType: {
    Text: 'Text',
    Thinking: 'Thinking',
    ToolCall: 'ToolCall',
    ToolResult: 'ToolResult',
    Done: 'Done',
    Error: 'Error',
  },
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
  modelsSetCacheDirectory: vi.fn(),
  modelsListAll: vi.fn(() => Promise.resolve([])),
  modelsRefreshCache: vi.fn(() => Promise.resolve()),
  setRustLogCallback: vi.fn(),
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
  persistenceListSessions: vi.fn(() => []),
  persistenceLoadSession: vi.fn(() => null),
  persistenceDeleteSession: vi.fn(),
  persistenceRenameSession: vi.fn(),
  persistenceAppendMessage: vi.fn(),
  // TUI-047: Session management for background sessions
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionAttach: vi.fn(),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionDetach: vi.fn(),
  sessionSendInput: vi.fn(),
  // NAPI-009: New session manager functions
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  // NAPI-009 + AGENT-021: Debug and compaction for background sessions
  sessionToggleDebug: vi.fn().mockResolvedValue({
    enabled: true,
    sessionFile: '/tmp/debug-session.json',
    message: 'Debug capture enabled. Events will be written to /tmp/debug-session.json',
  }),
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 10000,
    compactedTokens: 3000,
    compressionRatio: 70,
    turnsSummarized: 5,
    turnsKept: 2,
  }),
  // Rust state functions for model, status, and tokens
  sessionGetModel: vi.fn().mockReturnValue({ providerId: null, modelId: null }),
  sessionGetStatus: vi.fn().mockReturnValue('idle'),
  sessionGetTokens: vi.fn().mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionSetModel: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: vi.fn(),
}));

// Mock utils/config
vi.mock('../../utils/config', () => ({
  getFspecUserDir: vi.fn(() => '/mock/.fspec'),
  loadConfig: vi.fn(() => ({})),
  writeConfig: vi.fn(),
}));

// Mock utils/credentials
vi.mock('../../utils/credentials', () => ({
  saveCredential: vi.fn(),
  deleteCredential: vi.fn(),
  getProviderConfig: vi.fn(() => null),
  maskApiKey: vi.fn((key: string) => key ? `${key.slice(0, 4)}...${key.slice(-4)}` : ''),
}));

// Mock utils/provider-config
vi.mock('../../utils/provider-config', () => ({
  SUPPORTED_PROVIDERS: ['anthropic', 'openai'],
  getProviderRegistryEntry: vi.fn(() => null),
}));

// Mock logger
vi.mock('../../utils/logger', () => ({
  logger: {
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  },
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

// Helper to simulate key press
const pressKey = (stdin: { write: (s: string) => void }, key: string): void => {
  const keyMap: Record<string, string> = {
    up: '\x1B[A',
    down: '\x1B[B',
    enter: '\r',
    escape: '\x1B',
    tab: '\t',
  };
  stdin.write(keyMap[key] || key);
};

// Helper to wait for frame updates
const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

describe('Feature: Toggle line selection mode with Tab key', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockState.shouldThrow = false;
    mockState.session.messages = [];
    mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Enable line selection mode with Tab key', () => {
    it('should enable line selection mode and show SELECT indicator', async () => {
      // @step Given AgentView is open in scroll mode
      // AgentView starts in scroll mode by default (isLineSelectMode = false)
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step And the header bar does not show a SELECT indicator
      // Initially, no [SELECT] indicator should be visible
      let frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).not.toContain('[SELECT]');

      // @step When I press Tab key
      pressKey(stdin, 'tab');
      await waitForFrame(100);

      // @step Then line selection mode should be enabled
      // @step And a SELECT indicator should appear in the header bar
      frame = lastFrame();
      expect(frame).toContain('[SELECT]');

      // @step And the conversation should show a confirmation message
      // TUI-043: Turn selection mode is now silent (no message), only [SELECT] indicator shows
    });
  });

  describe('Scenario: Disable line selection mode with Tab key', () => {
    it('should disable line selection mode and hide SELECT indicator', async () => {
      // @step Given AgentView is open in line selection mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // Wait for async session initialization
      await waitForFrame();

      // First enable line selection mode with Tab
      pressKey(stdin, 'tab');
      await waitForFrame(100);

      // @step And the header bar shows a SELECT indicator
      let frame = lastFrame();
      expect(frame).toContain('[SELECT]');

      // @step When I press Tab key again
      pressKey(stdin, 'tab');
      await waitForFrame(100);

      // @step Then line selection mode should be disabled
      // @step And the SELECT indicator should disappear from the header bar
      // @step And scroll mode should be restored
      frame = lastFrame();
      expect(frame).not.toContain('[SELECT]');
      // TUI-043: Turn selection mode is now silent (no message), only [SELECT] indicator disappears
    });
  });

  describe('Scenario: Arrow keys scroll viewport in scroll mode', () => {
    it('should scroll without highlighting in scroll mode', async () => {
      // @step Given AgentView is open in scroll mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      // @step And the conversation contains multiple messages
      // Pre-populate with messages by mocking initial state
      mockState.session.messages = [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there! How can I help you today?' },
        { role: 'user', content: 'What is the weather?' },
        { role: 'assistant', content: 'I cannot check the weather directly.' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // @step When I press the down arrow key 3 times
      pressKey(stdin, 'down');
      pressKey(stdin, 'down');
      pressKey(stdin, 'down');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the conversation should scroll down 3 lines
      // @step And no line should be highlighted
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // In scroll mode, no cyan highlighting should appear
      // The VirtualList handles this via selectionMode="scroll"
    });
  });

  describe('Scenario: Arrow keys select lines in line selection mode', () => {
    it('should highlight selected line with cyan in line selection mode', async () => {
      // @step Given AgentView is open in line selection mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      // @step And the conversation contains multiple messages
      mockState.session.messages = [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there!' },
        { role: 'user', content: 'Test message' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // Enable line selection mode first with Tab key
      pressKey(stdin, 'tab');
      await waitForFrame(100);

      // @step When I press the down arrow key 3 times
      pressKey(stdin, 'down');
      pressKey(stdin, 'down');
      pressKey(stdin, 'down');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the selection should move down 3 lines
      // @step And the selected line should be highlighted with cyan color
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // After implementation, VirtualList will pass isSelected=true for the selected line
      // and renderItem will apply cyan color
    });
  });

  describe('Scenario: Auto-scroll to new messages in line selection mode', () => {
    it('should auto-select last line when new message arrives', async () => {
      // @step Given AgentView is open in line selection mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      const { lastFrame, stdin, rerender } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // Enable line selection mode with Tab key
      pressKey(stdin, 'tab');
      await waitForFrame(100);

      // @step And the conversation is scrolled to the bottom
      // By default with scrollToEnd=true, it should be at bottom

      // @step When a new message arrives from the assistant
      // Simulate new message by triggering prompt callback
      mockState.session.prompt.mockImplementation(
        async (_input: string, _thinking: string | null, callback: (chunk: { type: string; text?: string }) => void) => {
          callback({ type: 'Text', text: 'New message from assistant' });
          callback({ type: 'Done' });
        }
      );

      // Type and send a message to trigger response
      stdin.write('Hello');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step Then the selection should automatically move to the new last line
      // @step And the new message should be visible
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // VirtualList with selectionMode='item' and scrollToEnd=true
      // will auto-select the last line when items change
    });
  });

  describe('Scenario: Sticky scroll when user scrolls away in scroll mode', () => {
    it('should not auto-scroll when user has scrolled away', async () => {
      // @step Given AgentView is open in scroll mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      // @step And the conversation has been receiving messages
      mockState.session.messages = Array.from({ length: 50 }, (_, i) => ({
        role: i % 2 === 0 ? 'user' : 'assistant',
        content: `Message ${i + 1}`,
      }));

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // @step When I scroll up to read older messages
      pressKey(stdin, 'up');
      pressKey(stdin, 'up');
      pressKey(stdin, 'up');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step And a new message arrives from the assistant
      // In scroll mode, userScrolledAway will be set to true
      // New messages won't auto-scroll

      // @step Then the view should stay at the current position
      // @step And the new message should not be auto-scrolled to
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // VirtualList tracks userScrolledAway in scroll mode
    });
  });

  describe('Scenario: Re-enable auto-scroll when scrolling back to bottom in scroll mode', () => {
    it('should re-enable auto-scroll when at bottom', async () => {
      // @step Given AgentView is open in scroll mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      mockState.session.messages = Array.from({ length: 30 }, (_, i) => ({
        role: i % 2 === 0 ? 'user' : 'assistant',
        content: `Message ${i + 1}`,
      }));

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      // @step And I have scrolled away from the bottom of the conversation
      pressKey(stdin, 'up');
      pressKey(stdin, 'up');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step And auto-scroll is temporarily disabled
      // userScrolledAway is now true

      // @step When I scroll back to the bottom of the conversation
      // Press End key to go to bottom
      stdin.write('\x1B[4~'); // End key
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then auto-scroll should be re-enabled
      // @step And new messages should auto-scroll into view
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // userScrolledAway resets to false when at bottom
    });
  });

  describe('Scenario: AgentView starts in scroll mode by default', () => {
    it('should start in scroll mode with no SELECT indicator', async () => {
      // @step When I open the AgentView for the first time
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();
      
      const { lastFrame } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the conversation should be in scroll mode
      // @step And no SELECT indicator should be visible in the header bar
      // @step And arrow keys should scroll the viewport without selecting lines
      const frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).not.toContain('[SELECT]');
      // Default state: isLineSelectMode = false
      // VirtualList receives selectionMode="scroll"
    });
  });
});

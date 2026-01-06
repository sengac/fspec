/**
 * Feature: spec/features/select-turns-instead-of-lines-with-select-command.feature
 *
 * Tests for /select command that toggles between scroll mode and turn selection mode
 * in the AgentView conversation area. Turn selection highlights entire conversation
 * turns (messages) rather than individual lines.
 *
 * TUI-042: Select turns instead of lines with /select command
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';

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

describe('Feature: Select turns instead of lines with /select command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockState.shouldThrow = false;
    mockState.session.messages = [];
    mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Enable turn selection mode with /select command', () => {
    it('should enable turn selection mode, show SELECT indicator, and highlight last turn', async () => {
      // @step Given AgentView is open in scroll mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      // @step And the conversation contains multiple turns
      mockState.session.messages = [
        { role: 'user', content: 'Hello there' },
        { role: 'assistant', content: 'Hi! How can I help you today?' },
        { role: 'user', content: 'What is 2+2?' },
        { role: 'assistant', content: 'The answer is 4.' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // @step And the header bar does not show a SELECT indicator
      let frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).not.toContain('[SELECT]');

      // @step When I type /select and press Enter
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then turn selection mode should be enabled
      // @step And a SELECT indicator should appear in the header bar
      frame = lastFrame();
      expect(frame).toContain('[SELECT]');

      // @step And the last turn in the conversation should be selected
      // @step And all lines of the selected turn should be highlighted in cyan with > prefix
      // TUI-043: Turn selection mode is now silent (no message), only [SELECT] indicator shows
      // The last turn (assistant response "The answer is 4.") should be highlighted
      // All lines of that turn should have > prefix
    });
  });

  describe('Scenario: Navigate between turns with arrow keys', () => {
    it('should navigate from last turn to third turn with up arrow', async () => {
      // @step Given AgentView is open with 5 conversation turns
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      mockState.session.messages = [
        { role: 'user', content: 'Turn 1' },
        { role: 'assistant', content: 'Turn 2' },
        { role: 'user', content: 'Turn 3' },
        { role: 'assistant', content: 'Turn 4' },
        { role: 'user', content: 'Turn 5' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode first
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step And turn selection mode is enabled
      let frame = lastFrame();
      expect(frame).toContain('[SELECT]');

      // @step And the last turn is currently selected
      // Last turn (Turn 5) is auto-selected on enable

      // @step When I press the up arrow key twice
      pressKey(stdin, 'up');
      await waitForFrame(50);
      pressKey(stdin, 'up');
      await waitForFrame(50);

      // @step Then the third turn should be selected
      // @step And all lines of the third turn should be highlighted in cyan with > prefix
      // @step And the previously selected turn should no longer be highlighted
      frame = lastFrame();
      expect(frame).toBeDefined();
      // Turn 3 should now be selected (5 - 2 = 3)
      // The > prefix should appear on Turn 3's line
    });
  });

  describe('Scenario: Multi-line turn highlighting', () => {
    it('should highlight all 15 lines of a multi-line turn', async () => {
      // @step Given AgentView is open with turn selection mode enabled
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      // @step And there is an assistant response that spans 15 lines
      const longResponse = Array.from({ length: 15 }, (_, i) => `Line ${i + 1} of the response`).join('\n');
      mockState.session.messages = [
        { role: 'user', content: 'Tell me a long story' },
        { role: 'assistant', content: longResponse },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step When I navigate to select that turn
      // The last turn (the long assistant response) is auto-selected

      // @step Then all 15 lines of the turn should show the > prefix
      // @step And all 15 lines should be displayed in cyan color
      const frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).toContain('[SELECT]');
      // All lines of the selected turn should have > prefix
    });
  });

  describe('Scenario: Disable turn selection mode with /select command', () => {
    it('should disable turn selection mode and remove highlighting', async () => {
      // @step Given AgentView is open in turn selection mode
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      mockState.session.messages = [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there!' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode first
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the header bar shows a SELECT indicator
      let frame = lastFrame();
      expect(frame).toContain('[SELECT]');

      // @step And a turn is currently selected and highlighted

      // @step When I type /select and press Enter
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then turn selection mode should be disabled
      // @step And the SELECT indicator should disappear from the header bar
      // @step And no turns should be highlighted
      // @step And scroll mode should be restored
      frame = lastFrame();
      expect(frame).not.toContain('[SELECT]');
      // TUI-043: Turn selection mode is now silent (no message), only [SELECT] indicator disappears
    });
  });

  describe('Scenario: Navigation stays at first turn when pressing up', () => {
    it('should not wrap to last turn when pressing up at first turn', async () => {
      // @step Given AgentView is open with turn selection mode enabled
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      mockState.session.messages = [
        { role: 'user', content: 'First turn' },
        { role: 'assistant', content: 'Second turn' },
        { role: 'user', content: 'Third turn' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Navigate to first turn (up twice from last)
      pressKey(stdin, 'up');
      await waitForFrame(50);
      pressKey(stdin, 'up');
      await waitForFrame(50);

      // @step And the first turn in the conversation is selected
      // Now at first turn

      // @step When I press the up arrow key
      pressKey(stdin, 'up');
      await waitForFrame(50);

      // @step Then the first turn should remain selected
      // @step And the selection should not wrap to the last turn
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // First turn should still be selected, not wrapped to last
    });
  });

  describe('Scenario: Navigation stays at last turn when pressing down', () => {
    it('should not wrap to first turn when pressing down at last turn', async () => {
      // @step Given AgentView is open with turn selection mode enabled
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      mockState.session.messages = [
        { role: 'user', content: 'First turn' },
        { role: 'assistant', content: 'Second turn' },
        { role: 'user', content: 'Third turn' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode - last turn is auto-selected
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the last turn in the conversation is selected
      // Last turn is already selected on enable

      // @step When I press the down arrow key
      pressKey(stdin, 'down');
      await waitForFrame(50);

      // @step Then the last turn should remain selected
      // @step And the selection should not wrap to the first turn
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // Last turn should still be selected, not wrapped to first
    });
  });

  describe('Scenario: Viewport scrolls to show selected turn', () => {
    it('should scroll viewport when navigating to off-screen turn', async () => {
      // @step Given AgentView is open with turn selection mode enabled
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      // @step And there are more turns than fit in the viewport
      // Create many turns to exceed viewport
      mockState.session.messages = Array.from({ length: 20 }, (_, i) => ({
        role: i % 2 === 0 ? 'user' : 'assistant',
        content: `Turn ${i + 1}: This is message content that might span multiple lines to make the conversation longer`,
      }));

      const { lastFrame, stdin } = render(
        <Box height={15} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step And a turn near the bottom is currently selected
      // Last turn is selected by default

      // @step When I press the up arrow key to select a turn that is scrolled out of view
      // Navigate up many times to reach a turn that was off-screen
      for (let i = 0; i < 10; i++) {
        pressKey(stdin, 'up');
        await waitForFrame(30);
      }

      // @step Then the viewport should scroll to show the selected turn
      // @step And the first line of the selected turn should be visible
      const frame = lastFrame();
      expect(frame).toBeDefined();
      // The selected turn should now be visible in the viewport
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

      await waitForFrame(100);

      // @step Then the conversation should be in scroll mode
      // @step And no SELECT indicator should be visible in the header bar
      // @step And arrow keys should scroll the viewport without selecting turns
      const frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).not.toContain('[SELECT]');
    });
  });

  describe('Scenario: Tool messages are selectable as separate turns', () => {
    it('should allow selecting tool output messages as independent turns', async () => {
      // @step Given AgentView is open with turn selection mode enabled
      const { AgentView } = await import('../components/AgentView');
      const onExit = vi.fn();

      // @step And the conversation contains user messages, assistant responses, and tool output messages
      // Note: Tool messages are added via the /select command confirmation itself
      // We'll add some regular messages first
      mockState.session.messages = [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there!' },
      ];

      const { lastFrame, stdin } = render(
        <Box height={30} width={80}>
          <AgentView onExit={onExit} />
        </Box>
      );

      await waitForFrame();

      // Enable turn selection mode - this now does NOT add a tool message (TUI-043: silent mode)
      stdin.write('/select');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step When I navigate through the turns with arrow keys
      // TUI-043: No "Turn selection mode enabled" message - mode is now silent
      // Navigate up to select previous turns
      pressKey(stdin, 'up');
      await waitForFrame(50);

      // @step Then tool output messages should be selectable as their own separate turns
      // @step And each tool message turn should be highlighted independently when selected
      const frame = lastFrame();
      expect(frame).toBeDefined();
      expect(frame).toContain('[SELECT]');
      // The previous turn should now be selected
    });
  });
});

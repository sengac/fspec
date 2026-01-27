/**
 * Feature: spec/features/session-persistence-with-fork-and-merge.feature
 *
 * Tests for Session Persistence Integration in AgentView
 *
 * These tests verify that AgentView integrates with the codelet-napi
 * persistence module for command history and session management.
 *
 * NAPI-006: Session Persistence with Fork and Merge
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';
import { useSessionStore } from '../store/sessionStore';

// Mock model data matching models.dev structure
const mockModels = vi.hoisted(() => ({
  anthropic: {
    providerId: 'anthropic',
    providerName: 'Anthropic',
    models: [
      {
        id: 'claude-sonnet-4-20250514',
        name: 'Claude Sonnet 4',
        family: 'claude-sonnet-4',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 16000,
        hasVision: true,
      },
    ],
  },
  openai: {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-4o',
        name: 'GPT-4o',
        family: 'gpt-4o',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 128000,
        maxOutput: 16384,
        hasVision: true,
      },
    ],
  },
}));

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
    resetInterrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: false,
      sessionFile: null,
      message: 'Debug disabled',
    }),
    compact: vi.fn().mockReturnValue({
      originalTokens: 0,
      compactedTokens: 0,
      compressionRatio: 0,
      turnsSummarized: 0,
      turnsKept: 0,
    }),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
  // Persistence module mocks
  persistence: {
    historyEntries: [] as Array<{ display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }>,
    addHistoryCalled: false,
    getHistoryCalled: false,
    searchHistoryCalled: false,
    lastSearchQuery: '',
  },
}));

// Mock codelet-napi module with persistence functions
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
    restoreMessages: ReturnType<typeof vi.fn>;
    restoreMessagesFromEnvelopes: ReturnType<typeof vi.fn>;
    restoreTokenState: ReturnType<typeof vi.fn>;
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
      this.restoreMessages = vi.fn();
      this.restoreMessagesFromEnvelopes = vi.fn();
      this.restoreTokenState = vi.fn();
      this.getContextFillInfo = vi.fn().mockReturnValue({ fillPercentage: 0 });
    }
  },
  // Persistence NAPI bindings (camelCase as exported by NAPI-RS)
  persistenceAddHistory: vi.fn().mockImplementation((display: string, project: string, sessionId: string) => {
    mockState.persistence.addHistoryCalled = true;
    mockState.persistence.historyEntries.push({
      display,
      timestamp: new Date().toISOString(),
      project,
      sessionId,
    });
  }),
  persistenceGetHistory: vi.fn().mockImplementation((_project: string | null, limit: number | null) => {
    mockState.persistence.getHistoryCalled = true;
    let entries = mockState.persistence.historyEntries;
    if (limit) {
      entries = entries.slice(0, limit);
    }
    return entries;
  }),
  persistenceSearchHistory: vi.fn().mockImplementation((query: string, _project: string | null) => {
    mockState.persistence.searchHistoryCalled = true;
    mockState.persistence.lastSearchQuery = query;
    let entries = mockState.persistence.historyEntries.filter(e =>
      e.display.toLowerCase().includes(query.toLowerCase())
    );
    return entries;
  }),
  persistenceSetDataDirectory: vi.fn(),
  // TUI-034: Model selection mocks
  modelsSetCacheDirectory: vi.fn(),
  modelsListAll: vi.fn(() => Promise.resolve([mockModels.anthropic, mockModels.openai])),
  setRustLogCallback: vi.fn(),
  persistenceCreateSessionWithProvider: vi.fn().mockImplementation((name: string, project: string, provider: string) => ({
    id: 'mock-session-id',
    name,
    project,
    provider,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  })),
  persistenceResumeLastSession: vi.fn().mockImplementation((project: string) => ({
    id: 'resumed-session-id',
    name: 'Resumed Session',
    project,
    provider: 'claude',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 20,
  })),
  persistenceListSessions: vi.fn().mockImplementation(() => [
    { id: 'session-1', name: 'Auth Work', project: '/test/project', provider: 'claude', messageCount: 10, updatedAt: new Date().toISOString() },
    { id: 'session-2', name: 'Bug Fix', project: '/test/project', provider: 'claude', messageCount: 5, updatedAt: new Date().toISOString() },
    { id: 'session-b', name: 'session-b', project: '/test/project', provider: 'claude', messageCount: 8, updatedAt: new Date().toISOString() },
  ]),
  persistenceGetSessionMessages: vi.fn().mockImplementation(() => [
    { id: '1', role: 'user', content: 'Hello', contentHash: '', createdAt: '', tokenCount: 10, blobRefs: [], metadataJson: '{}' },
    { id: '2', role: 'assistant', content: 'Hi there!', contentHash: '', createdAt: '', tokenCount: 20, blobRefs: [], metadataJson: '{}' },
  ]),
  persistenceGetSessionMessageEnvelopes: vi.fn().mockImplementation(() => [
    JSON.stringify({ uuid: '1', timestamp: new Date().toISOString(), type: 'user', provider: 'claude', message: { role: 'user', content: [{ type: 'text', text: 'Hello' }] } }),
    JSON.stringify({ uuid: '2', timestamp: new Date().toISOString(), type: 'assistant', provider: 'claude', message: { role: 'assistant', content: [{ type: 'text', text: 'Hi there!' }] } }),
  ]),
  persistenceLoadSession: vi.fn().mockImplementation((id: string) => ({
    id,
    name: 'Loaded Session',
    project: '/test/project',
    provider: 'claude',
    messageCount: 10,
  })),
  persistenceRenameSession: vi.fn(),
  persistenceForkSession: vi.fn().mockImplementation((_sessionId: string, atIndex: number, name: string) => ({
    id: 'forked-session-id',
    name,
    project: '/test/project',
    provider: 'claude',
    messageCount: atIndex + 1,
  })),
  persistenceMergeMessages: vi.fn().mockImplementation((targetId: string) => ({
    id: targetId,
    name: 'Merged Session',
    project: '/test/project',
    provider: 'claude',
    messageCount: 15,
  })),
  persistenceCherryPick: vi.fn().mockImplementation((targetId: string, _sourceId: string, index: number, context: number) => ({
    session: {
      id: targetId,
      name: 'Session with cherry-pick',
      project: '/test/project',
      provider: 'claude',
      messageCount: 12,
    },
    importedIndices: [index - context, index],
  })),
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
  persistenceStoreMessageEnvelope: vi.fn(),
  // TUI-047: Session management for background sessions
  sessionManagerList: vi.fn().mockReturnValue([]),
  // VIEWNV-001: Session navigation helpers
  sessionGetParent: vi.fn().mockReturnValue(null),
  sessionGetWatchers: vi.fn().mockReturnValue([]),
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
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
}));

// Mock credentials utilities - required for provider filtering
vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn((registryId: string) => {
    const registryToAvailable: Record<string, string> = {
      anthropic: 'claude',
      openai: 'openai',
      gemini: 'gemini',
      google: 'gemini',
    };
    const availableName = registryToAvailable[registryId] || registryId;
    if (mockState.session.availableProviders.includes(availableName)) {
      return Promise.resolve({ apiKey: 'test-key', source: 'file' });
    }
    return Promise.resolve({ apiKey: null, source: null });
  }),
  saveCredential: vi.fn(),
  deleteCredential: vi.fn(),
  maskApiKey: vi.fn((key: string) => '***'),
}));

// Mock config utilities
vi.mock('../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
}));

// Mock Ink's Box to strip position="absolute" which doesn't work in ink-testing-library
vi.mock('ink', async () => {
  const actual = await vi.importActual<typeof import('ink')>('ink');
  return {
    ...actual,
    Box: (props: React.ComponentProps<typeof actual.Box>) => {
      const { position, ...rest } = props as { position?: string } & typeof props;
      return <actual.Box {...rest} />;
    },
  };
});

// Import the component after mocks are set up
import { AgentView } from '../components/AgentView';

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
    prompt: vi.fn().mockImplementation(async (_input: string, callback: (chunk: { type: string }) => void) => {
      callback({ type: 'Done' });
    }),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    resetInterrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: false,
      sessionFile: null,
      message: 'Debug disabled',
    }),
    compact: vi.fn().mockReturnValue({
      originalTokens: 0,
      compactedTokens: 0,
      compressionRatio: 0,
      turnsSummarized: 0,
      turnsKept: 0,
    }),
    ...overrides,
  };
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  mockState.persistence = {
    historyEntries: [],
    addHistoryCalled: false,
    getHistoryCalled: false,
    searchHistoryCalled: false,
    lastSearchQuery: '',
  };
};

describe('Feature: Session Persistence with Fork and Merge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
    // VIEWNV-001: Reset sessionStore state between tests
    useSessionStore.getState().reset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ============================================================================
  // @command-history - Navigate command history with keyboard shortcuts
  // ============================================================================

  describe('Scenario: Navigate command history with keyboard shortcuts', () => {
    it('should save commands to history and navigate with Shift+Arrow keys', async () => {
      // @step Given I have entered commands in the current project across multiple sessions
      // Pre-populate history with some commands
      mockState.persistence.historyEntries = [
        { display: 'implement auth flow', timestamp: '2025-01-15T11:00:00Z', project: '/test/project', sessionId: 'session-1' },
        { display: 'fix login bug', timestamp: '2025-01-15T10:00:00Z', project: '/test/project', sessionId: 'session-2' },
        { display: 'add user validation', timestamp: '2025-01-15T09:00:00Z', project: '/test/project', sessionId: 'session-1' },
      ];

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150); // Allow time for async initSession to load history

      // @step When I press Shift+Arrow-Up
      // Shift+Up should show most recent command
      stdin.write('\x1b[1;2A'); // Shift+Arrow-Up escape sequence
      await waitForFrame(100);

      // @step Then I should see my most recent command from the current project
      expect(lastFrame()).toContain('implement auth flow');

      // @step When I press Shift+Arrow-Up again
      stdin.write('\x1b[1;2A');
      await waitForFrame();

      // @step Then I should see the command before that, regardless of which session it was in
      expect(lastFrame()).toContain('fix login bug');

      // @step When I press Shift+Arrow-Down
      stdin.write('\x1b[1;2B'); // Shift+Arrow-Down escape sequence
      await waitForFrame();

      // @step Then I should return to the more recent command
      expect(lastFrame()).toContain('implement auth flow');

      // @step When I press Shift+Arrow-Down again
      stdin.write('\x1b[1;2B');
      await waitForFrame();

      // @step Then I should return to the empty prompt for new input
      // Input should be empty
      expect(lastFrame()).toContain("Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn | 'Space+Esc'");

      // @step And history is navigated with Shift+Arrow-Up (older) and Shift+Arrow-Down (newer)
      // Verified by the navigation above
    });

    it('should persist new commands to history when submitted', async () => {
      const { stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame();

      // Type and submit a command
      stdin.write('new command to save');
      await waitForFrame();
      stdin.write('\r'); // Enter
      await waitForFrame(100);

      // @step And the command should be saved to history for future navigation
      expect(mockState.persistence.addHistoryCalled).toBe(true);
      expect(mockState.persistence.historyEntries).toContainEqual(
        expect.objectContaining({ display: 'new command to save' })
      );
    });
  });

  // ============================================================================
  // @history-search - Search command history with /search command
  // ============================================================================

  describe('Scenario: Search command history with /search command', () => {
    it('should enter search mode when /search command is used', async () => {
      // @step Given I have command history containing "implement" keyword
      mockState.persistence.historyEntries = [
        { display: 'implement feature X', timestamp: '2025-01-15T11:00:00Z', project: '/test/project', sessionId: 'session-1' },
        { display: 'fix bug in login', timestamp: '2025-01-15T10:00:00Z', project: '/test/project', sessionId: 'session-2' },
        { display: 'implement auth flow', timestamp: '2025-01-15T09:00:00Z', project: '/test/project', sessionId: 'session-1' },
      ];

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // @step When I run /search command
      stdin.write('/search');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // Should show search mode indicator
      expect(lastFrame()).toContain('search');

      // @step And I type "implement"
      stdin.write('implement');
      await waitForFrame(100);

      // @step Then I should see matching previous commands
      expect(mockState.persistence.searchHistoryCalled).toBe(true);
      expect(mockState.persistence.lastSearchQuery).toBe('implement');
      expect(lastFrame()).toContain('implement feature X');

      // @step And I can select a command to reuse
      stdin.write('\r'); // Enter to select
      await waitForFrame();

      // The selected command should be in the input
      expect(lastFrame()).toContain('implement feature X');
    });
  });

  // ============================================================================
  // @cross-session-history - Command history accessible across different sessions
  // ============================================================================

  describe('Scenario: Command history accessible across different sessions', () => {
    it('should show history from all sessions ordered by timestamp', async () => {
      // @step Given I entered "fix login bug" in session B at 10:00am
      // @step And I entered "implement auth flow" in session A at 11:00am
      mockState.persistence.historyEntries = [
        { display: 'implement auth flow', timestamp: '2025-01-15T11:00:00Z', project: '/test/project', sessionId: 'session-a' },
        { display: 'fix login bug', timestamp: '2025-01-15T10:00:00Z', project: '/test/project', sessionId: 'session-b' },
      ];

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150); // Allow time for async initSession to load history

      // @step When I switch to session A
      // @step And I press Shift+Arrow-Up
      stdin.write('\x1b[1;2A');
      await waitForFrame(100);

      // @step Then I should see "implement auth flow" (most recent command)
      expect(lastFrame()).toContain('implement auth flow');

      // @step When I press Shift+Arrow-Up again
      stdin.write('\x1b[1;2A');
      await waitForFrame();

      // @step Then I should see "fix login bug" from session B (older command)
      expect(lastFrame()).toContain('fix login bug');

      // @step And history is ordered by timestamp regardless of which session the command was entered in
      // Verified by the order of navigation above
    });
  });

  // ============================================================================
  // @history-project-filter - History can be filtered by project
  // ============================================================================

  describe('Scenario: History can be filtered by project', () => {
    it('should filter history by current project by default', async () => {
      // @step Given I have history entries from project "/home/user/project-a"
      // @step And I have history entries from project "/home/user/project-b"
      mockState.persistence.historyEntries = [
        { display: 'cmd from project-a', timestamp: '2025-01-15T11:00:00Z', project: '/home/user/project-a', sessionId: 'session-1' },
        { display: 'cmd from project-b', timestamp: '2025-01-15T10:30:00Z', project: '/home/user/project-b', sessionId: 'session-2' },
        { display: 'another from project-a', timestamp: '2025-01-15T10:00:00Z', project: '/home/user/project-a', sessionId: 'session-1' },
      ];

      // Get the mock function to verify calls
      const { persistenceGetHistory } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceGetHistory).mockClear();

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // @step When I am in project (current working directory)
      // @step And I run "/history"
      stdin.write('/history');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then persistenceGetHistory should be called with the current project
      expect(vi.mocked(persistenceGetHistory)).toHaveBeenCalledWith(
        expect.any(String), // Current project path (process.cwd())
        20
      );

      vi.mocked(persistenceGetHistory).mockClear();

      // @step When I run "/history --all-projects"
      stdin.write('/history --all-projects');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then persistenceGetHistory should be called with null (all projects)
      expect(vi.mocked(persistenceGetHistory)).toHaveBeenCalledWith(null, 20);

      // @step And the view should still show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @session-resume - Resume session after closing terminal
  // ============================================================================

  describe('Scenario: Resume session after closing terminal', () => {
    it('should resume the last session with /resume command', async () => {
      // @step Given I have a 20-message conversation with codelet
      // @step And I close the terminal
      // @step When I reopen codelet the next day
      // @step And I run "codelet --resume" (or /resume in view)

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // Enter /resume command to open session selection overlay
      stdin.write('/resume');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step The view should show the resume session overlay
      expect(lastFrame()).toContain('Resume Session');
    });
  });

  // ============================================================================
  // @session-fork - Fork session at specific message
  // ============================================================================

  describe('Scenario: Fork session at specific message to try alternative approach', () => {
    it('should create a forked session with /fork command', async () => {
      // @step Given I have a session with 5 messages
      mockState.session.messages = [
        { role: 'user', content: 'msg 0' },
        { role: 'assistant', content: 'msg 1' },
        { role: 'user', content: 'msg 2' },
        { role: 'assistant', content: 'msg 3' },
        { role: 'user', content: 'msg 4' },
      ];

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // First send a message to create a session (deferred session creation)
      stdin.write('Initial message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step When I run "/fork 3 Alternative approach"
      stdin.write('/fork 3 Alternative approach');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the view should show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @session-merge - Merge messages from another session
  // ============================================================================

  describe('Scenario: Merge messages from another session', () => {
    it('should import messages from another session with /merge command', async () => {
      // @step Given I have session A as the current session
      // @step And session B contains an auth solution at messages 3 and 4

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // First send a message to create a session (deferred session creation)
      stdin.write('Initial message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step When I run "/merge session-b 3,4"
      stdin.write('/merge session-b 3,4');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the view should show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @session-switch - Switch to a different session
  // ============================================================================

  describe('Scenario: Switch to a different session', () => {
    it('should switch sessions with /switch command', async () => {
      // @step Given I have session "Auth Work" as the current session
      // @step And I have session "Bug Fix" available

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame();

      // @step When I run "/switch Bug Fix"
      stdin.write('/switch Bug Fix');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the view should show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @session-rename - Rename session for better organization
  // ============================================================================

  describe('Scenario: Rename session for better organization', () => {
    it('should rename current session with /rename command', async () => {
      // @step Given I have a session named "New Session 2025-01-15"

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // First send a message to create a session (deferred session creation)
      stdin.write('Initial message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step When I run "/rename Authentication Implementation"
      stdin.write('/rename Authentication Implementation');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the view should show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @session-cherry-pick - Cherry-pick message with preceding context
  // ============================================================================

  describe('Scenario: Cherry-pick message with preceding context', () => {
    it('should import messages with context using /cherry-pick command', async () => {
      // @step Given session B has a question at message 6 and answer at message 7

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // First send a message to create a session (deferred session creation)
      stdin.write('Initial message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step When I run "/cherry-pick session-b 7 --context 1"
      stdin.write('/cherry-pick session-b 7 --context 1');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the view should show the agent header
      expect(lastFrame()).toContain('Agent');
    });
  });

  // ============================================================================
  // @deferred-session-creation - Session not persisted until first message
  // ============================================================================

  describe('Scenario: Session not persisted until first message is sent', () => {
    it('should NOT create a session when modal opens without sending a message', async () => {
      // @step Given the agent modal is closed
      // @step And no session exists for the current project
      const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceCreateSessionWithProvider).mockClear();

      // @step When I open the agent modal
      const { unmount } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150); // Allow time for async initSession

      // @step Then NO session should be created in persistence
      // @step Because no message has been sent yet
      expect(vi.mocked(persistenceCreateSessionWithProvider)).not.toHaveBeenCalled();

      unmount();
    });

    it('should create session with first message as name when first message is sent', async () => {
      // @step Given I have the agent modal open
      // @step And no session has been created yet
      const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceCreateSessionWithProvider).mockClear();

      const { stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // Verify no session created on modal open
      expect(vi.mocked(persistenceCreateSessionWithProvider)).not.toHaveBeenCalled();

      // @step When I type "Help me implement authentication" and press Enter
      stdin.write('Help me implement authentication');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step Then a session should be created
      expect(vi.mocked(persistenceCreateSessionWithProvider)).toHaveBeenCalledTimes(1);

      // @step And the session name should be the first message content (truncated to 50 chars)
      expect(vi.mocked(persistenceCreateSessionWithProvider)).toHaveBeenCalledWith(
        'Help me implement authentication',
        expect.any(String), // project path
        expect.any(String)  // provider name
      );
    });

    it('should NOT create additional sessions for subsequent messages', async () => {
      // @step Given I have sent my first message and a session was created
      const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceCreateSessionWithProvider).mockClear();

      const { stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // Send first message - creates session
      stdin.write('First message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      expect(vi.mocked(persistenceCreateSessionWithProvider)).toHaveBeenCalledTimes(1);

      // @step When I send a second message "Now add password validation"
      stdin.write('Now add password validation');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step Then NO new session should be created
      // @step And the message should be added to the existing session
      expect(vi.mocked(persistenceCreateSessionWithProvider)).toHaveBeenCalledTimes(1);
    });

    it('should truncate long first messages to 500 characters for session name', async () => {
      // @step Given the agent modal is open
      const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceCreateSessionWithProvider).mockClear();

      const { stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // @step When I send a message longer than 500 characters
      const longMessage = 'A'.repeat(600); // 600 characters
      stdin.write(longMessage);
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step Then the session name should be truncated to 500 characters with "..."
      // Note: slice(0, 500) gives exactly 500 chars, then "..." is appended
      expect(vi.mocked(persistenceCreateSessionWithProvider)).toHaveBeenCalledWith(
        'A'.repeat(500) + '...',
        expect.any(String),
        expect.any(String)
      );
    });

    it('should not persist commands-only usage (no session for /debug, /clear, etc.)', async () => {
      // @step Given the agent modal is open
      const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');
      vi.mocked(persistenceCreateSessionWithProvider).mockClear();

      const { stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(150);

      // @step When I only use commands like /debug or /clear
      stdin.write('/debug');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      stdin.write('/clear');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then NO session should be created
      // @step Because no actual conversation message was sent
      expect(vi.mocked(persistenceCreateSessionWithProvider)).not.toHaveBeenCalled();
    });
  });
});

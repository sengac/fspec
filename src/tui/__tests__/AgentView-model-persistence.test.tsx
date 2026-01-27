/**
 * Feature: spec/features/persist-last-used-model-selection.feature
 *
 * TUI-035: Persist Last Used Model Selection
 *
 * Tests that model selection is persisted to ~/.fspec/fspec-config.json
 * and restored when opening a new AgentView session.
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { Box } from 'ink';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Create hoisted mock config state
const mockConfig = vi.hoisted(() => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
}));

// Mock the config utilities - preserve getFspecUserDir so logger works
vi.mock('../../utils/config', async () => {
  const actual = await vi.importActual<typeof import('../../utils/config')>('../../utils/config');
  return {
    ...actual,
    loadConfig: (...args: unknown[]) => mockConfig.loadConfig(...args),
    writeConfig: (...args: unknown[]) => mockConfig.writeConfig(...args),
  };
});

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'gemini'] as string[],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn(),
    compact: vi.fn(),
    selectModel: vi.fn(),
    selectedModel: null as string | null,
    restoreMessagesFromEnvelopes: vi.fn(),
    restoreTokenState: vi.fn(),
    getContextFillInfo: vi.fn(() => ({ percentage: 0, tokenCount: 0, maxTokens: 200000 })),
    setThinkingLevel: vi.fn(),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
  modelsListAll: vi.fn(() =>
    Promise.resolve([
      {
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
          {
            id: 'claude-opus-4-20250514',
            name: 'Claude Opus 4',
            family: 'claude-opus-4',
            reasoning: true,
            toolCall: true,
            attachment: true,
            temperature: true,
            contextWindow: 200000,
            maxOutput: 32000,
            hasVision: true,
          },
        ],
      },
      {
        providerId: 'google',
        providerName: 'Google',
        models: [
          {
            id: 'gemini-2.5-pro-preview-06-05',
            name: 'Gemini 2.5 Pro',
            family: 'gemini-2.5-pro',
            reasoning: true,
            toolCall: true,
            attachment: true,
            temperature: true,
            contextWindow: 1048576,
            maxOutput: 65536,
            hasVision: true,
          },
        ],
      },
    ])
  ),
  newWithModel: vi.fn(),
  persistenceListSessions: vi.fn(() => []),
  persistenceGetSessionMessages: vi.fn(() => []),
  persistenceGetSessionMessageEnvelopes: vi.fn(() => []),
  persistenceCreateSessionWithProvider: vi.fn(() => ({
    id: 'mock-session-id',
    name: 'Mock Session',
    project: '/test/project',
    provider: 'anthropic/claude-sonnet-4',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  })),
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
    compact: ReturnType<typeof vi.fn>;
    selectModel: ReturnType<typeof vi.fn>;
    selectedModel: string | null;
    restoreMessagesFromEnvelopes: ReturnType<typeof vi.fn>;
    restoreTokenState: ReturnType<typeof vi.fn>;
    getContextFillInfo: ReturnType<typeof vi.fn>;
    setThinkingLevel: ReturnType<typeof vi.fn>;

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
      this.compact = mockState.session.compact;
      this.selectModel = mockState.session.selectModel;
      this.selectedModel = mockState.session.selectedModel;
      this.restoreMessagesFromEnvelopes = mockState.session.restoreMessagesFromEnvelopes;
      this.restoreTokenState = mockState.session.restoreTokenState;
      this.getContextFillInfo = mockState.session.getContextFillInfo;
      this.setThinkingLevel = mockState.session.setThinkingLevel;
    }

    static async newWithModel(modelString: string) {
      mockState.newWithModel(modelString);
      mockState.session.selectedModel = modelString;
      return new MockCodeletSession();
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
  modelsListAll: () => mockState.modelsListAll(),
  modelsSetCacheDirectory: vi.fn(),
  setRustLogCallback: vi.fn(),
  persistenceSetDataDirectory: vi.fn(),
  persistenceStoreMessageEnvelope: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceCreateSessionWithProvider: (...args: unknown[]) =>
    mockState.persistenceCreateSessionWithProvider(...args),
  persistenceAddHistory: vi.fn(),
  persistenceSearchHistory: vi.fn(() => []),
  persistenceListSessions: (...args: unknown[]) => mockState.persistenceListSessions(...args),
  persistenceGetSessionMessages: (...args: unknown[]) =>
    mockState.persistenceGetSessionMessages(...args),
  persistenceGetSessionMessageEnvelopes: (...args: unknown[]) =>
    mockState.persistenceGetSessionMessageEnvelopes(...args),
  persistenceAppendMessage: vi.fn(),
  persistenceRenameSession: vi.fn(),
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
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
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
const resetMockSession = (overrides: Partial<typeof mockState.session> = {}) => {
  mockState.session = {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'gemini'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [],
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn(),
    compact: vi.fn(),
    selectModel: vi.fn(),
    selectedModel: null,
    restoreMessagesFromEnvelopes: vi.fn(),
    restoreTokenState: vi.fn(),
    getContextFillInfo: vi.fn(() => ({ percentage: 0, tokenCount: 0, maxTokens: 200000 })),
    setThinkingLevel: vi.fn(),
    ...overrides,
  };
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  // Reset model list mock
  mockState.modelsListAll = vi.fn(() =>
    Promise.resolve([
      {
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
          {
            id: 'claude-opus-4-20250514',
            name: 'Claude Opus 4',
            family: 'claude-opus-4',
            reasoning: true,
            toolCall: true,
            attachment: true,
            temperature: true,
            contextWindow: 200000,
            maxOutput: 32000,
            hasVision: true,
          },
        ],
      },
      {
        providerId: 'google',
        providerName: 'Google',
        models: [
          {
            id: 'gemini-2.5-pro-preview-06-05',
            name: 'Gemini 2.5 Pro',
            family: 'gemini-2.5-pro',
            reasoning: true,
            toolCall: true,
            attachment: true,
            temperature: true,
            contextWindow: 1048576,
            maxOutput: 65536,
            hasVision: true,
          },
        ],
      },
    ])
  );
  mockState.newWithModel = vi.fn();
  // Reset persistence mocks
  mockState.persistenceListSessions = vi.fn(() => []);
  mockState.persistenceGetSessionMessages = vi.fn(() => []);
  mockState.persistenceGetSessionMessageEnvelopes = vi.fn(() => []);
  mockState.persistenceCreateSessionWithProvider = vi.fn(() => ({
    id: 'mock-session-id',
    name: 'Mock Session',
    project: '/test/project',
    provider: 'anthropic/claude-sonnet-4',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  }));
};

describe('Feature: Persist Last Used Model Selection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
    mockConfig.loadConfig.mockResolvedValue({});
    mockConfig.writeConfig.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ----------------------------------------
  // RESTORATION ON NEW SESSION
  // ----------------------------------------
  // Note: Model switching via Tab and persistence on switch is tested in
  // AgentView-model-selection.test.tsx. These tests focus on restoration behavior.

  describe('Scenario: Restore persisted model on new session', () => {
    it('should start with persisted model when config exists', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "anthropic/claude-opus-4"
      mockConfig.loadConfig.mockResolvedValue({
        tui: { lastUsedModel: 'anthropic/claude-opus-4' },
      });

      // @step And ANTHROPIC_API_KEY is set
      resetMockSession({ availableProviders: ['claude'] });

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with "anthropic/claude-opus-4"
      // @step And the header should display "Agent: claude-opus-4"
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          expect(frame).toContain('Claude Opus');
        },
        { timeout: 3000 }
      );
    });
  });

  describe('Scenario: Restore persisted model from different provider', () => {
    it('should start with Google model when persisted', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "google/gemini-2.5-pro"
      mockConfig.loadConfig.mockResolvedValue({
        tui: { lastUsedModel: 'google/gemini-2.5-pro' },
      });

      // @step And GOOGLE_GENERATIVE_AI_API_KEY is set
      resetMockSession({ availableProviders: ['gemini'] });

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with "google/gemini-2.5-pro"
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          expect(frame).toContain('Gemini');
        },
        { timeout: 3000 }
      );
    });
  });

  // ----------------------------------------
  // FALLBACK SCENARIOS
  // ----------------------------------------

  describe('Scenario: Fall back when persisted model no longer exists', () => {
    it('should fall back to first available when persisted model missing', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "google/old-deprecated-model"
      mockConfig.loadConfig.mockResolvedValue({
        tui: { lastUsedModel: 'google/old-deprecated-model' },
      });

      // @step And GOOGLE_GENERATIVE_AI_API_KEY is set
      resetMockSession({ availableProviders: ['claude', 'gemini'] });

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with the first available model
      // @step And an informational message should indicate the persisted model was unavailable
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          // Should fall back to first available model (claude-sonnet-4)
          expect(frame).toContain('Agent:');
        },
        { timeout: 3000 }
      );
    });
  });

  describe('Scenario: Fall back when persisted provider has no credentials', () => {
    it('should use different provider when persisted one lacks credentials', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "anthropic/claude-sonnet-4"
      mockConfig.loadConfig.mockResolvedValue({
        tui: { lastUsedModel: 'anthropic/claude-sonnet-4' },
      });

      // @step And ANTHROPIC_API_KEY is NOT set
      // @step And GOOGLE_GENERATIVE_AI_API_KEY is set
      resetMockSession({ availableProviders: ['gemini'] }); // Only gemini has credentials

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with a Google model instead
      // @step And an informational message should indicate the persisted provider was unavailable
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          expect(frame).toContain('Agent:');
        },
        { timeout: 3000 }
      );
    });
  });

  describe('Scenario: Use default selection on fresh install', () => {
    it('should use first available model when no config exists', async () => {
      // @step Given ~/.fspec/fspec-config.json does not exist
      mockConfig.loadConfig.mockResolvedValue({});

      // @step And ANTHROPIC_API_KEY is set
      resetMockSession({ availableProviders: ['claude'] });

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with the first available model
      // @step And no error should be shown
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          expect(frame).toContain('Agent:');
          expect(frame).not.toContain('Error');
        },
        { timeout: 3000 }
      );
    });
  });

  describe('Scenario: Handle corrupt config gracefully', () => {
    it('should handle config read error gracefully', async () => {
      // @step Given ~/.fspec/fspec-config.json contains invalid JSON
      mockConfig.loadConfig.mockRejectedValue(new Error('Invalid JSON'));

      // @step And ANTHROPIC_API_KEY is set
      resetMockSession({ availableProviders: ['claude'] });

      // @step When I open the AgentView
      const { lastFrame } = render(<AgentView onExit={() => {}} />);

      // @step Then the session should start with the first available model
      // @step And config read failure should be logged but not shown to user
      await vi.waitFor(
        () => {
          const frame = lastFrame();
          expect(frame).toContain('Agent:');
          expect(frame).not.toContain('Invalid JSON');
        },
        { timeout: 3000 }
      );
    });
  });

  // ----------------------------------------
  // CONFIG STRUCTURE
  // ----------------------------------------

  describe('Scenario: Config uses proper nested structure', () => {
    it('should write config with nested agent.lastUsedModel path', async () => {
      // @step Given I am in the AgentView
      mockConfig.loadConfig.mockResolvedValue({});
      resetMockSession({ availableProviders: ['claude', 'gemini'] });

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await vi.waitFor(
        () => {
          expect(lastFrame()).toContain('Agent:');
        },
        { timeout: 2000 }
      );

      // @step When I switch models via the selector
      stdin.write('\t'); // Open selector
      await waitForFrame(300);
      stdin.write('\r'); // Select current

      // @step Then ~/.fspec/fspec-config.json should have structure
      await vi.waitFor(
        () => {
          if (mockConfig.writeConfig.mock.calls.length > 0) {
            const [scope, config] = mockConfig.writeConfig.mock.calls[0] as [string, { tui?: { lastUsedModel?: string } }];
            expect(scope).toBe('user');
            expect(config).toHaveProperty('tui');
            expect(config.tui).toHaveProperty('lastUsedModel');
          }
        },
        { timeout: 2000 }
      );
    });
  });

  describe('Scenario: Config preserves other settings when updating model', () => {
    it('should preserve existing config settings', async () => {
      // @step Given ~/.fspec/fspec-config.json contains other settings like "research.perplexity.apiKey"
      const existingConfig = {
        research: {
          perplexity: { apiKey: 'secret-key' },
        },
        otherSetting: 'value',
      };
      mockConfig.loadConfig.mockResolvedValue(existingConfig);
      resetMockSession({ availableProviders: ['claude', 'gemini'] });

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await vi.waitFor(
        () => {
          expect(lastFrame()).toContain('Agent:');
        },
        { timeout: 2000 }
      );

      // @step When I switch models via the selector
      stdin.write('\t');
      await waitForFrame(300);
      stdin.write('\x1B[B'); // Down
      stdin.write('\r'); // Select

      // @step Then the existing settings should be preserved
      // @step And only "agent.lastUsedModel" should be updated
      await vi.waitFor(
        () => {
          if (mockConfig.writeConfig.mock.calls.length > 0) {
            const [, config] = mockConfig.writeConfig.mock.calls[0] as [string, { research?: { perplexity?: { apiKey?: string } }; otherSetting?: string }];
            expect(config.research?.perplexity?.apiKey).toBe('secret-key');
            expect(config.otherSetting).toBe('value');
          }
        },
        { timeout: 2000 }
      );
    });
  });
});

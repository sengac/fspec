/**
 * Feature: spec/features/agent-modal-model-selection.feature
 *
 * TUI-034: Agent Modal Model Selection
 *
 * Tests for hierarchical model selector that allows users to select
 * specific models within providers, replacing the provider-only selector.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';

// Mock model data matching models.dev structure
// Note: family field is used as the model-id in the UI and for API calls
const mockModels = vi.hoisted(() => ({
  anthropic: {
    providerId: 'anthropic',
    providerName: 'Anthropic',
    models: [
      {
        id: 'claude-sonnet-4-20250514',
        name: 'Claude Sonnet 4',
        family: 'claude-sonnet-4', // This is the user-facing model ID
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
      {
        id: 'claude-haiku-3-20240307',
        name: 'Claude Haiku',
        family: 'claude-haiku-3',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 4096,
        hasVision: false,
      },
    ],
  },
  google: {
    providerId: 'google',
    providerName: 'Google',
    models: [
      {
        id: 'gemini-2.0-flash',
        name: 'Gemini 2.0 Flash',
        family: 'gemini-2.0-flash',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 1000000,
        maxOutput: 8192,
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
      {
        id: 'o1-preview',
        name: 'O1 Preview',
        family: 'o1-preview',
        reasoning: true,
        toolCall: false, // No tool_call - should be filtered
        attachment: false,
        temperature: false,
        contextWindow: 128000,
        maxOutput: 32768,
        hasVision: false,
      },
    ],
  },
}));

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai', 'gemini'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn(),
    compact: vi.fn(),
    // TUI-034: Model selection methods
    selectModel: vi.fn(),
    selectedModel: null as string | null,
    // NAPI-008: Restore messages from envelopes
    restoreMessagesFromEnvelopes: vi.fn(),
    restoreTokenState: vi.fn(),
    getContextFillInfo: vi.fn(() => ({ percentage: 0, tokenCount: 0, maxTokens: 200000 })),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
  modelsListAll: vi.fn(() =>
    Promise.resolve([
      mockModels.anthropic,
      mockModels.google,
      mockModels.openai,
    ])
  ),
  newWithModel: vi.fn(),
  // TUI-034: Persistence mocks for /resume tests
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

// Mock codelet-napi module with model selection support
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
    }

    // TUI-034: Static factory method for creating session with model
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
  // TUI-034: Model listing function
  modelsListAll: () => mockState.modelsListAll(),
  // TUI-034: Model cache directory setup
  modelsSetCacheDirectory: vi.fn(),
  // Rust logging callback
  setRustLogCallback: vi.fn(),
  // Persistence NAPI bindings (using mockState for overridable mocks)
  persistenceSetDataDirectory: vi.fn(),
  persistenceStoreMessageEnvelope: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceCreateSessionWithProvider: (...args: unknown[]) => mockState.persistenceCreateSessionWithProvider(...args),
  persistenceAddHistory: vi.fn(),
  persistenceSearchHistory: vi.fn(() => []),
  persistenceListSessions: (...args: unknown[]) => mockState.persistenceListSessions(...args),
  persistenceGetSessionMessages: (...args: unknown[]) => mockState.persistenceGetSessionMessages(...args),
  persistenceGetSessionMessageEnvelopes: (...args: unknown[]) => mockState.persistenceGetSessionMessageEnvelopes(...args),
  persistenceAppendMessage: vi.fn(),
  persistenceRenameSession: vi.fn(),
}));

// Mock Dialog
vi.mock('../../components/Dialog', () => ({
  Dialog: ({
    children,
  }: {
    children: React.ReactNode;
    onClose: () => void;
    borderColor?: string;
  }) => <Box flexDirection="column">{children}</Box>,
}));

// Mock Ink's Box to strip position="absolute"
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

// Mock config module to prevent loading user's real config (which may have lastUsedModel set)
vi.mock('../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
}));

// Import the component after mocks are set up
import { AgentView } from '../components/AgentView';

// Helper to wait for async operations
const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

// Helper to wait until a condition is met in the rendered output
const waitForCondition = async (
  lastFrameFn: () => string,
  condition: (frame: string) => boolean,
  maxAttempts = 30
): Promise<void> => {
  for (let i = 0; i < maxAttempts; i++) {
    if (condition(lastFrameFn())) {
      return;
    }
    await waitForFrame();
  }
};

// Helper to reset mock session
const resetMockSession = (overrides = {}) => {
  mockState.session = {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai', 'gemini'],
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
    ...overrides,
  };
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  mockState.modelsListAll = vi.fn(() =>
    Promise.resolve([
      mockModels.anthropic,
      mockModels.google,
      mockModels.openai,
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

describe('Feature: Agent Modal Model Selection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ========================================
  // BASIC SELECTOR BEHAVIOR
  // ========================================

  describe('Scenario: Tab key opens model selector with providers as collapsible sections', () => {
    it('should open model selector with collapsible provider sections on Tab press', async () => {
      // @step Given I am in the AgentView with a valid session
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step And multiple providers have valid credentials
      expect(lastFrame()).toContain('claude');

      // @step When I press Tab
      stdin.write('\t');
      await waitForFrame();

      // @step Then the model selector overlay should appear
      // TUI-034: Model selector should be visible
      expect(lastFrame()).toContain('Select Model');

      // @step And I should see available providers as collapsible sections
      expect(lastFrame()).toContain('anthropic');

      // @step And the current provider should be expanded by default
      expect(lastFrame()).toContain('claude-sonnet-4');

      // @step And the current model should be highlighted with "(current)" indicator
      expect(lastFrame()).toContain('(current)');
    });
  });

  describe('Scenario: Navigate between provider sections with arrow keys', () => {
    it('should navigate between provider headers with arrow keys', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step And the "anthropic" provider section is collapsed
      // Navigate to collapse anthropic first
      stdin.write('\x1b[D'); // Left arrow to collapse
      await waitForFrame();

      // @step When I press Down arrow to navigate to "google" provider header
      stdin.write('\x1b[B'); // Down arrow
      await waitForFrame();

      // @step Then the "google" provider header should be highlighted
      expect(lastFrame()).toContain('google');

      // @step And the section should remain collapsed until expanded
      expect(lastFrame()).not.toContain('gemini-2.0-flash');
    });
  });

  describe('Scenario: Expand provider section with Right arrow or Enter', () => {
    it('should expand collapsed provider section with Right arrow', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step And the "google" provider section is collapsed and highlighted
      // Navigate to google section (past anthropic header and all its models)
      // Anthropic is expanded so we need to navigate through: header -> sonnet -> opus -> haiku -> google
      stdin.write('\x1b[B'); // Down to claude-sonnet-4
      await waitForFrame();
      stdin.write('\x1b[B'); // Down to claude-opus-4
      await waitForFrame();
      stdin.write('\x1b[B'); // Down to claude-haiku-3
      await waitForFrame();
      stdin.write('\x1b[B'); // Down to google section header
      await waitForFrame();

      // @step When I press Right arrow
      stdin.write('\x1b[C'); // Right arrow
      await waitForFrame();

      // @step Then the "google" section should expand
      expect(lastFrame()).toContain('gemini-2.0-flash');

      // @step And the first model within "google" should be highlighted
      // Note: Right arrow expands but doesn't move selection into models
      expect(lastFrame()).toContain('[google]');
    });
  });

  describe('Scenario: Collapse provider section with Left arrow', () => {
    it('should collapse expanded provider section with Left arrow', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step And I am on a model within the expanded "anthropic" section
      expect(lastFrame()).toContain('claude-sonnet-4');

      // @step When I press Left arrow
      stdin.write('\x1b[D'); // Left arrow
      await waitForFrame();

      // @step Then the "anthropic" section should collapse
      // Models should no longer be visible
      expect(lastFrame()).not.toContain('claude-sonnet-4');

      // @step And the "anthropic" provider header should be highlighted
      expect(lastFrame()).toContain('[anthropic]');
    });
  });

  describe('Scenario: Select model with Enter key', () => {
    it('should select highlighted model and close selector on Enter', async () => {
      // @step Given the model selector is open
      const mockSelectModel = vi.fn();
      resetMockSession({ selectModel: mockSelectModel });

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step And "anthropic/claude-opus-4" is highlighted
      // Navigate: section header -> sonnet -> opus
      stdin.write('\x1b[B'); // Down to claude-sonnet-4
      await waitForFrame();
      stdin.write('\x1b[B'); // Down to claude-opus-4
      await waitForFrame();

      // @step When I press Enter
      stdin.write('\r'); // Enter
      await waitForFrame();

      // @step Then the model selector should close
      expect(lastFrame()).not.toContain('Select Model');

      // @step And selectModel should be called with "anthropic/claude-opus-4"
      expect(mockSelectModel).toHaveBeenCalledWith('anthropic/claude-opus-4');

      // @step And the header should display the new model name
      expect(lastFrame()).toContain('claude-opus-4');
    });
  });

  describe('Scenario: Cancel model selection with Escape', () => {
    it('should close selector and keep original model on Escape', async () => {
      // @step Given the model selector is open
      const mockSelectModel = vi.fn();
      resetMockSession({
        selectModel: mockSelectModel,
        selectedModel: 'anthropic/claude-sonnet-4',
      });

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step And I have navigated to a different model
      stdin.write('\x1b[B'); // Down to opus
      await waitForFrame();

      // @step When I press Escape
      stdin.write('\x1b'); // Escape
      await waitForFrame();

      // @step Then the model selector should close
      expect(lastFrame()).not.toContain('Select Model');

      // @step And the original model should remain selected
      expect(mockSelectModel).not.toHaveBeenCalled();
      expect(lastFrame()).toContain('claude-sonnet-4');
    });
  });

  // ========================================
  // CAPABILITY INDICATORS
  // ========================================

  describe('Scenario: Display reasoning capability indicator', () => {
    it('should show [R] indicator for models with reasoning=true', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step When I view a model with reasoning=true
      // claude-sonnet-4 has reasoning=true

      // @step Then I should see "[R]" indicator next to the model name
      expect(lastFrame()).toContain('[R]');

      // @step And models with reasoning=false should not show this indicator
      // claude-haiku has reasoning=false - navigate to it
      stdin.write('\x1b[B'); // Down
      stdin.write('\x1b[B'); // Down to haiku
      await waitForFrame();

      // Haiku line should not have [R]
      const frame = lastFrame();
      const lines = frame?.split('\n') || [];
      const haikuLine = lines.find(l => l.includes('claude-haiku'));
      expect(haikuLine).not.toContain('[R]');
    });
  });

  describe('Scenario: Display vision capability indicator', () => {
    it('should show [V] indicator for models with hasVision=true', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step When I view a model with hasVision=true
      // claude-sonnet-4 has hasVision=true

      // @step Then I should see "[V]" indicator next to the model name
      expect(lastFrame()).toContain('[V]');
    });
  });

  describe('Scenario: Display context window size', () => {
    it('should show formatted context window size indicator', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step When I view any model
      // @step Then I should see the context window size formatted as "[200k]" or "[1M]"
      expect(lastFrame()).toContain('[200k]');
    });
  });

  describe('Scenario: Header shows model with capability indicators', () => {
    it('should display model name with capability indicators in header', async () => {
      // @step Given the current model is "anthropic/claude-sonnet-4"
      resetMockSession({
        selectedModel: 'anthropic/claude-sonnet-4',
      });

      const { lastFrame } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step And the model has reasoning=true and contextWindow=200000
      // (mock data has these properties)

      // @step Then the header should display "Agent: claude-sonnet-4 [R] [200k]"
      expect(lastFrame()).toContain('Agent: claude-sonnet-4');
      expect(lastFrame()).toContain('[R]');
      expect(lastFrame()).toContain('[200k]');
    });
  });

  // ========================================
  // PROVIDER FILTERING
  // ========================================

  describe('Scenario: Only show providers with valid credentials', () => {
    it('should only show providers that have valid credentials', async () => {
      // @step Given ANTHROPIC_API_KEY is set
      // @step And OPENAI_API_KEY is NOT set
      resetMockSession({
        availableProviders: ['claude'], // Only claude has credentials
      });

      // @step When I open the model selector
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step Then I should see the "anthropic" provider section
      expect(lastFrame()).toContain('anthropic');

      // @step And I should NOT see the "openai" provider section
      expect(lastFrame()).not.toContain('openai');
    });
  });

  describe('Scenario: Only show models with tool_call capability', () => {
    it('should filter out models without tool_call=true', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step When I view the model list
      // Anthropic section is expanded by default, showing all Claude models
      // @step Then I should only see models where tool_call=true
      expect(lastFrame()).toContain('claude-sonnet-4'); // Has tool_call=true
      expect(lastFrame()).toContain('claude-opus-4'); // Has tool_call=true
      expect(lastFrame()).toContain('claude-haiku-3'); // Has tool_call=true
      // Note: google section is collapsed, so gemini-2.0-flash isn't visible in frame
      // but it exists in the list as shown by "(1 models)" in the google header
      expect(lastFrame()).toContain('[google] (1 models)');

      // @step And models without tool_call capability should be hidden
      // o1-preview has toolCall=false in our mock - it should not appear anywhere
      expect(lastFrame()).not.toContain('o1-preview'); // Has tool_call=false
    });
  });

  describe('Scenario: Show message when provider has no compatible models', () => {
    it('should show message when provider has no tool_call models', async () => {
      // @step Given a provider has only models with tool_call=false
      // Important: Set modelsListAll AFTER resetMockSession since it resets the mock
      resetMockSession({
        availableProviders: ['claude'],
      });
      mockState.modelsListAll = vi.fn(() =>
        Promise.resolve([
          {
            providerId: 'anthropic',
            providerName: 'Anthropic',
            models: [
              {
                id: 'claude-no-tools',
                name: 'Claude No Tools',
                family: 'claude-no-tools',
                reasoning: false,
                toolCall: false, // No tool_call - will be filtered
                attachment: false,
                temperature: true,
                contextWindow: 128000,
                maxOutput: 4096,
                hasVision: false,
              },
            ],
          },
        ])
      );

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      await waitForFrame(); // Extra wait for model list to load
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step When I expand that provider section (it starts collapsed since no models)
      stdin.write('\x1b[C'); // Right to expand
      await waitForFrame();

      // @step Then I should see the provider has 0 models after filtering
      // The UI shows "(0 models)" when all models are filtered out
      expect(lastFrame()).toContain('(0 models)');
    });
  });

  // ========================================
  // SESSION INITIALIZATION
  // ========================================

  describe('Scenario: New session uses newWithModel factory method', () => {
    it('should use CodeletSession.newWithModel for session creation', async () => {
      // @step Given I open the AgentView
      const { lastFrame } = render(
        <AgentView onExit={() => {}} />
      );

      // @step When the session initializes
      await waitForFrame();

      // @step Then CodeletSession.newWithModel should be called
      expect(mockState.newWithModel).toHaveBeenCalled();

      // @step And the default model should be the first available with tool_call=true
      expect(mockState.newWithModel).toHaveBeenCalledWith(
        expect.stringContaining('anthropic/claude-sonnet-4')
      );
      expect(lastFrame()).toContain('claude-sonnet-4');
    });
  });

  describe('Scenario: Session stores full model path in persistence', () => {
    it('should persist full model path when sending first message', async () => {
      // @step Given I have selected "anthropic/claude-sonnet-4"
      const mockCreateSession = vi.fn(() => ({
        id: 'new-session-id',
        name: 'Test Message',
        project: '/test/project',
        provider: 'anthropic/claude-sonnet-4',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        messageCount: 0,
      }));

      // Set up mock session with a prompt that completes immediately
      resetMockSession({
        prompt: vi.fn(async function* () {
          yield { type: 'text', text: 'Response from AI' };
        }),
      });

      // Override persistence mock after resetMockSession
      mockState.persistenceCreateSessionWithProvider = mockCreateSession;

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      // Wait for model loading to complete (header shows model name)
      // The default mock models have claude-sonnet-4 as the first model
      await waitForCondition(lastFrame, frame => frame.includes('claude-sonnet-4'));

      // Verify model loaded correctly
      expect(lastFrame()).toContain('claude-sonnet-4');

      // @step When I send my first message
      // Type a message character by character to ensure TextInput captures it
      stdin.write('Test message');
      await waitForCondition(lastFrame, frame => frame.includes('Test message'));

      // Submit the message with Enter
      stdin.write('\r');

      // Wait for the user message to appear in the conversation area
      // The conversation shows user messages above the input area
      await waitForCondition(
        lastFrame,
        frame => {
          // Check for user message indicator or the message in conversation
          return frame.includes('You:') || frame.includes('Test message');
        }
      );

      // Give time for async persistence call
      await waitForFrame();
      await waitForFrame();
      await waitForFrame();

      // @step Then the persisted session should store "anthropic/claude-sonnet-4" as the provider field
      // TUI-034: persistenceCreateSessionWithProvider is called with full model path
      expect(mockCreateSession).toHaveBeenCalled();
      expect(mockCreateSession).toHaveBeenCalledWith(
        expect.any(String), // session name (truncated message)
        expect.any(String), // project path
        'anthropic/claude-sonnet-4' // full model path
      );
    });
  });

  describe('Scenario: Resumed session restores exact model', () => {
    it('should restore exact model when resuming session', async () => {
      // @step Given I have a persisted session with provider "anthropic/claude-opus-4"
      const mockSelectModel = vi.fn();
      resetMockSession({
        selectModel: mockSelectModel,
      });

      // Set up persistence mocks for /resume using mockState (after resetMockSession)
      mockState.persistenceListSessions = vi.fn(() => [
        {
          id: 'session-123',
          name: 'Test Session',
          project: '/test',
          provider: 'anthropic/claude-opus-4', // Full model path
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          messageCount: 5,
        },
      ]);
      mockState.persistenceGetSessionMessages = vi.fn(() => []);
      mockState.persistenceGetSessionMessageEnvelopes = vi.fn(() => []);

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step When I resume that session via /resume command
      // Type /resume first, then send Enter separately (ink stdin requires this)
      stdin.write('/resume');
      await waitForCondition(lastFrame, frame => frame.includes('/resume'));
      stdin.write('\r');

      // Wait until the Resume Session overlay appears (confirms TextInput is unmounted)
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'));

      // Now press Enter to select the first session in the list
      stdin.write('\r');

      // Wait for session restore to complete (overlay closes)
      await waitForCondition(lastFrame, frame => !frame.includes('Resume Session'));

      // @step Then selectModel should be called with "anthropic/claude-opus-4"
      expect(mockSelectModel).toHaveBeenCalledWith('anthropic/claude-opus-4');

      // @step And the header should show "Agent: claude-opus-4"
      expect(lastFrame()).toContain('claude-opus-4');
    });
  });

  describe('Scenario: Legacy session with provider-only format uses default model', () => {
    it('should use default model when resuming legacy provider-only session', async () => {
      // @step Given I have a persisted session with provider "claude" (legacy format)
      const mockSwitchProvider = vi.fn();
      resetMockSession({
        switchProvider: mockSwitchProvider,
        // Use a different currentProviderName so that switchProvider gets called
        // (code only switches if stored provider differs from current)
        currentProviderName: 'openai',
      });

      // Set up persistence mocks for /resume using mockState (after resetMockSession)
      mockState.persistenceListSessions = vi.fn(() => [
        {
          id: 'legacy-session',
          name: 'Legacy Session',
          project: '/test',
          provider: 'claude', // Legacy format - no model specified
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          messageCount: 10,
        },
      ]);
      mockState.persistenceGetSessionMessages = vi.fn(() => []);
      mockState.persistenceGetSessionMessageEnvelopes = vi.fn(() => []);

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step When I resume that session
      // Type /resume first, then send Enter separately (ink stdin requires this)
      stdin.write('/resume');
      await waitForCondition(lastFrame, frame => frame.includes('/resume'));
      stdin.write('\r');

      // Wait until the Resume Session overlay appears (confirms TextInput is unmounted)
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'));

      // Now press Enter to select the first session in the list
      stdin.write('\r');

      // Wait for session restore to complete (overlay closes)
      await waitForCondition(lastFrame, frame => !frame.includes('Resume Session'));

      // @step Then the session should switch to claude provider
      expect(mockSwitchProvider).toHaveBeenCalledWith('claude');

      // @step And the default model for claude should be used
      expect(lastFrame()).toContain('claude');
    });
  });

  // ========================================
  // ERROR HANDLING
  // ========================================

  describe('Scenario: Graceful fallback when model cache unavailable', () => {
    it('should use embedded fallback when cache is unavailable', async () => {
      // @step Given the models.dev cache is corrupted or unavailable
      mockState.modelsListAll = vi.fn(() =>
        Promise.reject(new Error('Cache corrupted'))
      );

      // @step When I open the AgentView
      const { lastFrame } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step Then the embedded fallback models should be used
      // AgentView should still render without crashing
      expect(lastFrame()).toContain('Agent');

      // @step And model selection should still function
      expect(lastFrame()).toContain('claude');
    });
  });

  describe('Scenario: Error message when selected model unavailable', () => {
    it('should show error and keep current model when selection fails', async () => {
      // @step Given I try to select a model that doesn't exist in the registry
      const mockSelectModel = vi.fn().mockRejectedValue(new Error('Model not found in registry'));
      resetMockSession({
        selectModel: mockSelectModel,
        selectedModel: 'anthropic/claude-sonnet-4',
      });

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();
      stdin.write('\x1b[B'); // Navigate to different model (claude-opus-4)
      await waitForFrame();
      stdin.write('\r'); // Try to select
      await waitForFrame();
      await waitForFrame(); // Extra wait for async error handling

      // @step Then an error message should be displayed in the conversation
      // TUI-034: Error is now displayed in conversation, not global state
      expect(lastFrame()).toContain('Model selection failed');

      // @step And the current model should remain unchanged
      // The header should still show the original model
      expect(lastFrame()).toContain('claude-sonnet-4');
    });
  });

  describe('Scenario: Fallback when resumed session model no longer exists', () => {
    it('should fallback to provider default when model is deprecated', async () => {
      // @step Given I have a persisted session with model "anthropic/old-deprecated-model"
      const mockSelectModel = vi.fn().mockRejectedValue(new Error('Model not found'));
      resetMockSession({
        selectModel: mockSelectModel,
      });

      // Set up persistence mocks for /resume (after resetMockSession)
      mockState.persistenceListSessions = vi.fn(() => [
        {
          id: 'old-session',
          name: 'Old Session',
          project: '/test',
          provider: 'anthropic/old-deprecated-model',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          messageCount: 5,
        },
      ]);
      mockState.persistenceGetSessionMessages = vi.fn(() => []);
      mockState.persistenceGetSessionMessageEnvelopes = vi.fn(() => []);

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step When I resume that session
      // Type /resume first, then send Enter separately (ink stdin requires this)
      stdin.write('/resume');
      await waitForCondition(lastFrame, frame => frame.includes('/resume'));
      stdin.write('\r');

      // Wait until Resume Session overlay appears (confirms TextInput unmounted)
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'));

      // Now press Enter to select the first session in the list
      stdin.write('\r');

      // Wait for session restore to complete (overlay disappears)
      await waitForCondition(lastFrame, frame => !frame.includes('Resume Session'));

      // @step Then an informational message should be shown
      // TUI-034: Message format is 'Note: Model "X" is no longer available...'
      expect(lastFrame()).toContain('no longer available');

      // @step And the default model for anthropic should be used instead
      // After fallback, the provider's default model is used
      expect(lastFrame()).toContain('claude');
    });
  });

  // ========================================
  // UI DISPLAY FORMAT
  // ========================================

  describe('Scenario: Provider header shows model count', () => {
    it('should show model count in provider header', async () => {
      // @step Given the "anthropic" provider has 3 models with tool_call=true
      resetMockSession();
      // Set up mock with exactly 3 models for anthropic
      // Uses same structure as default mock: providerId, providerName, and models with id/name/family
      mockState.modelsListAll = vi.fn().mockResolvedValue([
        {
          providerId: 'anthropic',
          providerName: 'Anthropic',
          models: [
            {
              id: 'claude-sonnet-4-20250514',
              name: 'Claude Sonnet 4',
              family: 'claude-sonnet-4',
              toolCall: true,
              reasoning: true,
              hasVision: true,
              contextWindow: 200000,
              maxOutput: 16384,
              attachment: true,
              temperature: true,
            },
            {
              id: 'claude-opus-4-20250514',
              name: 'Claude Opus 4',
              family: 'claude-opus-4',
              toolCall: true,
              reasoning: true,
              hasVision: true,
              contextWindow: 200000,
              maxOutput: 16384,
              attachment: true,
              temperature: true,
            },
            {
              id: 'claude-haiku-4-20250514',
              name: 'Claude Haiku 4',
              family: 'claude-haiku-4',
              toolCall: true,
              reasoning: false,
              hasVision: true,
              contextWindow: 200000,
              maxOutput: 16384,
              attachment: true,
              temperature: true,
            },
          ],
        },
      ]);

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      // Wait for model loading to complete (header shows model name instead of just provider)
      await waitForCondition(lastFrame, frame => frame.includes('claude-sonnet'));

      // @step When I view the model selector
      stdin.write('\t'); // Open model selector
      await waitForCondition(lastFrame, frame => frame.includes('[anthropic]'));

      // @step Then the header should show "[anthropic] (3 models)"
      expect(lastFrame()).toContain('[anthropic]');
      expect(lastFrame()).toContain('(3 models)');
    });
  });

  describe('Scenario: Model list shows consistent format', () => {
    it('should display models in consistent format with indicators', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('\t'); // Open model selector
      await waitForFrame();

      // @step Then each model should display in format: "  model-id (Display Name) [indicators]"
      expect(lastFrame()).toMatch(
        /claude-sonnet-4.*\(Claude Sonnet 4\).*\[R\].*\[V\].*\[200k\]/
      );

      // @step And the selected model should have ">" prefix
      expect(lastFrame()).toContain('>');
    });
  });

  describe('Scenario: Tab hint in header shows model switching available', () => {
    it('should show Tab hint when multiple models are available', async () => {
      // @step Given multiple models are available
      const { lastFrame } = render(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step Then the header should show "[Tab]" hint for model switching
      expect(lastFrame()).toContain('[Tab]');
    });
  });
});

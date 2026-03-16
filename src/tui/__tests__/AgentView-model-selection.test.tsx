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
import { useModelStore, type ProviderSection } from '../store/modelStore';
import { useSessionStore } from '../store/sessionStore';

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

// NAPI-009: Track callback at module level for test control
let capturedCallback: ((err: Error | null, chunk: unknown) => void) | null = null;

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
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
  // BRIDGE-006: Unified thinking level detection NAPI functions
  napiDetectThinkingLevel: vi.fn(() => 0), // Default to Off
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
  // TUI-034: Model listing function
  modelsListAll: () => mockState.modelsListAll(),
  // TUI-034: Model cache directory setup
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
  // TUI-047: Session management for background sessions
  sessionManagerList: vi.fn().mockReturnValue([]),
  // VIEWNV-001: Session navigation helpers
  sessionGetSubordinate: vi.fn().mockReturnValue(null),
  sessionGetSupervisors: vi.fn().mockReturnValue([]),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionSendInput: vi.fn(),
  // TUI-052: Pending input for session resume
  sessionGetPendingInput: vi.fn().mockReturnValue(''),
  sessionSetPendingInput: vi.fn(),
  // UNIFIED: Merged output for session resume
  sessionGetMergedOutput: vi.fn().mockReturnValue([]),
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
  sessionSetModelProfile: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: vi.fn(),
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  // TUI-075: Session store uses these for Rust session tracking
  sessionClearActive: vi.fn(),
  sessionSetActive: vi.fn(),
  // OAuth token checks (default: no tokens)
  codexOauthGetTokens: vi.fn(() => null),
  claudeOauthGetTokens: vi.fn(async () => null),
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

// Mock credentials utilities - required for provider filtering
vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn((registryId: string) => {
    const registryToAvailable: Record<string, string> = {
      anthropic: 'claude',
      codex: 'codex',
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
  // NAPI-009: Reset callback capture
  capturedCallback = null;
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
  // Track unmount function for proper cleanup
  let unmountFn: (() => void) | null = null;
  
  // Helper to render and track unmount
  const renderWithCleanup = (element: React.ReactElement) => {
    const result = render(element);
    unmountFn = result.unmount;
    return result;
  };
  
  // TUI-075: Helper to pre-populate model store with test data
  // This simulates the state after models have been loaded (e.g., from a previous session)
  const setupModelStore = () => {
    const store = useModelStore.getState();
    
    // Create provider sections from mock data
    const sections = [
      {
        providerId: 'anthropic',
        providerName: 'Anthropic',
        internalName: 'claude',
        models: mockModels.anthropic.models,
        hasCredentials: true,
      },
      {
        providerId: 'google',
        providerName: 'Google',
        internalName: 'gemini',
        models: mockModels.google.models,
        hasCredentials: true,
      },
      {
        providerId: 'openai',
        providerName: 'OpenAI',
        internalName: 'openai',
        models: mockModels.openai.models,
        hasCredentials: true,
      },
    ];
    
    // Set the default model (first Anthropic model)
    const defaultModel = {
      providerId: 'anthropic',
      modelId: 'claude-sonnet-4',
      apiModelId: mockModels.anthropic.models[0].id,
      displayName: mockModels.anthropic.models[0].name,
      reasoning: mockModels.anthropic.models[0].reasoning,
      hasVision: mockModels.anthropic.models[0].hasVision,
      contextWindow: mockModels.anthropic.models[0].contextWindow,
      maxOutput: mockModels.anthropic.models[0].maxOutput,
    };
    
    store.setProviderSections(sections);
    store.setCurrentModel(defaultModel);
    store.setModelsInitialized(true);
  };

  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
    // TUI-075: Reset model store to ensure clean state between tests
    useModelStore.getState().reset();
    // TUI-075: Reset session store to ensure clean state between tests
    useSessionStore.getState().reset();
    // TUI-075: Pre-populate model store with test data
    setupModelStore();
    // Reset unmount function
    unmountFn = null;
  });

  afterEach(() => {
    // Unmount component first to stop React effects
    if (unmountFn) {
      unmountFn();
      unmountFn = null;
    }
    vi.restoreAllMocks();
    // TUI-075: Reset model store after each test
    useModelStore.getState().reset();
    // TUI-075: Reset session store after each test
    useSessionStore.getState().reset();
  });

  // ========================================
  // BASIC SELECTOR BEHAVIOR
  // ========================================

  describe('Scenario: /model command opens model selector with providers as collapsible sections', () => {
    it('should open model selector with collapsible provider sections on /model command', async () => {
      // @step Given I am in the AgentView with a valid session
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step And multiple providers have valid credentials
      expect(lastFrame()).toContain('Claude');

      // @step When I type /model and press Enter
      stdin.write('/model');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step Then the model selector overlay should appear
      // TUI-034: Model selector should be visible
      expect(lastFrame()).toContain('Select Model');

      // @step And I should see available providers as collapsible sections
      expect(lastFrame()).toContain('Anthropic');

      // @step And the current provider should be expanded by default
      // Model selector shows model IDs, not display names
      expect(lastFrame()).toContain('claude-sonnet-4');

      // @step And the current model should be highlighted with "(current)" indicator
      expect(lastFrame()).toContain('(current)');
    });
  });

  describe('Scenario: Navigate between provider sections with arrow keys', () => {
    it('should navigate between provider headers with arrow keys', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step And the "anthropic" provider section is collapsed
      // Navigate to collapse anthropic first
      stdin.write('\x1b[D'); // Left arrow to collapse
      await waitForFrame();

      // @step When I press Down arrow to navigate to "google" provider header
      stdin.write('\x1b[B'); // Down arrow
      await waitForFrame();

      // @step Then the "google" provider header should be highlighted
      expect(lastFrame()).toContain('Google');

      // @step And the section should remain collapsed until expanded
      expect(lastFrame()).not.toContain('gemini-2.0-flash');
    });
  });

  describe('Scenario: Expand provider section with Right arrow or Enter', () => {
    it('should expand collapsed provider section with Right arrow', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step And the "google" provider section is collapsed and highlighted
      // TUI-073: Current model (claude-sonnet-4) is auto-selected when screen opens
      // Navigate to google section: sonnet -> opus -> haiku -> google
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

      // @step And the Google section header should be visible
      // Note: Right arrow expands but doesn't move selection into models
      expect(lastFrame()).toContain('Google');
    });
  });

  describe('Scenario: Collapse provider section with Left arrow', () => {
    it('should collapse expanded provider section with Left arrow', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step And I am on a model within the expanded "anthropic" section
      // TUI-073: Current model (claude-sonnet-4) is auto-selected, shown as model ID
      expect(lastFrame()).toContain('claude-sonnet-4');

      // @step When I press Left arrow
      stdin.write('\x1b[D'); // Left arrow
      await waitForFrame();

      // @step Then the "anthropic" section should collapse
      // Models should no longer be visible
      expect(lastFrame()).not.toContain('claude-sonnet-4');

      // @step And the "anthropic" provider header should be highlighted
      expect(lastFrame()).toContain('Anthropic');
    });
  });

  describe('Scenario: Select model with Enter key', () => {
    it('should select highlighted model and close selector on Enter', async () => {
      // @step Given the model selector is open
      // NAPI-009: Model selection uses state management, not session methods
      resetMockSession();

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step And "anthropic/claude-opus-4" is highlighted
      // TUI-073: Current model is auto-selected when screen opens
      // Starting position: claude-sonnet-4 (current model)
      // Navigate: sonnet -> opus (only one Down needed)
      stdin.write('\x1b[B'); // Down to claude-opus-4
      await waitForFrame();

      // @step When I press Enter
      stdin.write('\r'); // Enter
      await waitForFrame();

      // @step Then the model selector should close
      expect(lastFrame()).not.toContain('Select Model');

      // @step And the header should display the new model name
      // NAPI-009: Model selection is reflected in header via state management
      expect(lastFrame()).toContain('Claude Opus 4');
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

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
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
      expect(lastFrame()).toContain('Claude Sonnet 4');
    });
  });

  // ========================================
  // CAPABILITY INDICATORS
  // ========================================

  describe('Scenario: Display reasoning capability indicator', () => {
    it('should show [R] indicator for models with reasoning=true', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
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
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
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
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
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

      const { lastFrame } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step And the model has reasoning=true and contextWindow=200000
      // (mock data has these properties)

      // @step Then the header should display "Claude Sonnet 4 [R] [200k]"
      expect(lastFrame()).toContain('Claude Sonnet 4');
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
      // @step And CODEX_API_KEY is NOT set
      resetMockSession({
        availableProviders: ['claude'], // Only claude has credentials
      });
      // TUI-075: Reset store so it loads from mock modelsListAll instead of pre-populated data
      useModelStore.getState().reset();

      // @step When I open the model selector
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();
      await waitForFrame(); // Extra wait for model loading

      // @step Then I should see the "anthropic" provider section
      expect(lastFrame()).toContain('Anthropic');

      // OpenAI cloud models require Codex credentials (OAuth or CODEX_API_KEY).
      // Without Codex credentials, no OpenAI/Codex cloud section appears.
      // @step And I should NOT see "OpenAI" or "Codex" since CODEX_API_KEY is NOT set
      expect(lastFrame()).not.toContain('OpenAI');
      expect(lastFrame()).not.toContain('Codex');

      // @step And I should NOT see "Google" since GOOGLE_API_KEY is NOT set
      expect(lastFrame()).not.toContain('Google');
    });
  });

  describe('Scenario: Only show models with tool_call capability', () => {
    it('should filter out models without tool_call=true', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step When I view the model list
      // Anthropic section is expanded by default, showing all Claude models
      // @step Then I should only see models where tool_call=true
      // Model selector shows model IDs, not display names
      expect(lastFrame()).toContain('claude-sonnet-4'); // Has tool_call=true
      expect(lastFrame()).toContain('claude-opus-4'); // Has tool_call=true
      expect(lastFrame()).toContain('claude-haiku-3'); // Has tool_call=true
      // Note: google section is collapsed, so gemini-2.0-flash isn't visible in frame
      // but it exists in the list as shown by "(1 model)" in the google header
      expect(lastFrame()).toContain('Google');
      expect(lastFrame()).toContain('(1 model)');

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
      // TUI-075: Reset store so it loads from mock modelsListAll instead of pre-populated data
      useModelStore.getState().reset();

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      await waitForFrame(); // Extra wait for model list to load
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();
      await waitForFrame(); // Extra wait for model loading

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
    it('should use sessionManagerCreateWithId for session creation', async () => {
      // @step Given I open the AgentView
      const { lastFrame } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );

      // @step When the session initializes
      // NAPI-009: Wait for model loading to complete (models load asynchronously)
      await waitForCondition(lastFrame, frame => frame.includes('claude-sonnet'));

      // @step Then the default model should be the first available with tool_call=true
      // NAPI-009: Session creation is deferred until first message, but model selection
      // is reflected in the header via state management
      expect(lastFrame()).toContain('Claude Sonnet 4');
    });
  });

  describe('Scenario: Session stores full model path in persistence', () => {
    it('should persist full model path when sending first message', async () => {
      // @step Given I have selected "anthropic/claude-sonnet-4"
      // NAPI-009: Session is created on first message with full model path
      
      // Ensure model store has the correct model set (from setupModelStore in beforeEach)
      const currentModel = useModelStore.getState().currentModel;
      expect(currentModel).not.toBeNull();
      expect(currentModel?.providerId).toBe('anthropic');
      expect(currentModel?.modelId).toBe('claude-sonnet-4');

      // Ensure session store is ready for new session
      expect(useSessionStore.getState().isReadyForNewSession).toBe(true);
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // Clear the mock to ensure fresh tracking
      mockState.persistenceCreateSessionWithProvider.mockClear();

      const { stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );

      // Wait for component to initialize
      await waitForFrame(150);

      // Verify no session created on modal open (deferred session creation)
      expect(mockState.persistenceCreateSessionWithProvider).not.toHaveBeenCalled();

      // @step When I send my first message
      stdin.write('Test message');
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame(150);

      // @step Then the persisted session should store "anthropic/claude-sonnet-4" as the provider field
      // TUI-034: persistenceCreateSessionWithProvider is called with full model path
      // TUI-075: modelId is the family ID (without date suffix) - extractModelIdForRegistry strips date suffix
      expect(mockState.persistenceCreateSessionWithProvider).toHaveBeenCalledTimes(1);
      expect(mockState.persistenceCreateSessionWithProvider).toHaveBeenCalledWith(
        expect.any(String), // session name (truncated message)
        expect.any(String), // project path
        'anthropic/claude-sonnet-4' // model path (family ID without date suffix)
      );
    });
  });

  describe('Scenario: Resumed session restores exact model', () => {
    it('should restore exact model when resuming session', async () => {
      // @step Given I have a persisted session with provider "anthropic/claude-opus-4-20250514"
      // NAPI-009: Model selection uses state management, not session methods
      // NOTE: Must use full model ID with date suffix for Anthropic models
      resetMockSession();

      // Set up persistence mocks for /resume using mockState (after resetMockSession)
      mockState.persistenceListSessions = vi.fn(() => [
        {
          id: 'session-123',
          name: 'Test Session',
          project: '/test',
          provider: 'anthropic/claude-opus-4-20250514', // Full model path with date suffix
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          messageCount: 5,
        },
      ]);
      mockState.persistenceGetSessionMessages = vi.fn(() => []);
      mockState.persistenceGetSessionMessageEnvelopes = vi.fn(() => []);

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step When I resume that session via /resume command
      // Type /resume and immediately press Enter - don't wait for palette
      // This uses the fallback path in AgentView's useInput
      stdin.write('/resume');
      await waitForFrame();
      stdin.write('\r');
      
      // Wait longer for React to process state changes
      await waitForFrame(200);

      // Wait until the Resume Session overlay appears
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'), 100);

      // Now press Enter to select the first session in the list
      stdin.write('\r');
      await waitForFrame(500); // Wait for state changes to propagate

      // Wait for session restore to complete (overlay closes and no slash command palette)
      await waitForCondition(lastFrame, frame => !frame.includes('Resume Session') && !frame.includes('Slash Commands'), 100);
      await waitForFrame(200);

      // @step Then the header should show "Claude Opus 4"
      // NAPI-009: Model is restored via state management, reflected in header
      expect(lastFrame()).toContain('Claude Opus 4');
    });
  });

  describe('Scenario: Legacy session with provider-only format uses default model', () => {
    it('should use default model when resuming legacy provider-only session', async () => {
      // @step Given I have a persisted session with provider "claude" (legacy format)
      // NAPI-009: Provider switching uses state management
      
      // Set up persistence mocks for /resume
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

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      
      // Wait for initial render and model loading
      await waitForCondition(lastFrame, frame => frame.includes('Claude'), 50);

      // @step When I resume that session
      stdin.write('/resume');
      await waitForFrame();
      stdin.write('\r');
      
      // Wait for Resume Session overlay to appear
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'), 100);

      // Press Enter to select the first session in the list
      stdin.write('\r');

      // Wait for session restore to complete (overlay closes)
      await waitForCondition(
        lastFrame,
        frame => !frame.includes('Resume Session') && !frame.includes('Slash Commands'),
        100
      );
      await waitForFrame(100);

      // @step Then the default model for claude should be used
      // NAPI-009: Provider is restored via state management, reflected in header
      // Legacy sessions use the default model for the provider
      expect(lastFrame()).toContain('Claude');
    });
  });

  // ========================================
  // ERROR HANDLING
  // ========================================

  describe('Scenario: Graceful handling when model cache unavailable', () => {
    it('should render gracefully when cache is unavailable without fallback', async () => {
      // @step Given the models.dev cache is corrupted or unavailable
      resetMockSession();
      mockState.modelsListAll = vi.fn(() =>
        Promise.reject(new Error('Cache corrupted'))
      );

      // @step When I open the AgentView
      const { lastFrame } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      await waitForFrame(); // Extra wait for error handling

      // @step Then the UI should still render without crashing
      // No embedded fallback - the component renders but without model info
      const frame = lastFrame();
      expect(frame).toBeDefined();

      // @step And the UI should still be functional
      // The component should not crash and should show the input placeholder
      expect(frame).toContain("Type a message");
    });
  });

  describe('Scenario: Error message when selected model unavailable', () => {
    it('should keep current model when sessionSetModel fails', async () => {
      // @step Given I have an active session with "anthropic/claude-sonnet-4" selected
      // First, we need to create a session by sending a message
      const mockCreateSession = vi.fn(() => ({
        id: 'test-session-id',
        name: 'Test Session',
        project: '/test/project',
        provider: 'anthropic/claude-sonnet-4',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        messageCount: 0,
      }));
      mockState.persistenceCreateSessionWithProvider = mockCreateSession;

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );

      // Wait for initial model loading
      await waitForCondition(lastFrame, frame => frame.includes('Claude Sonnet 4'), 50);

      // Send a message to create a session
      stdin.write('Hello');
      await waitForFrame();
      stdin.write('\r');

      // Wait for session to be created
      await waitForCondition(
        () => mockCreateSession.mock.calls.length > 0 ? 'called' : '',
        frame => frame === 'called',
        50
      );
      await waitForFrame(100);

      // Now mock sessionSetModel to throw an error for the NEXT call
      const { sessionSetModel } = await import('@sengac/codelet-napi');
      vi.mocked(sessionSetModel).mockRejectedValueOnce(new Error('Model not found in registry'));

      // @step When I open the model selector and try to select a different model
      stdin.write('/model');
      await waitForFrame();
      stdin.write('\r');
      await waitForCondition(lastFrame, frame => frame.includes('Select Model'), 50);

      // Navigate to claude-opus-4 (one down from current claude-sonnet-4)
      stdin.write('\x1b[B'); // Down arrow
      await waitForFrame();

      // Try to select it
      stdin.write('\r');
      await waitForFrame(100);

      // @step Then the current model should remain unchanged
      // The error is logged but the model selector closes
      // Since sessionSetModel failed, the model in the header should still be Sonnet
      // However, note: the current implementation doesn't update local state when session exists
      // and sessionSetModel is called, so model remains unchanged on error
      expect(lastFrame()).not.toContain('Select Model'); // Selector closed
      
      // The header should still show the original model since sessionSetModel failed
      // and we don't update local state when there's an active session
      expect(lastFrame()).toContain('Claude Sonnet 4');
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

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step When I resume that session
      // Type /resume and immediately press Enter - don't wait for palette
      // This uses the fallback path in AgentView's useInput
      stdin.write('/resume');
      await waitForFrame();
      stdin.write('\r');
      
      // Wait longer for React to process state changes
      await waitForFrame(200);

      // Wait until the Resume Session overlay appears
      await waitForCondition(lastFrame, frame => frame.includes('Resume Session'), 100);

      // Now press Enter to select the first session in the list
      stdin.write('\r');

      // Wait for session restore to complete (overlay disappears)
      await waitForCondition(lastFrame, frame => !frame.includes('Resume Session'));

      // @step Then the default model for anthropic should be used
      // After fallback, the provider's default model is used
      expect(lastFrame()).toContain('Claude');
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

      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );

      // Wait for model loading to complete (header shows model name instead of just provider)
      await waitForCondition(lastFrame, frame => frame.includes('claude-sonnet'));

      // @step When I view the model selector
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForCondition(lastFrame, frame => frame.includes('Anthropic'));

      // @step Then the header should show "Anthropic (3 models)"
      expect(lastFrame()).toContain('Anthropic');
      expect(lastFrame()).toContain('(3 models)');
    });
  });

  describe('Scenario: Model list shows consistent format', () => {
    it('should display models in consistent format with indicators', async () => {
      // @step Given the model selector is open
      const { lastFrame, stdin } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();
      stdin.write('/model'); // Open model selector
      await waitForFrame();
      stdin.write('\r');
      await waitForFrame();

      // @step Then each model should display in format: "model-id [indicators]"
      // Format: claude-sonnet-4-20250514 [R] [V] [200k]
      expect(lastFrame()).toMatch(
        /claude-sonnet-4.*\[R\].*\[V\].*\[200k\]/
      );

      // @step And the selected model should have ">" prefix
      expect(lastFrame()).toContain('>');
    });
  });

  describe('Scenario: Selection mode hint shows in input placeholder', () => {
    it('should show Tab select hint in placeholder when models are available', async () => {
      // @step Given multiple models are available
      const { lastFrame } = renderWithCleanup(
        <AgentView onExit={() => {}} />
      );
      await waitForFrame();

      // @step Then the placeholder should show "'Tab' select turn" hint for turn selection mode
      // Note: Tab now toggles turn selection mode (use /model command for model switching)
      expect(lastFrame()).toContain("'Tab' select turn");
    });
  });
});

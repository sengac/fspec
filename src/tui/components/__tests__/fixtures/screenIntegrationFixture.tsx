/**
 * Screen Integration Test Fixture
 *
 * TUI-075: Full integration test fixture for screen component integration.
 *
 * This fixture COMPOSES existing fixtures and provides:
 * - Real ModelSelectorScreen and ProviderSettingsScreen components
 * - Real hooks (useModelSelectorState, useProviderSettingsState)
 * - NAPI network boundary mocking only
 * - Screen switching state simulation
 *
 * SOLID: Single Responsibility - Screen integration test setup only
 * DRY: Composes existing fixtures instead of duplicating logic
 * COMPOSABLE: Can be extended for specific test scenarios
 */

import { vi } from 'vitest';
import React, { useState, useCallback } from 'react';

import {
  createHomeDirectoryFixture,
  type HomeDirectoryEnv,
} from '../../../../test-helpers/home-directory-fixture';
import type { NapiProviderModels } from '@sengac/codelet-napi';
import type { ProfileConfig } from '../../../../utils/provider-config';
import type { ModelSelection } from '../../../types/provider';
import { useModelStore } from '../../../store/modelStore';

// Import from single source of truth for NAPI model data
import {
  createNapiModelInfo,
  createAnthropicNapiModels,
  createOpenAiNapiModels,
  createDefaultCloudProviders,
} from '../../../../test-helpers/napi-model-fixtures';

// Re-export NAPI builders for fixture consumers
export {
  createNapiModelInfo,
  createAnthropicNapiModels,
  createOpenAiNapiModels,
  createDefaultCloudProviders,
} from '../../../../test-helpers/napi-model-fixtures';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Screen visibility state (simulates AgentView's coordination state)
 */
export interface ScreenState {
  showModelSelector: boolean;
  showSettingsTab: boolean;
  currentModel: ModelSelection | null;
}

/**
 * Mock NAPI response configuration
 */
export interface NapiMockConfig {
  /** Cloud providers response from modelsListAll */
  cloudProviders: NapiProviderModels[];
  /** Local models by baseUrl (for modelsListLocalOpenai) */
  localServerModels: Map<string, string[] | Error>;
  /** Whether modelsRefreshCache should succeed */
  refreshSuccess: boolean;
  /** Connection test results by provider/profile */
  connectionResults: Map<string, { success: boolean; error?: string }>;
}

/**
 * Callback tracking for assertions
 */
export interface IntegrationCallbackTracker {
  /** Model selection events */
  onSelectModel: {
    calls: ModelSelection[];
    mock: ReturnType<typeof vi.fn>;
  };
  /** Model screen close events */
  modelScreenClose: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
  /** Settings screen close events */
  settingsScreenClose: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
  /** Screen switch events */
  screenSwitch: {
    calls: Array<{ from: 'model' | 'settings'; to: 'model' | 'settings' }>;
  };
  /** Session model updates */
  sessionModelUpdate: {
    calls: Array<{ sessionId: string; providerId: string; modelId: string }>;
    mock: ReturnType<typeof vi.fn>;
  };
}

/**
 * Screen integration fixture
 */
export interface ScreenIntegrationFixture {
  /** Test environment (HOME, config paths) */
  env: HomeDirectoryEnv;

  /** NAPI mock configuration */
  napiConfig: NapiMockConfig;

  /** Callback tracker for assertions */
  callbacks: IntegrationCallbackTracker;

  /** Current screen state */
  screenState: ScreenState;

  // ---- Delegated from HomeDirectoryFixture ----

  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;

  createCredential: (providerId: string, apiKey: string) => Promise<void>;

  // ---- NAPI Configuration ----

  configureNapi: (config: Partial<NapiMockConfig>) => void;
  setLocalServerModels: (baseUrl: string, models: string[] | Error) => void;
  setConnectionResult: (
    providerId: string,
    profileName: string | undefined,
    success: boolean,
    error?: string
  ) => void;

  // ---- NAPI Mocks ----

  modelsListAllMock: ReturnType<typeof vi.fn>;
  modelsListLocalOpenaiMock: ReturnType<typeof vi.fn>;
  modelsRefreshCacheMock: ReturnType<typeof vi.fn>;
  testProviderConnectionMock: ReturnType<typeof vi.fn>;
  sessionSetModelMock: ReturnType<typeof vi.fn>;

  // ---- Screen State Control ----

  /** Simulate /model command */
  openModelSelector: () => void;
  /** Simulate /provider command */
  openProviderSettings: () => void;
  /** Reset screen state */
  closeAllScreens: () => void;

  // ---- Lifecycle ----

  reset: () => Promise<void>;
  resetCallbacks: () => void;
  cleanup: () => Promise<void>;
  waitForModelsLoaded: (timeout?: number) => Promise<void>;
}

// =============================================================================
// NAPI CONFIG FACTORY
// =============================================================================

/**
 * Creates default NAPI mock configuration
 */
function createDefaultNapiConfig(): NapiMockConfig {
  return {
    cloudProviders: createDefaultCloudProviders(),
    localServerModels: new Map(),
    refreshSuccess: true,
    connectionResults: new Map(),
  };
}

// =============================================================================
// CALLBACK TRACKER FACTORY
// =============================================================================

function createCallbackTracker(): IntegrationCallbackTracker {
  const tracker: IntegrationCallbackTracker = {
    onSelectModel: {
      calls: [],
      mock: vi.fn(),
    },
    modelScreenClose: {
      calls: 0,
      mock: vi.fn(),
    },
    settingsScreenClose: {
      calls: 0,
      mock: vi.fn(),
    },
    screenSwitch: {
      calls: [],
    },
    sessionModelUpdate: {
      calls: [],
      mock: vi.fn(),
    },
  };

  // Connect mocks to call trackers
  tracker.onSelectModel.mock.mockImplementation((model: ModelSelection) => {
    tracker.onSelectModel.calls.push(model);
  });
  tracker.modelScreenClose.mock.mockImplementation(() => {
    tracker.modelScreenClose.calls++;
  });
  tracker.settingsScreenClose.mock.mockImplementation(() => {
    tracker.settingsScreenClose.calls++;
  });
  tracker.sessionModelUpdate.mock.mockImplementation(
    (sessionId: string, providerId: string, modelId: string) => {
      tracker.sessionModelUpdate.calls.push({ sessionId, providerId, modelId });
    }
  );

  return tracker;
}

// =============================================================================
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a screen integration fixture for TUI-075 testing.
 *
 * This fixture:
 * - Composes HomeDirectoryFixture for real file system operations
 * - Provides NAPI mock functions for network boundary ONLY
 * - Uses REAL ModelSelectorScreen and ProviderSettingsScreen components
 * - Uses REAL hooks (useModelSelectorState, useProviderSettingsState)
 * - Simulates AgentView's screen coordination state
 *
 * @example
 * ```typescript
 * describe('Screen Integration', () => {
 *   let fixture: ScreenIntegrationFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createScreenIntegrationFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should switch screens with Tab', async () => {
 *     await fixture.createCredential('anthropic', 'test-key');
 *     fixture.openModelSelector();
 *     // render and test...
 *   });
 * });
 * ```
 */
export async function createScreenIntegrationFixture(
  testName: string
): Promise<ScreenIntegrationFixture> {
  // ========================================
  // Compose HomeDirectoryFixture
  // ========================================

  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-screen-integration',
  });

  // TUI-075: Reset model store to ensure clean state for each test
  useModelStore.getState().reset();

  // ========================================
  // Screen State
  // ========================================

  const screenState: ScreenState = {
    showModelSelector: false,
    showSettingsTab: false,
    currentModel: null,
  };

  // ========================================
  // NAPI Mock Configuration
  // ========================================

  const napiConfig = createDefaultNapiConfig();

  // Create mock functions
  const modelsListAllMock = vi.fn();
  const modelsListLocalOpenaiMock = vi.fn();
  const modelsRefreshCacheMock = vi.fn();
  const testProviderConnectionMock = vi.fn();
  const sessionSetModelMock = vi.fn();

  // Configure mock implementations
  const updateMockImplementations = () => {
    modelsListAllMock.mockImplementation(async () => {
      return napiConfig.cloudProviders;
    });

    modelsListLocalOpenaiMock.mockImplementation(async (baseUrl: string) => {
      const result = napiConfig.localServerModels.get(baseUrl);
      if (result instanceof Error) {
        throw result;
      }
      return result || [];
    });

    modelsRefreshCacheMock.mockImplementation(async () => {
      if (!napiConfig.refreshSuccess) {
        throw new Error('Refresh failed');
      }
      return undefined;
    });

    testProviderConnectionMock.mockImplementation(async (providerId: string) => {
      const result = napiConfig.connectionResults.get(providerId);
      if (result) {
        return result;
      }
      return { success: true };
    });

    sessionSetModelMock.mockImplementation(
      async (sessionId: string, providerId: string, modelId: string) => {
        callbacks.sessionModelUpdate.mock(sessionId, providerId, modelId);
        return undefined;
      }
    );
  };

  updateMockImplementations();

  // ========================================
  // Callback Tracker
  // ========================================

  let callbacks = createCallbackTracker();

  // ========================================
  // NAPI Configuration Helpers
  // ========================================

  const configureNapi = (config: Partial<NapiMockConfig>): void => {
    if (config.cloudProviders !== undefined) {
      napiConfig.cloudProviders = config.cloudProviders;
    }
    if (config.localServerModels !== undefined) {
      napiConfig.localServerModels = config.localServerModels;
    }
    if (config.refreshSuccess !== undefined) {
      napiConfig.refreshSuccess = config.refreshSuccess;
    }
    if (config.connectionResults !== undefined) {
      napiConfig.connectionResults = config.connectionResults;
    }
    updateMockImplementations();
  };

  const setLocalServerModels = (
    baseUrl: string,
    models: string[] | Error
  ): void => {
    napiConfig.localServerModels.set(baseUrl, models);
    updateMockImplementations();
  };

  const setConnectionResult = (
    providerId: string,
    profileName: string | undefined,
    success: boolean,
    error?: string
  ): void => {
    const key = profileName ? `${providerId}:${profileName}` : providerId;
    napiConfig.connectionResults.set(key, { success, error });
    updateMockImplementations();
  };

  // ========================================
  // Screen State Control
  // ========================================

  const openModelSelector = (): void => {
    screenState.showModelSelector = true;
    screenState.showSettingsTab = false;
  };

  const openProviderSettings = (): void => {
    screenState.showModelSelector = false;
    screenState.showSettingsTab = true;
  };

  const closeAllScreens = (): void => {
    screenState.showModelSelector = false;
    screenState.showSettingsTab = false;
  };

  // ========================================
  // Lifecycle
  // ========================================

  const resetCallbacks = (): void => {
    callbacks = createCallbackTracker();
  };

  const reset = async (): Promise<void> => {
    // Reset HOME directory fixture
    await homeFixture.reset();

    // TUI-075: Reset model store to clear any state from previous tests
    useModelStore.getState().reset();

    // Reset NAPI config
    napiConfig.cloudProviders = createDefaultCloudProviders();
    napiConfig.localServerModels.clear();
    napiConfig.refreshSuccess = true;
    napiConfig.connectionResults.clear();

    // Clear mocks
    modelsListAllMock.mockClear();
    modelsListLocalOpenaiMock.mockClear();
    modelsRefreshCacheMock.mockClear();
    testProviderConnectionMock.mockClear();
    sessionSetModelMock.mockClear();

    updateMockImplementations();

    // Reset screen state
    closeAllScreens();
    screenState.currentModel = null;

    // Reset callbacks
    resetCallbacks();
  };

  const cleanup = async (): Promise<void> => {
    // TUI-075: Reset model store before cleanup to prevent state leakage
    useModelStore.getState().reset();
    await homeFixture.cleanup();
  };

  const waitForModelsLoaded = async (timeout = 2000): Promise<void> => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeout) {
      if (modelsListAllMock.mock.calls.length > 0) {
        // Give a small delay for state to update
        await new Promise(resolve => setTimeout(resolve, 50));
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 10));
    }
    throw new Error(`Timeout waiting for models to load after ${timeout}ms`);
  };

  return {
    // Delegate from HomeDirectoryFixture
    env: homeFixture.env,
    createProfile: homeFixture.createProfile,
    createCredential: homeFixture.createCredential,

    // Integration-specific
    napiConfig,
    callbacks,
    screenState,
    configureNapi,
    setLocalServerModels,
    setConnectionResult,
    modelsListAllMock,
    modelsListLocalOpenaiMock,
    modelsRefreshCacheMock,
    testProviderConnectionMock,
    sessionSetModelMock,
    openModelSelector,
    openProviderSettings,
    closeAllScreens,
    reset,
    resetCallbacks,
    cleanup,
    waitForModelsLoaded,
  };
}

// =============================================================================
// TEST WRAPPER COMPONENT
// =============================================================================

export interface ScreenIntegrationWrapperProps {
  /** Terminal width */
  width: number;
  /** Terminal height */
  height: number;
  /** Initial screen state */
  initialScreenState?: Partial<ScreenState>;
  /** Current model ID for highlighting */
  currentModelId?: string;
  /** Called when model is selected */
  onModelSelected?: (model: ModelSelection) => void;
  /** Called when session model should be updated */
  onSessionModelUpdate?: (sessionId: string, model: ModelSelection) => void;
  /** Active session ID (if any) */
  sessionId?: string;
  /** Screen state change callback */
  onScreenStateChange?: (state: ScreenState) => void;
}

/**
 * Test wrapper component that simulates AgentView's screen coordination.
 * 
 * This component:
 * - Manages showModelSelector and showSettingsTab state
 * - Renders real ModelSelectorScreen and ProviderSettingsScreen
 * - Handles screen switching via Tab
 * - Handles closing via Escape
 * - Calls callbacks when model is selected
 * 
 * Use this for testing the integration between screens without full AgentView.
 */
export function createScreenIntegrationWrapper(
  ModelSelectorScreen: React.ComponentType<{
    width: number;
    height: number;
    currentModelId?: string;
    onSelectModel: (model: ModelSelection) => void;
    onClose: () => void;
    onSwitchToSettings: () => void;
  }>,
  ProviderSettingsScreen: React.ComponentType<{
    width: number;
    height: number;
    onClose: () => void;
    onSwitchToModels: () => void;
  }>
): React.FC<ScreenIntegrationWrapperProps> {
  return function ScreenIntegrationWrapper({
    width,
    height,
    initialScreenState,
    currentModelId,
    onModelSelected,
    onSessionModelUpdate,
    sessionId,
    onScreenStateChange,
  }: ScreenIntegrationWrapperProps): React.ReactElement | null {
    const [showModelSelector, setShowModelSelector] = useState(
      initialScreenState?.showModelSelector ?? false
    );
    const [showSettingsTab, setShowSettingsTab] = useState(
      initialScreenState?.showSettingsTab ?? false
    );
    const [currentModel, setCurrentModel] = useState<ModelSelection | null>(
      initialScreenState?.currentModel ?? null
    );

    // Notify parent of state changes
    const notifyStateChange = useCallback(() => {
      onScreenStateChange?.({
        showModelSelector,
        showSettingsTab,
        currentModel,
      });
    }, [showModelSelector, showSettingsTab, currentModel, onScreenStateChange]);

    // Handle model selection
    const handleModelSelect = useCallback(
      (model: ModelSelection) => {
        setCurrentModel(model);
        setShowModelSelector(false);
        onModelSelected?.(model);

        if (sessionId) {
          onSessionModelUpdate?.(sessionId, model);
        }
      },
      [sessionId, onModelSelected, onSessionModelUpdate]
    );

    // Handle model selector close
    const handleModelSelectorClose = useCallback(() => {
      setShowModelSelector(false);
    }, []);

    // Handle switch to settings
    const handleSwitchToSettings = useCallback(() => {
      setShowModelSelector(false);
      setShowSettingsTab(true);
    }, []);

    // Handle settings close
    const handleSettingsClose = useCallback(() => {
      setShowSettingsTab(false);
    }, []);

    // Handle switch to models
    const handleSwitchToModels = useCallback(() => {
      setShowSettingsTab(false);
      setShowModelSelector(true);
    }, []);

    // Render model selector screen
    if (showModelSelector) {
      return (
        <ModelSelectorScreen
          width={width}
          height={height}
          currentModelId={currentModelId ?? currentModel?.apiModelId}
          onSelectModel={handleModelSelect}
          onClose={handleModelSelectorClose}
          onSwitchToSettings={handleSwitchToSettings}
        />
      );
    }

    // Render provider settings screen
    if (showSettingsTab) {
      return (
        <ProviderSettingsScreen
          width={width}
          height={height}
          onClose={handleSettingsClose}
          onSwitchToModels={handleSwitchToModels}
        />
      );
    }

    // No screen open - return null (main view would be shown)
    return null;
  };
}

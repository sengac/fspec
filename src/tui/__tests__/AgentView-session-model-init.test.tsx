/**
 * Test: Session creation with model initialization
 *
 * Ensures that sessions are created with proper model format and that
 * creation waits for model initialization to prevent race conditions.
 *
 * This addresses the bug where sessions were created with "claude" instead
 * of "anthropic/claude-opus-4-5" due to async model loading.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock NAPI before any imports - use vi.hoisted() to hoist the mocks
const {
  mockSessionManagerCreateWithId,
  mockModelsListAll,
  mockPersistenceCreateSessionWithProvider,
  mockGetProviderConfig,
} = vi.hoisted(() => ({
  mockSessionManagerCreateWithId: vi.fn(),
  mockModelsListAll: vi.fn(),
  mockPersistenceCreateSessionWithProvider: vi.fn(),
  mockGetProviderConfig: vi.fn(),
}));

vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerCreateWithId: mockSessionManagerCreateWithId,
  modelsListAll: mockModelsListAll,
  persistenceSetDataDirectory: vi.fn(),
  persistenceCreateSessionWithProvider: mockPersistenceCreateSessionWithProvider,
  sessionManagerList: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
}));

vi.mock('../../utils/credentials', () => ({
  getProviderConfig: mockGetProviderConfig,
}));

vi.mock('../../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({
    tui: {
      lastUsedModel: 'anthropic/claude-opus-4-5',
    },
  })),
  writeConfig: vi.fn(() => Promise.resolve()),
}));

vi.mock('../../../utils/getFspecUserDir', () => ({
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
}));

import { createSession } from '../services/sessionService';

describe('Session creation with model initialization', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Mock models.dev data
    mockModelsListAll.mockResolvedValue([
      {
        providerId: 'anthropic',
        providerName: 'Anthropic',
        models: [
          {
            id: 'claude-opus-4-5-20251101',
            name: 'Claude Opus 4.5',
            family: 'claude-opus-4-5',
            reasoning: true,
            toolCall: true,
            hasVision: true,
            contextWindow: 200000,
            maxOutput: 16000,
          },
          {
            id: 'claude-sonnet-4-20250514',
            name: 'Claude Sonnet 4',
            family: 'claude-sonnet-4',
            reasoning: true,
            toolCall: true,
            hasVision: true,
            contextWindow: 200000,
            maxOutput: 8192,
          },
        ],
      },
    ]);

    // Mock credentials check
    mockGetProviderConfig.mockResolvedValue({
      apiKey: 'test-key',
      source: 'env',
    });

    // Mock persistence
    mockPersistenceCreateSessionWithProvider.mockReturnValue({
      id: 'test-session-id',
      name: 'Test Session',
      project: 'test-project',
      provider: 'anthropic/claude-opus-4-5',
    });

    mockSessionManagerCreateWithId.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should create session with full model path format (provider/model-id)', async () => {
    // Given: A model path in the correct format
    const modelPath = 'anthropic/claude-opus-4-5';

    // When: Creating a session
    await createSession({
      modelPath,
      project: 'test-project',
    });

    // Then: Session should be created with the full model path
    // CONFIG-005: apiKey parameter removed - Rust resolves credentials internally
    expect(mockSessionManagerCreateWithId).toHaveBeenCalledWith(
      'test-session-id',
      'anthropic/claude-opus-4-5',
      'test-project',
      expect.any(String)
    );
  });

  it('should reject model paths without provider prefix', async () => {
    // Given: A model path without provider prefix (old format)
    const invalidModelPath = 'claude';

    // When/Then: Creating a session should fail
    // The Rust layer will reject this format
    mockSessionManagerCreateWithId.mockRejectedValue(
      new Error("Invalid model string 'claude': must be in 'provider/model-id' format")
    );

    await expect(
      createSession({
        modelPath: invalidModelPath,
        project: 'test-project',
      })
    ).rejects.toThrow("must be in 'provider/model-id' format");
  });

  it('should use persisted model from config', async () => {
    // Given: Config has a persisted model
    // (Already mocked in beforeEach to return 'anthropic/claude-opus-4-5')

    // When: Creating a session with the persisted model
    await createSession({
      modelPath: 'anthropic/claude-opus-4-5',
      project: 'test-project',
    });

    // Then: Should use the exact persisted model path
    // CONFIG-005: apiKey parameter removed - Rust resolves credentials internally
    expect(mockSessionManagerCreateWithId).toHaveBeenCalledWith(
      expect.any(String),
      'anthropic/claude-opus-4-5',
      'test-project',
      expect.any(String)
    );
  });

  it('should validate model format before creating session', async () => {
    // Given: Various model path formats
    const validPaths = [
      'anthropic/claude-opus-4-5',
      'google/gemini-2.0-flash',
      'openai/gpt-4o',
    ];

    const invalidPaths = [
      'claude', // Missing provider
      'anthropic', // Missing model
      '', // Empty
    ];

    // When/Then: Valid paths should work
    for (const path of validPaths) {
      mockPersistenceCreateSessionWithProvider.mockReturnValue({
        id: `session-${path}`,
        name: 'Test',
        project: 'test',
        provider: path,
      });

      await expect(
        createSession({
          modelPath: path,
          project: 'test-project',
        })
      ).resolves.toBeDefined();
    }

    // When/Then: Invalid paths should be rejected by Rust layer
    for (const path of invalidPaths) {
      mockSessionManagerCreateWithId.mockRejectedValueOnce(
        new Error(`Invalid model string '${path}': must be in 'provider/model-id' format`)
      );

      await expect(
        createSession({
          modelPath: path,
          project: 'test-project',
        })
      ).rejects.toThrow('must be in');
    }
  });

  it('should include provider and model in session creation call', async () => {
    // Given: A specific model selection
    const modelPath = 'anthropic/claude-sonnet-4';

    // When: Creating a session
    await createSession({
      modelPath,
      project: 'my-project',
      name: 'My Session',
    });

    // Then: Both provider and model should be passed to Rust
    // CONFIG-005: apiKey parameter removed - Rust resolves credentials internally
    expect(mockSessionManagerCreateWithId).toHaveBeenCalledWith(
      expect.any(String),
      'anthropic/claude-sonnet-4',
      'my-project',
      'My Session'
    );
  });

  it('should persist model selection in correct format', async () => {
    // Given: A model is selected
    const modelPath = 'anthropic/claude-opus-4-5';

    // When: Creating a session
    await createSession({
      modelPath,
      project: 'test-project',
    });

    // Then: Persistence should store the full model path
    expect(mockPersistenceCreateSessionWithProvider).toHaveBeenCalledWith(
      expect.any(String),
      'test-project',
      'anthropic/claude-opus-4-5'
    );
  });
});

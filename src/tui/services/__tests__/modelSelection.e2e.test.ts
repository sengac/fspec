/**
 * Feature: spec/features/profile-based-model-selection-not-restored-on-session-startup.feature
 *
 * E2E tests for model selection and switching.
 * Uses REAL NAPI bindings - NO MOCKS, NO STUBS.
 *
 * BUG-097: Verifies that model switching actually changes the model
 * the session uses for API calls, not just metadata.
 */

import {
  describe,
  it,
  expect,
  beforeAll,
  afterAll,
  beforeEach,
  afterEach,
} from 'vitest';
import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

// ========================================
// CONSTANTS
// ========================================

const TEST_MODEL_CLAUDE = 'anthropic/claude-sonnet-4-20250514';
const CLEANUP_DELAY_MS = 100;

// ========================================
// E2E FIXTURE - Real NAPI
// ========================================

interface ModelSelectionFixture {
  testDir: string;
  createdSessionIds: string[];
  createSession: (model?: string, name?: string) => Promise<string>;
  destroyAllSessions: () => Promise<void>;
  cleanup: () => Promise<void>;
}

async function createModelSelectionFixture(
  testName: string
): Promise<ModelSelectionFixture> {
  const testDir = join(
    tmpdir(),
    `fspec-model-e2e-${testName}-${randomUUID().slice(0, 8)}`
  );
  const specDir = join(testDir, 'spec');
  const createdSessionIds: string[] = [];

  // Create project structure
  await mkdir(specDir, { recursive: true });

  // Create minimal work-units.json
  await writeFile(
    join(specDir, 'work-units.json'),
    JSON.stringify(
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      },
      null,
      2
    )
  );

  // Set persistence directory for NAPI
  const { persistenceSetDataDirectory } = await import('@sengac/codelet-napi');
  persistenceSetDataDirectory(testDir);

  const createSession = async (
    model = TEST_MODEL_CLAUDE,
    name = 'Model Test Session'
  ): Promise<string> => {
    const { sessionManagerCreateWithId } = await import('@sengac/codelet-napi');
    const sessionId = randomUUID();

    try {
      await sessionManagerCreateWithId(sessionId, model, testDir, name);
    } catch {
      // Session creation may fail due to invalid API key, but session still registered
    }

    createdSessionIds.push(sessionId);
    return sessionId;
  };

  const destroyAllSessions = async (): Promise<void> => {
    const { sessionManagerDestroy, sessionManagerList } = await import(
      '@sengac/codelet-napi'
    );

    // Destroy tracked sessions
    for (const id of [...createdSessionIds]) {
      try {
        sessionManagerDestroy(id);
      } catch {
        // Ignore errors in cleanup
      }
    }
    createdSessionIds.length = 0;

    // Clean up orphaned sessions
    try {
      const allSessions = sessionManagerList();
      for (const session of allSessions) {
        try {
          sessionManagerDestroy(session.id);
        } catch {
          // Ignore errors in cleanup
        }
      }
    } catch {
      // Ignore errors in cleanup
    }
  };

  const cleanup = async (): Promise<void> => {
    await destroyAllSessions();
    await new Promise(resolve => setTimeout(resolve, CLEANUP_DELAY_MS));
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  };

  return {
    testDir,
    createdSessionIds,
    createSession,
    destroyAllSessions,
    cleanup,
  };
}

// ========================================
// E2E TESTS
// ========================================

describe('Feature: Model Selection E2E', () => {
  let fixture: ModelSelectionFixture;

  beforeAll(async () => {
    fixture = await createModelSelectionFixture('model-selection');
  });

  afterAll(async () => {
    await fixture.cleanup();
  });

  beforeEach(async () => {
    await fixture.destroyAllSessions();
  });

  afterEach(async () => {
    await fixture.destroyAllSessions();
  });

  // ========================================
  // Basic model operations
  // ========================================

  describe('Scenario: Verify sessionGetModel returns correct model after creation', () => {
    it('should return the model used to create the session', async () => {
      // @step Given I create a session with model "anthropic/claude-sonnet-4-20250514"
      const sessionId = await fixture.createSession(TEST_MODEL_CLAUDE);

      // @step When I call sessionGetModel
      const { sessionGetModel } = await import('@sengac/codelet-napi');
      const modelInfo = sessionGetModel(sessionId);

      // @step Then the providerId should be "anthropic"
      expect(modelInfo.providerId).toBe('anthropic');

      // @step And the modelId should be "claude-sonnet-4-20250514"
      expect(modelInfo.modelId).toBe('claude-sonnet-4-20250514');
    });
  });

  describe('Scenario: Verify sessionSetModelProfile changes session model', () => {
    it('should update model returned by sessionGetModel after sessionSetModelProfile', async () => {
      // @step Given I create a session with model "anthropic/claude-sonnet-4-20250514"
      const sessionId = await fixture.createSession(TEST_MODEL_CLAUDE);

      const { sessionGetModel, sessionSetModelProfile } = await import(
        '@sengac/codelet-napi'
      );

      // Verify initial model
      const initialModel = sessionGetModel(sessionId);
      expect(initialModel.providerId).toBe('anthropic');
      expect(initialModel.modelId).toBe('claude-sonnet-4-20250514');

      // @step When I call sessionSetModelProfile with provider "openai" and model "gpt-4o"
      await sessionSetModelProfile(sessionId, 'openai', 'gpt-4o');

      // @step Then sessionGetModel should return the NEW model
      const updatedModel = sessionGetModel(sessionId);
      console.log('Model after sessionSetModelProfile:', updatedModel);

      expect(updatedModel.providerId).toBe('openai');
      expect(updatedModel.modelId).toBe('gpt-4o');
    });
  });

  describe('Scenario: Verify model switch affects only the specified session', () => {
    it('should only change the model for the specified session, not others', async () => {
      // @step Given I create session A with model "anthropic/claude-sonnet-4-20250514"
      const sessionA = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Session A'
      );

      // @step And I create session B with model "anthropic/claude-sonnet-4-20250514"
      const sessionB = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Session B'
      );

      const { sessionGetModel, sessionSetModelProfile } = await import(
        '@sengac/codelet-napi'
      );

      // Verify both sessions start with Claude
      expect(sessionGetModel(sessionA).providerId).toBe('anthropic');
      expect(sessionGetModel(sessionB).providerId).toBe('anthropic');

      // @step When I call sessionSetModelProfile on session A with "openai/gpt-4o"
      await sessionSetModelProfile(sessionA, 'openai', 'gpt-4o');

      // @step Then session A should have model "openai/gpt-4o"
      const modelA = sessionGetModel(sessionA);
      expect(modelA.providerId).toBe('openai');
      expect(modelA.modelId).toBe('gpt-4o');

      // @step And session B should STILL have model "anthropic/claude-sonnet-4-20250514"
      const modelB = sessionGetModel(sessionB);
      expect(modelB.providerId).toBe('anthropic');
      expect(modelB.modelId).toBe('claude-sonnet-4-20250514');
    });
  });

  describe('Scenario: Verify internal provider is updated, not just metadata', () => {
    it('should verify the internal provider type changes', async () => {
      // @step Given I create a session with model "anthropic/claude-sonnet-4-20250514"
      const sessionId = await fixture.createSession(TEST_MODEL_CLAUDE);

      const { sessionSetModelProfile, sessionGetInternalProvider } =
        await import('@sengac/codelet-napi');

      // @step Then internal provider should be "claude"
      const initialProvider = await sessionGetInternalProvider(sessionId);
      console.log('Initial internal provider:', initialProvider);
      expect(initialProvider.providerId).toBe('claude');

      // @step When I call sessionSetModelProfile to switch to "openai/gpt-4o"
      await sessionSetModelProfile(sessionId, 'openai', 'gpt-4o');

      // @step Then internal provider should be "openai"
      const updatedProvider = await sessionGetInternalProvider(sessionId);
      console.log('Updated internal provider:', updatedProvider);
      expect(updatedProvider.providerId).toBe('openai');
    });
  });

  // ========================================
  // Session management and active session tracking
  // ========================================

  describe('Scenario: Multiple sessions - sessionGetActive tracks most recent', () => {
    it('should return the most recently created session', async () => {
      const { sessionGetActive, sessionManagerList } = await import(
        '@sengac/codelet-napi'
      );

      // @step Given I create session A
      const sessionA = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Session A'
      );

      // @step Then session A should be active
      let activeSession = sessionGetActive();
      console.log('Active session after creating A:', activeSession);
      expect(activeSession).toBe(sessionA);

      // @step When I create session B
      const sessionB = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Session B'
      );

      // @step Then session B should now be active (most recently created)
      activeSession = sessionGetActive();
      console.log('Active session after creating B:', activeSession);
      expect(activeSession).toBe(sessionB);

      // Verify both exist
      const sessions = sessionManagerList();
      expect(sessions.map(s => s.id)).toContain(sessionA);
      expect(sessions.map(s => s.id)).toContain(sessionB);
    });
  });

  describe('Scenario: useSessionStore.currentSessionId vs sessionGetActive divergence', () => {
    it('should demonstrate potential mismatch between store and NAPI', async () => {
      const { sessionGetActive } = await import('@sengac/codelet-napi');
      const { useSessionStore } = await import('../../store/sessionStore');

      // @step Given I reset the store
      useSessionStore.getState().prepareForNewSession();
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // @step When I create a session via NAPI directly (bypassing store)
      const napiSessionId = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'NAPI Direct Session'
      );

      // @step Then sessionGetActive returns the NAPI session
      const rustActive = sessionGetActive();
      console.log('Rust active session:', rustActive);
      expect(rustActive).toBe(napiSessionId);

      // @step But useSessionStore.currentSessionId is still null!
      const storeSessionId = useSessionStore.getState().currentSessionId;
      console.log('Store currentSessionId:', storeSessionId);
      expect(storeSessionId).toBeNull();

      // This demonstrates that NAPI and store can diverge if not kept in sync
    });
  });

  describe('Scenario: Model change with properly synced store session ID', () => {
    it('should work correctly when store and NAPI are in sync', async () => {
      const {
        sessionGetActive,
        sessionGetModel,
        sessionSetModelProfile,
        sessionGetInternalProvider,
      } = await import('@sengac/codelet-napi');
      const { useSessionStore } = await import('../../store/sessionStore');

      // @step Given I create a session
      const sessionId = await fixture.createSession(TEST_MODEL_CLAUDE);

      // @step And I activate it in the store (simulating AgentView.activateSession)
      useSessionStore.getState().activateSession(sessionId);
      const storeSessionId = useSessionStore.getState().currentSessionId;
      console.log('Store currentSessionId:', storeSessionId);
      console.log('Created sessionId:', sessionId);

      // @step Then they should match
      expect(storeSessionId).toBe(sessionId);

      // @step And Rust active should also match
      const rustActive = sessionGetActive();
      console.log('Rust active:', rustActive);
      expect(rustActive).toBe(sessionId);

      // @step When I change the model using storeSessionId
      await sessionSetModelProfile(storeSessionId!, 'openai', 'gpt-4o');

      // @step Then the model should be changed for the correct session
      const model = sessionGetModel(sessionId);
      console.log('Model after change:', model);
      expect(model.providerId).toBe('openai');

      // @step And internal provider should also be updated
      const internalProvider = await sessionGetInternalProvider(sessionId);
      console.log('Internal provider:', internalProvider);
      expect(internalProvider.providerId).toBe('openai');
    });
  });

  describe('Scenario: What happens when model selector creates a background session', () => {
    it('should trace the divergence when a background session is created', async () => {
      const {
        sessionGetActive,
        sessionGetModel,
        sessionSetModelProfile,
        sessionManagerList,
      } = await import('@sengac/codelet-napi');
      const { useSessionStore } = await import('../../store/sessionStore');

      // @step Given I create and activate a "foreground" session
      const foregroundId = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Foreground Session'
      );
      useSessionStore.getState().activateSession(foregroundId);
      console.log('Foreground session created and activated:', foregroundId);

      // Verify store and NAPI are in sync
      expect(useSessionStore.getState().currentSessionId).toBe(foregroundId);
      expect(sessionGetActive()).toBe(foregroundId);

      // @step When a "background" session is created (simulating what might happen)
      const backgroundId = await fixture.createSession(
        TEST_MODEL_CLAUDE,
        'Background Session'
      );
      console.log('Background session created:', backgroundId);

      // @step Then sessionGetActive changes to the background session
      const activeAfterBackground = sessionGetActive();
      console.log('Active after background created:', activeAfterBackground);
      expect(activeAfterBackground).toBe(backgroundId); // This is likely the bug!

      // @step But store still points to foreground
      const storeSessionId = useSessionStore.getState().currentSessionId;
      console.log('Store currentSessionId still:', storeSessionId);
      expect(storeSessionId).toBe(foregroundId);

      // @step So if model selection uses sessionGetActive() instead of store...
      // It would change the WRONG session!
      await sessionSetModelProfile(activeAfterBackground, 'openai', 'gpt-4o');

      // Background session is changed
      const backgroundModel = sessionGetModel(backgroundId);
      console.log('Background model after change:', backgroundModel);
      expect(backgroundModel.providerId).toBe('openai');

      // But foreground session is NOT changed!
      const foregroundModel = sessionGetModel(foregroundId);
      console.log('Foreground model (should be unchanged):', foregroundModel);
      expect(foregroundModel.providerId).toBe('anthropic'); // Still Claude!

      // List all sessions
      const allSessions = sessionManagerList();
      console.log(
        'All sessions:',
        allSessions.map(s => ({ id: s.id, name: s.name }))
      );
    });
  });

  // ========================================
  // BUG-097: Profile session -> Cloud model switch fails
  // ========================================

  describe('Scenario: Profile session can switch to cloud model', () => {
    it('should allow switching from profile model to cloud model', async () => {
      const { sessionGetModel, sessionSetModel, sessionGetInternalProvider } =
        await import('@sengac/codelet-napi');

      // @step Given I create a session with a PROFILE model format
      const PROFILE_MODEL = 'openai:test-profile/local-model-123';
      const sessionId = await fixture.createSession(
        PROFILE_MODEL,
        'Profile Session'
      );

      // @step Then the session should have the profile model
      const initialModel = sessionGetModel(sessionId);
      console.log('Initial model (profile):', initialModel);
      expect(initialModel.providerId).toBe('openai');

      // @step When I switch to a CLOUD model using sessionSetModel
      await sessionSetModel(sessionId, 'anthropic', 'claude-sonnet-4');

      // @step Then the model should be updated
      const updatedModel = sessionGetModel(sessionId);
      console.log('Model after switch:', updatedModel);
      expect(updatedModel.providerId).toBe('anthropic');
      expect(updatedModel.modelId).toBe('claude-sonnet-4');

      // @step And the internal provider should also be updated
      const internalProvider = await sessionGetInternalProvider(sessionId);
      console.log('Internal provider:', internalProvider);
      expect(internalProvider.providerId).toBe('claude');
    });
  });

  describe('Scenario: Verify Node.js process.env propagates to Rust', () => {
    it('should check if env vars set in Node.js are visible to Rust', async () => {
      const { getEnvVar } = await import('@sengac/codelet-napi');

      // Set a test env var in Node.js
      const testValue = `test-value-${Date.now()}`;
      process.env.FSPEC_TEST_ENV_VAR = testValue;

      // Check if Rust can read it
      const rustSees = getEnvVar('FSPEC_TEST_ENV_VAR');
      console.log('Set in Node.js:', testValue);
      console.log('Rust sees:', rustSees);

      // This test verifies the fundamental assumption about env var propagation
      expect(rustSees).toBe(testValue);
    });
  });
});

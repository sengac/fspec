/**
 * Session Service Integration Tests
 *
 * TUI-068: Integration tests for session-work unit state management.
 * Uses REAL stores and REAL NAPI bindings with reusable fixtures.
 *
 * NO MOCKS - Tests exercise the actual session service implementation.
 *
 * SOLID/DRY: Reusable fixtures, real implementations, composable setup.
 */

import {
  describe,
  it,
  expect,
  beforeEach,
  afterEach,
  beforeAll,
  afterAll,
} from 'vitest';

import {
  createSessionServiceFixture,
  createSessionServiceNapiFixture,
  type SessionServiceFixture,
  type SessionServiceNapiFixture,
} from './fixtures/sessionServiceFixture';

describe('Feature: Session Service Integration (Real Stores)', () => {
  let fixture: SessionServiceFixture;

  beforeEach(async () => {
    fixture = await createSessionServiceFixture('store-integration');
  });

  afterEach(async () => {
    await fixture.cleanup();
  });

  describe('Scenario: Session attachment updates both stores atomically', () => {
    it('should update sessionAttachments and currentWorkUnit when attaching', async () => {
      // @step Given I have a clean store state
      const initialState = fixture.getStoreState();
      expect(initialState.sessionAttachments.size).toBe(0);
      expect(initialState.currentWorkUnitId).toBeNull();

      // @step When I attach session "session-123" to work unit "TOOL-014"
      fixture.attachSession('TOOL-014', 'session-123');
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');

      // @step Then sessionAttachments should map "TOOL-014" to "session-123"
      expect(fixture.getAttachedSession('TOOL-014')).toBe('session-123');

      // @step And currentWorkUnitId should be "TOOL-014"
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBe('TOOL-014');
      expect(state.currentWorkUnitStatus).toBe('specifying');
    });
  });

  describe('Scenario: Session detachment clears all state atomically', () => {
    it('should clear both stores when detaching session', async () => {
      // @step Given I have session "session-123" attached to "TOOL-014"
      fixture.attachSession('TOOL-014', 'session-123');
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');
      expect(fixture.getAttachedSession('TOOL-014')).toBe('session-123');

      // @step When I detach session from "TOOL-014"
      fixture.detachSession('TOOL-014');
      fixture.setCurrentWorkUnit(null, null);

      // @step Then sessionAttachments should NOT contain "TOOL-014"
      expect(fixture.getAttachedSession('TOOL-014')).toBeUndefined();

      // @step And currentWorkUnitId should be null
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBeNull();
      expect(state.currentWorkUnitStatus).toBeNull();
    });
  });

  describe('Scenario: Work unit context change via IPC', () => {
    it('should update attachments when session moves to different work unit', async () => {
      // @step Given session "session-123" is attached to "TOOL-014"
      fixture.attachSession('TOOL-014', 'session-123');
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');

      // @step When AI changes work unit to "AUTH-001" via IPC
      fixture.detachSession('TOOL-014'); // Old attachment
      fixture.attachSession('AUTH-001', 'session-123'); // New attachment
      fixture.setCurrentWorkUnit('AUTH-001', 'implementing');

      // @step Then "TOOL-014" should have no attached session
      expect(fixture.getAttachedSession('TOOL-014')).toBeUndefined();

      // @step And "AUTH-001" should have session "session-123" attached
      expect(fixture.getAttachedSession('AUTH-001')).toBe('session-123');

      // @step And currentWorkUnitId should be "AUTH-001"
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBe('AUTH-001');
      expect(state.currentWorkUnitStatus).toBe('implementing');
    });
  });

  describe('Scenario: Multiple sessions can be attached to different work units', () => {
    it('should support concurrent sessions on different work units', async () => {
      // @step Given session "session-1" attached to "TOOL-014"
      fixture.attachSession('TOOL-014', 'session-1');

      // @step And session "session-2" attached to "AUTH-001"
      fixture.attachSession('AUTH-001', 'session-2');

      // @step Then both mappings should exist
      expect(fixture.getAttachedSession('TOOL-014')).toBe('session-1');
      expect(fixture.getAttachedSession('AUTH-001')).toBe('session-2');

      // @step And reverse lookup should work
      expect(fixture.getWorkUnitBySession('session-1')).toBe('TOOL-014');
      expect(fixture.getWorkUnitBySession('session-2')).toBe('AUTH-001');
    });
  });

  describe('Scenario: Fixture resets cleanly between tests', () => {
    it('should start with clean state after reset', async () => {
      // @step Given I have some state set
      fixture.attachSession('TOOL-014', 'session-123');
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');

      // @step When I reset the stores
      fixture.resetStores();

      // @step Then all state should be cleared
      const state = fixture.getStoreState();
      expect(state.sessionAttachments.size).toBe(0);
      expect(state.currentWorkUnitId).toBeNull();
    });
  });
});

describe('Feature: Session Service Integration (Real NAPI)', () => {
  let fixture: SessionServiceNapiFixture;

  beforeAll(async () => {
    // NAPI fixture is more expensive - create once per describe block
    fixture = await createSessionServiceNapiFixture('napi-integration');
  });

  afterAll(async () => {
    await fixture.cleanup();
  });

  beforeEach(() => {
    // Reset stores between tests (but keep NAPI state for efficiency)
    fixture.resetStores();
  });

  describe('Scenario: Real session creation with store integration', () => {
    it('should create NAPI session and track in stores', async () => {
      // @step Given I create a real NAPI session
      const sessionId = await fixture.createNapiSession(
        'Integration Test Session'
      );
      expect(sessionId).toBeDefined();
      expect(sessionId).toMatch(/^[0-9a-f-]{36}$/); // UUID format

      // @step When I attach it to a work unit
      fixture.attachSession('TOOL-014', sessionId);
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');

      // @step Then the stores should reflect the attachment
      expect(fixture.getAttachedSession('TOOL-014')).toBe(sessionId);
      expect(fixture.getWorkUnitBySession(sessionId)).toBe('TOOL-014');

      // @step And the created session should be tracked for cleanup
      expect(fixture.getCreatedSessionIds()).toContain(sessionId);
    });
  });

  describe('Scenario: destroySession orchestrates cleanup', () => {
    it('should clean up both NAPI and stores when destroying session', async () => {
      // @step Given I have a real session attached to a work unit
      const sessionId = await fixture.createNapiSession('Destroy Test Session');
      fixture.attachSession('TOOL-014', sessionId);
      fixture.setCurrentWorkUnit('TOOL-014', 'specifying');

      // Verify setup
      expect(fixture.getAttachedSession('TOOL-014')).toBe(sessionId);

      // @step When I destroy the session using the service
      const { destroySession } = await import('../sessionService');
      await destroySession(sessionId);

      // @step Then the stores should be cleared
      // Note: destroySession clears sessionAttachments and currentWorkUnit
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBeNull();

      // The session should no longer be in the attachment map for TOOL-014
      expect(fixture.getAttachedSession('TOOL-014')).toBeUndefined();
    });
  });

  describe('Scenario: attachToWorkUnit orchestrates all stores', () => {
    it('should update all stores atomically via service', async () => {
      // @step Given I have a real session
      const sessionId = await fixture.createNapiSession('Attach Test Session');

      // @step When I attach via the service
      const { attachToWorkUnit } = await import('../sessionService');
      attachToWorkUnit(sessionId, 'TOOL-014', 'specifying');

      // @step Then fspecStore should have the attachment
      expect(fixture.getAttachedSession('TOOL-014')).toBe(sessionId);

      // @step And sessionStore should have currentWorkUnitId set
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBe('TOOL-014');
      expect(state.currentWorkUnitStatus).toBe('specifying');
    });
  });

  describe('Scenario: detachFromWorkUnit clears state via service', () => {
    it('should clear all stores atomically via service', async () => {
      // @step Given I have a session attached via the service
      const sessionId = await fixture.createNapiSession('Detach Test Session');
      const { attachToWorkUnit, detachFromWorkUnit } = await import(
        '../sessionService'
      );
      attachToWorkUnit(sessionId, 'TOOL-014', 'implementing');

      // Verify attachment
      expect(fixture.getAttachedSession('TOOL-014')).toBe(sessionId);

      // @step When I detach via the service
      detachFromWorkUnit(sessionId);

      // @step Then all stores should be cleared
      expect(fixture.getAttachedSession('TOOL-014')).toBeUndefined();
      const state = fixture.getStoreState();
      expect(state.currentWorkUnitId).toBeNull();
    });
  });
});

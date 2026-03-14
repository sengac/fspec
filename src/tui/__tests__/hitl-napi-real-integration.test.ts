/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Part 1: NAPI export verification + real session flow.
 * NO MOCKS — calls actual NAPI bindings.
 *
 * Tests:
 * - sessionGetHitlRequest exported, validates session IDs
 * - sessionSendHitlResponse exported, validates session IDs
 * - Fresh session has idle status and null hitlRequest
 *
 * BUG-118: HITL TUI integration
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

import {
  sessionGetHitlRequest,
  sessionSendHitlResponse,
  sessionManagerCreateWithId,
  sessionManagerDestroy,
  sessionManagerList,
  sessionGetStatus,
  persistenceSetDataDirectory,
} from '@sengac/codelet-napi';

describe('HITL NAPI Integration: Real Session Flow', () => {
  let dataDir: string;

  beforeEach(() => {
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'hitl-napi-'));
    persistenceSetDataDirectory(dataDir);
  });

  afterEach(() => {
    const sessions = sessionManagerList();
    for (const s of sessions) {
      try {
        sessionManagerDestroy(s.id);
      } catch {
        // Already destroyed
      }
    }
    try {
      fs.rmSync(dataDir, { recursive: true, force: true });
    } catch {
      // Ignore
    }
  });

  // ==========================================================================
  // Scenario: NAPI getter returns HITL request when session is paused
  // ==========================================================================

  describe('Scenario: NAPI getter returns HITL request when session is paused', () => {
    it('sessionGetHitlRequest should be exported as a function', () => {
      // @step Given the NAPI module is loaded
      // @step Then sessionGetHitlRequest should be a function
      expect(typeof sessionGetHitlRequest).toBe('function');
    });

    it('sessionGetHitlRequest should throw for invalid session ID', () => {
      // @step Given an invalid session ID format
      // @step When sessionGetHitlRequest is called
      // @step Then it should throw an error about invalid session ID
      expect(() => {
        sessionGetHitlRequest('not-a-uuid');
      }).toThrow(/invalid/i);
    });

    it('sessionGetHitlRequest should throw for non-existent session', () => {
      // @step Given a valid UUID but non-existent session
      // @step When sessionGetHitlRequest is called
      // @step Then it should throw "Session not found"
      expect(() => {
        sessionGetHitlRequest('00000000-0000-0000-0000-000000000000');
      }).toThrow(/session not found/i);
    });

    it('sessionGetHitlRequest should return null for a fresh session', async () => {
      // @step Given a newly created BackgroundSession
      const sessionId = '11111111-1111-1111-1111-111111111111';
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        dataDir,
        'HITL Test'
      );

      try {
        // @step When sessionGetHitlRequest is called on the fresh session
        const result = sessionGetHitlRequest(sessionId);

        // @step Then it should return null (no HITL request pending)
        expect(result).toBeNull();
      } finally {
        sessionManagerDestroy(sessionId);
      }
    });
  });

  // ==========================================================================
  // Scenario: NAPI binding converts TypeScript response to Rust HitlResponse
  // ==========================================================================

  describe('Scenario: NAPI response sender validates sessions', () => {
    it('sessionSendHitlResponse should be exported as a function', () => {
      // @step Given the NAPI module is loaded
      // @step Then sessionSendHitlResponse should be a function
      expect(typeof sessionSendHitlResponse).toBe('function');
    });

    it('sessionSendHitlResponse should throw for non-existent session', () => {
      // @step Given a valid UUID but non-existent session
      // @step When sessionSendHitlResponse is called with answers
      // @step Then it should throw "Session not found"
      expect(() => {
        sessionSendHitlResponse('00000000-0000-0000-0000-000000000000', {
          cancelled: false,
          answers: [{ id: 'q1', selected: ['Yes'] }],
        });
      }).toThrow(/session not found/i);
    });

    it('sessionSendHitlResponse should accept cancellation format', () => {
      // @step Given a valid UUID but non-existent session
      // @step When sessionSendHitlResponse is called with cancelled=true
      // @step Then it should throw "Session not found" (not a format error)
      expect(() => {
        sessionSendHitlResponse('00000000-0000-0000-0000-000000000000', {
          cancelled: true,
        });
      }).toThrow(/session not found/i);
    });
  });

  // ==========================================================================
  // Scenario: Fresh session has idle status and no HITL request
  // ==========================================================================

  describe('Scenario: Fresh session state', () => {
    it('should have idle status and null hitl request after creation', async () => {
      // @step Given a newly created BackgroundSession
      const sessionId = '22222222-2222-2222-2222-222222222222';
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        dataDir,
        'HITL Fresh Session Test'
      );

      try {
        // @step When the session status and HITL request are queried
        const status = sessionGetStatus(sessionId);
        const hitlRequest = sessionGetHitlRequest(sessionId);

        // @step Then status should be "idle"
        expect(status).toBe('idle');

        // @step And hitlRequest should be null
        expect(hitlRequest).toBeNull();
      } finally {
        sessionManagerDestroy(sessionId);
      }
    });
  });
});

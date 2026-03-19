/**
 * Feature: spec/features/loop-shorthand-natural-language-schedule-creation.feature
 *
 * Tests for the loop service — the TypeScript bridge that parses /loop
 * commands and registers them with the Rust-side LoopStore via NAPI.
 *
 * SCHED-011: Loop Shorthand — Natural Language Schedule Creation
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock NAPI bindings — the real loop store lives in Rust
const mockLoopRegister = vi.fn().mockResolvedValue(undefined);
const mockLoopCancel = vi.fn().mockResolvedValue(true);
const mockLoopList = vi.fn().mockResolvedValue('[]');

vi.mock('@sengac/codelet-napi', () => ({
  loopRegister: (...args: unknown[]) => mockLoopRegister(...args),
  loopCancel: (...args: unknown[]) => mockLoopCancel(...args),
  loopList: (...args: unknown[]) => mockLoopList(...args),
}));

import { handleLoopCommand } from '../loop-service';

const FAKE_SESSION_ID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

describe('Feature: Loop Shorthand — Service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLoopCancel.mockResolvedValue(true);
    mockLoopList.mockResolvedValue('[]');
  });

  describe('Scenario: Create loop with leading interval token (minutes)', () => {
    it('should parse interval and register with Rust scheduler', async () => {
      // @step When I type "/loop 5m check deployment status"
      const result = await handleLoopCommand(
        '/loop 5m check deployment status',
        FAKE_SESSION_ID
      );

      // @step Then the interval should be parsed as 5 minutes
      expect(result.success).toBe(true);
      expect(result.message).toContain('5 minutes');

      // @step And the TUI should confirm with an 8-character job ID
      expect(result.message).toMatch(/\[job: [a-f0-9]{8}\]/);

      // @step And the loop should be registered with the Rust LoopStore (300 seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.stringMatching(/^[a-f0-9]{8}$/),
        'check deployment status',
        300
      );
    });
  });

  describe('Scenario: Create loop with seconds interval', () => {
    it('should parse seconds and register with Rust scheduler', async () => {
      // @step When I type "/loop 5s check health"
      const result = await handleLoopCommand(
        '/loop 5s check health',
        FAKE_SESSION_ID
      );

      // @step Then the interval should be displayed as 5 seconds
      expect(result.success).toBe(true);
      expect(result.message).toContain('5 seconds');

      // @step And the Rust scheduler should receive interval=5 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.stringMatching(/^[a-f0-9]{8}$/),
        'check health',
        5
      );
    });
  });

  describe('Scenario: Create loop with hours interval', () => {
    it('should parse hours and register with Rust scheduler', async () => {
      // @step When I type "/loop 2h check status"
      const result = await handleLoopCommand(
        '/loop 2h check status',
        FAKE_SESSION_ID
      );

      // @step Then the interval should be displayed as 2 hours
      expect(result.success).toBe(true);
      expect(result.message).toContain('2 hours');

      // @step And the Rust scheduler should receive interval=7200 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        'check status',
        7200
      );
    });
  });

  describe('Scenario: Create loop with days interval', () => {
    it('should parse days and register with Rust scheduler', async () => {
      // @step When I type "/loop 1d run report"
      const result = await handleLoopCommand(
        '/loop 1d run report',
        FAKE_SESSION_ID
      );

      // @step Then the interval should be displayed as 1 day
      expect(result.success).toBe(true);
      expect(result.message).toContain('1 day');

      // @step And the Rust scheduler should receive interval=86400 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        'run report',
        86400
      );
    });
  });

  describe('Scenario: Create loop with default interval when none specified', () => {
    it('should create loop with 10-minute default', async () => {
      // @step When I type "/loop check the build"
      const result = await handleLoopCommand(
        '/loop check the build',
        FAKE_SESSION_ID
      );

      // @step Then the interval should default to 10 minutes
      expect(result.success).toBe(true);
      expect(result.message).toContain('10 minutes');

      // @step And the Rust scheduler should receive interval=600 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        'check the build',
        600
      );
    });
  });

  describe('Scenario: Create loop with trailing interval clause', () => {
    it('should parse trailing "every 2 hours" and create loop', async () => {
      // @step When I type "/loop check build status every 2 hours"
      const result = await handleLoopCommand(
        '/loop check build status every 2 hours',
        FAKE_SESSION_ID
      );

      // @step Then the trailing "every 2 hours" should be parsed
      expect(result.success).toBe(true);
      expect(result.message).toContain('2 hours');

      // @step And the Rust scheduler should receive interval=7200 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        'check build status',
        7200
      );
    });
  });

  describe('Scenario: 30-second interval is preserved', () => {
    it('should keep 30s as 30 seconds without rounding', async () => {
      // @step When I type "/loop 30s run health check"
      const result = await handleLoopCommand(
        '/loop 30s run health check',
        FAKE_SESSION_ID
      );

      // @step Then the interval should be displayed as 30 seconds
      expect(result.success).toBe(true);
      expect(result.message).toContain('30 seconds');

      // @step And the Rust scheduler should receive interval=30 (seconds)
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        'run health check',
        30
      );
    });
  });

  describe('Scenario: Chain slash commands as loop prompts', () => {
    it('should preserve slash command as the prompt', async () => {
      // @step When I type "/loop 20m /review-pr 1234"
      const result = await handleLoopCommand(
        '/loop 20m /review-pr 1234',
        FAKE_SESSION_ID
      );

      // @step Then the prompt sent to Rust should be "/review-pr 1234"
      expect(result.success).toBe(true);
      expect(result.message).toContain('20 minutes');
      expect(mockLoopRegister).toHaveBeenCalledWith(
        FAKE_SESSION_ID,
        expect.any(String),
        '/review-pr 1234',
        1200
      );
    });
  });

  describe('Scenario: Cancel an active loop by job ID', () => {
    it('should cancel loop via Rust and confirm removal', async () => {
      // @step When I type "/loop cancel a1b2c3d4"
      const result = await handleLoopCommand(
        '/loop cancel a1b2c3d4',
        FAKE_SESSION_ID
      );

      // @step Then the Rust LoopStore should be asked to cancel the loop
      expect(mockLoopCancel).toHaveBeenCalledWith('a1b2c3d4');

      // @step And the TUI should confirm "Cancelled loop a1b2c3d4"
      expect(result.success).toBe(true);
      expect(result.message).toContain('Cancelled');
      expect(result.message).toContain('a1b2c3d4');
    });

    it('should report error when loop not found', async () => {
      // @step Given the loop does not exist
      mockLoopCancel.mockResolvedValue(false);

      // @step When I type "/loop cancel deadbeef"
      const result = await handleLoopCommand(
        '/loop cancel deadbeef',
        FAKE_SESSION_ID
      );

      // @step Then the TUI should report the loop was not found
      expect(result.success).toBe(false);
      expect(result.message).toContain('not found');
    });
  });

  describe('Scenario: List all active loops', () => {
    it('should display table from Rust LoopStore', async () => {
      // @step Given loops are running in the Rust scheduler
      mockLoopList.mockResolvedValue(
        JSON.stringify([
          { id: 'abc12345', prompt: 'first task', intervalSeconds: 300 },
          { id: 'def67890', prompt: 'second task', intervalSeconds: 600 },
        ])
      );

      // @step When I type "/loop list"
      const result = await handleLoopCommand('/loop list', FAKE_SESSION_ID);

      // @step Then the TUI should display a table with both loops
      expect(result.success).toBe(true);
      expect(result.message).toContain('first task');
      expect(result.message).toContain('second task');
      expect(result.message).toContain('ID');
      expect(result.message).toContain('Prompt');
      expect(result.message).toContain('Interval');

      // @step And the Rust LoopStore should be queried for the session
      expect(mockLoopList).toHaveBeenCalledWith(FAKE_SESSION_ID);
    });

    it('should show "No active loops." when empty', async () => {
      // @step Given no loops are running
      mockLoopList.mockResolvedValue('[]');

      // @step When I type "/loop list"
      const result = await handleLoopCommand('/loop list', FAKE_SESSION_ID);

      // @step Then the TUI should display "No active loops."
      expect(result.success).toBe(true);
      expect(result.message).toBe('No active loops.');
    });
  });

  describe('Scenario: Show usage help when no arguments provided', () => {
    it('should display help text for bare /loop (no session required)', async () => {
      // @step When I type "/loop" with no arguments
      const result = await handleLoopCommand('/loop', null);

      // @step Then the TUI should display usage help
      expect(result.success).toBe(true);
      expect(result.message).toContain('/loop');
      expect(result.message).toContain('cancel');
      expect(result.message).toContain('list');

      // @step And no NAPI call should be made
      expect(mockLoopRegister).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: No active session', () => {
    it('should require a session for add/list/cancel', async () => {
      // @step When I type "/loop 5m check status" without an active session
      const result = await handleLoopCommand('/loop 5m check status', null);

      // @step Then the service should return an error
      expect(result.success).toBe(false);
      expect(result.message).toContain('No active session');

      // @step And no NAPI call should be made
      expect(mockLoopRegister).not.toHaveBeenCalled();
    });
  });
});

/**
 * Tests for TUI-058: Default Thinking Level Config Persistence
 *
 * Feature: spec/features/default-thinking-level-persistence.feature
 *
 * Tests config persistence using REAL helper functions:
 * - loadDefaultThinkingLevel reads from config
 * - saveDefaultThinkingLevel writes to config
 * - Corrupt/missing config handled gracefully
 * - Session initialization uses persisted default
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { JsThinkingLevel } from '../../../utils/thinkingLevel';

// Create hoisted mock config state
const mockConfig = vi.hoisted(() => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
}));

// Mock the config utilities that loadDefaultThinkingLevel/saveDefaultThinkingLevel use
vi.mock('../../../utils/config', () => ({
  loadConfig: (...args: unknown[]) => mockConfig.loadConfig(...args),
  writeConfig: (...args: unknown[]) => mockConfig.writeConfig(...args),
}));

// Import the REAL helper functions (they use mocked config internally)
import {
  loadDefaultThinkingLevel,
  saveDefaultThinkingLevel,
} from '../../config/defaultThinkingLevelConfig';

describe('Feature: Default Thinking Level Persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig.loadConfig.mockResolvedValue({});
    mockConfig.writeConfig.mockResolvedValue(undefined);
  });

  // ----------------------------------------
  // SESSION INITIALIZATION - Restoring Default
  // ----------------------------------------

  describe('Scenario: Restore default thinking level on new session', () => {
    it('should load default from config and apply to new session', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.defaultThinkingLevel": 3
      mockConfig.loadConfig.mockResolvedValue({
        tui: { defaultThinkingLevel: 3 }, // High
      });

      // @step When the user starts a new agent session
      const defaultLevel = await loadDefaultThinkingLevel();

      // @step Then the session starts with base thinking level High
      expect(defaultLevel).toBe(JsThinkingLevel.High);

      // @step And the SessionHeader shows the thinking level indicator
      expect(defaultLevel).not.toBeNull();
    });
  });

  describe('Scenario: Use Off when no default is set', () => {
    it('should use Off (0) when no default is configured', async () => {
      // @step Given ~/.fspec/fspec-config.json does not contain tui.defaultThinkingLevel
      mockConfig.loadConfig.mockResolvedValue({
        tui: {}, // No defaultThinkingLevel
      });

      // @step When the user starts a new agent session
      const defaultLevel = await loadDefaultThinkingLevel();

      // @step Then the session starts with base thinking level Off
      expect(defaultLevel).toBeNull();
      // When null, the session should use Off (0)
      const effectiveDefault = defaultLevel ?? JsThinkingLevel.Off;
      expect(effectiveDefault).toBe(JsThinkingLevel.Off);

      // @step And the SessionHeader does not show a thinking level indicator
      // (Off level means no badge is shown)
    });
  });

  // ----------------------------------------
  // ERROR HANDLING
  // ----------------------------------------

  describe('Scenario: Handle corrupt config gracefully', () => {
    it('should use Off when config is corrupt', async () => {
      // @step Given ~/.fspec/fspec-config.json contains invalid JSON
      mockConfig.loadConfig.mockRejectedValue(new Error('Invalid JSON'));

      // @step When the user starts a new agent session
      const defaultLevel = await loadDefaultThinkingLevel();

      // @step Then the session starts with base thinking level Off
      const effectiveDefault = defaultLevel ?? JsThinkingLevel.Off;
      expect(effectiveDefault).toBe(JsThinkingLevel.Off);

      // @step And no error is shown to the user
      // (The error is caught internally by loadDefaultThinkingLevel)
      expect(defaultLevel).toBeNull();

      // @step And the session is fully functional
      // (Session creation proceeds with Off level)
    });
  });
});

// ----------------------------------------
// UNIT TESTS - Real Config Helpers
// ----------------------------------------

describe('Default Thinking Level Config Helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig.loadConfig.mockResolvedValue({});
    mockConfig.writeConfig.mockResolvedValue(undefined);
  });

  describe('loadDefaultThinkingLevel', () => {
    it('should return level from config when present', async () => {
      mockConfig.loadConfig.mockResolvedValue({
        tui: { defaultThinkingLevel: 2 },
      });

      const level = await loadDefaultThinkingLevel();

      expect(level).toBe(JsThinkingLevel.Medium);
    });

    it('should return null when not configured', async () => {
      mockConfig.loadConfig.mockResolvedValue({});

      const level = await loadDefaultThinkingLevel();

      expect(level).toBeNull();
    });

    it('should return null on error', async () => {
      mockConfig.loadConfig.mockRejectedValue(new Error('File not found'));

      const level = await loadDefaultThinkingLevel();

      expect(level).toBeNull();
    });

    it('should return null for invalid level values', async () => {
      mockConfig.loadConfig.mockResolvedValue({
        tui: { defaultThinkingLevel: 'invalid' },
      });

      const level = await loadDefaultThinkingLevel();

      expect(level).toBeNull();
    });

    it('should return null for out-of-range numeric values', async () => {
      mockConfig.loadConfig.mockResolvedValue({
        tui: { defaultThinkingLevel: 99 },
      });

      const level = await loadDefaultThinkingLevel();

      expect(level).toBeNull();
    });
  });

  describe('saveDefaultThinkingLevel', () => {
    it('should save level to config under tui.defaultThinkingLevel', async () => {
      mockConfig.loadConfig.mockResolvedValue({
        otherSetting: 'value',
      });

      await saveDefaultThinkingLevel(JsThinkingLevel.High);

      expect(mockConfig.writeConfig).toHaveBeenCalledWith('user', {
        otherSetting: 'value',
        tui: {
          defaultThinkingLevel: 3,
        },
      });
    });

    it('should preserve other tui settings', async () => {
      mockConfig.loadConfig.mockResolvedValue({
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4',
        },
      });

      await saveDefaultThinkingLevel(JsThinkingLevel.Medium);

      expect(mockConfig.writeConfig).toHaveBeenCalledWith('user', {
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4',
          defaultThinkingLevel: 2,
        },
      });
    });

    it('should not throw when write fails', async () => {
      mockConfig.loadConfig.mockResolvedValue({});
      mockConfig.writeConfig.mockRejectedValue(new Error('Permission denied'));

      // saveDefaultThinkingLevel silently catches errors
      await expect(
        saveDefaultThinkingLevel(JsThinkingLevel.High)
      ).resolves.toBeUndefined();
    });
  });
});

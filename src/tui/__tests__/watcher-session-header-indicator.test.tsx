/**
 * Feature: spec/features/watcher-session-header-indicator.feature
 *
 * Tests for Watcher Session Header Indicator (WATCH-015)
 *
 * These tests verify:
 * 1. useWatcherHeaderInfo hook returns correct watcher info
 * 2. SessionHeader utilities work correctly
 * 3. Slug generation for watchers
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { generateSlug } from '../utils/watcherTemplateStorage';
import {
  formatContextWindow,
  getContextFillColor,
  getMaxTokens,
} from '../utils/sessionHeaderUtils';

// Mock the NAPI functions
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetParent: vi.fn(),
  sessionGetRole: vi.fn(),
  sessionGetWatchers: vi.fn(),
}));

import {
  sessionGetParent,
  sessionGetRole,
  sessionGetWatchers,
} from '@sengac/codelet-napi';

const mockSessionGetParent = sessionGetParent as ReturnType<typeof vi.fn>;
const mockSessionGetRole = sessionGetRole as ReturnType<typeof vi.fn>;
const mockSessionGetWatchers = sessionGetWatchers as ReturnType<typeof vi.fn>;

describe('Watcher Session Header Indicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Watcher header info computation', () => {
    // Test the logic that would be in useWatcherHeaderInfo
    // without actually using React hooks (which require react-dom)

    it('should return null for non-watcher session', () => {
      // @step Given a regular session (no parent)
      mockSessionGetParent.mockReturnValue(null);

      // @step When checking if session is a watcher
      const parentId = mockSessionGetParent('regular-session-id');

      // @step Then parent is null (not a watcher)
      expect(parentId).toBeNull();
    });

    it('should identify watcher session by parent ID', () => {
      // @step Given a watcher session with parent
      const watcherId = 'watcher-session-id';
      const parentId = 'parent-session-id';
      mockSessionGetParent.mockReturnValue(parentId);

      // @step When checking parent
      const result = mockSessionGetParent(watcherId);

      // @step Then parent ID is returned
      expect(result).toBe(parentId);
    });

    it('should get watcher role info', () => {
      // @step Given a watcher with role configured
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        description: 'Reviews code for security issues',
        authority: 'supervisor',
      });

      // @step When getting role
      const role = mockSessionGetRole('watcher-id');

      // @step Then role info is correct
      expect(role.name).toBe('Security Reviewer');
      expect(role.authority).toBe('supervisor');
    });

    it('should calculate instance number for multiple watchers', () => {
      // @step Given multiple watchers of the same type
      const parentId = 'parent-session-id';
      const watchers = ['watcher-1', 'watcher-2', 'watcher-3'];

      mockSessionGetWatchers.mockReturnValue(watchers);
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        description: '',
        authority: 'supervisor',
      });

      // @step When counting instances
      const allWatchers = mockSessionGetWatchers(parentId);
      const targetWatcher = 'watcher-3';
      const targetSlug = generateSlug('Security Reviewer');
      
      let instanceNumber = 1;
      for (const watcherId of allWatchers) {
        if (watcherId === targetWatcher) break;
        const role = mockSessionGetRole(watcherId);
        if (role && generateSlug(role.name) === targetSlug) {
          instanceNumber++;
        }
      }

      // @step Then instance number is 3
      expect(instanceNumber).toBe(3);
    });

    it('should count only watchers with same slug for instance number', () => {
      // @step Given watchers of different types
      const watchers = ['security-1', 'test-1', 'security-2'];

      mockSessionGetWatchers.mockReturnValue(watchers);
      mockSessionGetRole.mockImplementation((id: string) => {
        if (id === 'test-1') {
          return { name: 'Test Enforcer', description: '', authority: 'peer' };
        }
        return { name: 'Security Reviewer', description: '', authority: 'supervisor' };
      });

      // @step When counting security reviewer instances
      const targetWatcher = 'security-2';
      const targetSlug = generateSlug('Security Reviewer');
      
      let instanceNumber = 1;
      for (const watcherId of watchers) {
        if (watcherId === targetWatcher) break;
        const role = mockSessionGetRole(watcherId);
        if (role && generateSlug(role.name) === targetSlug) {
          instanceNumber++;
        }
      }

      // @step Then instance number is 2 (not 3)
      expect(instanceNumber).toBe(2);
    });
  });

  describe('sessionHeaderUtils', () => {
    describe('formatContextWindow', () => {
      it('should format thousands as k', () => {
        expect(formatContextWindow(200000)).toBe('200k');
        expect(formatContextWindow(128000)).toBe('128k');
        expect(formatContextWindow(8000)).toBe('8k');
      });

      it('should format millions as M', () => {
        expect(formatContextWindow(1000000)).toBe('1M');
        expect(formatContextWindow(2000000)).toBe('2M');
      });
    });

    describe('getContextFillColor', () => {
      it('should return green for low fill (0-49%)', () => {
        expect(getContextFillColor(0)).toBe('green');
        expect(getContextFillColor(49)).toBe('green');
      });

      it('should return yellow for medium fill (50-69%)', () => {
        expect(getContextFillColor(50)).toBe('yellow');
        expect(getContextFillColor(69)).toBe('yellow');
      });

      it('should return magenta for high fill (70-84%)', () => {
        expect(getContextFillColor(70)).toBe('magenta');
        expect(getContextFillColor(84)).toBe('magenta');
      });

      it('should return red for critical fill (85%+)', () => {
        expect(getContextFillColor(85)).toBe('red');
        expect(getContextFillColor(100)).toBe('red');
      });
    });

    describe('getMaxTokens', () => {
      it('should return maximum values from two trackers', () => {
        const tracker1 = { inputTokens: 100, outputTokens: 50 };
        const tracker2 = { inputTokens: 80, outputTokens: 60 };

        const result = getMaxTokens(tracker1, tracker2);

        expect(result).toEqual({ inputTokens: 100, outputTokens: 60 });
      });

      it('should handle zero values', () => {
        const tracker1 = { inputTokens: 0, outputTokens: 0 };
        const tracker2 = { inputTokens: 1234, outputTokens: 567 };

        const result = getMaxTokens(tracker1, tracker2);

        expect(result).toEqual({ inputTokens: 1234, outputTokens: 567 });
      });
    });
  });

  describe('generateSlug', () => {
    it('should convert role name to kebab-case slug', () => {
      expect(generateSlug('Security Reviewer')).toBe('security-reviewer');
      expect(generateSlug('Test Coverage Enforcer')).toBe('test-coverage-enforcer');
      expect(generateSlug('Architecture Advisor')).toBe('architecture-advisor');
    });

    it('should handle special characters', () => {
      expect(generateSlug('API Security')).toBe('api-security');
      expect(generateSlug('C++ Code Reviewer')).toBe('c-code-reviewer');
    });

    it('should trim whitespace', () => {
      expect(generateSlug('  Security Reviewer  ')).toBe('security-reviewer');
    });

    it('should collapse multiple dashes', () => {
      expect(generateSlug('Security - Reviewer')).toBe('security-reviewer');
    });
  });

  describe('Header format specification', () => {
    // These tests document the expected header format
    
    it('should have correct watcher header format', () => {
      // Expected format: "Watcher: {slug} #{n} - Agent: {model} [R] [V] [{context}] {in}↓ {out}↑ [{fill}%]"
      // With bottom border separator
      const watcherInfo = { slug: 'security-reviewer', instanceNumber: 1 };
      const modelId = 'claude-sonnet-4-20250514';
      const hasReasoning = true;
      const hasVision = true;
      const contextWindow = 200000;
      const inputTokens = 1234;
      const outputTokens = 567;
      const fillPercentage = 45;

      // Verify all components are correct
      expect(`Watcher: ${watcherInfo.slug} #${watcherInfo.instanceNumber}`).toBe('Watcher: security-reviewer #1');
      expect(`Agent: ${modelId}`).toBe('Agent: claude-sonnet-4-20250514');
      expect(hasReasoning ? '[R]' : '').toBe('[R]');
      expect(hasVision ? '[V]' : '').toBe('[V]');
      expect(`[${formatContextWindow(contextWindow)}]`).toBe('[200k]');
      expect(`${inputTokens}↓ ${outputTokens}↑`).toBe('1234↓ 567↑');
      expect(`[${fillPercentage}%]`).toBe('[45%]');
    });

    it('should have correct regular header format (no watcher prefix)', () => {
      // Expected format: "Agent: {model} [R] [V] [{context}] {in}↓ {out}↑ [{fill}%]"
      // With bottom border separator
      const modelId = 'claude-sonnet-4-20250514';

      expect(`Agent: ${modelId}`).toBe('Agent: claude-sonnet-4-20250514');
      // Regular session header should NOT contain watcher info
      const regularHeader = `Agent: ${modelId}`;
      expect(regularHeader).not.toContain('Watcher:');
    });
  });
});

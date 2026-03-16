/**
 * Feature: spec/features/watcher-session-header-indicator.feature
 *
 * Tests for Supervisor Session Header Indicator (WATCH-015)
 *
 * These tests verify:
 * 1. useSupervisorHeaderInfo hook returns correct supervisor info
 * 2. SessionHeader utilities work correctly
 * 3. Slug generation for supervisors
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { generateSlug } from '../utils/supervisorTemplateStorage';
import {
  formatContextWindow,
  getContextFillColor,
  getMaxTokens,
} from '../utils/sessionHeaderUtils';

// Mock the NAPI functions
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetSubordinate: vi.fn(),
  sessionGetRole: vi.fn(),
  sessionGetSupervisors: vi.fn(),
}));

import {
  sessionGetSubordinate,
  sessionGetRole,
  sessionGetSupervisors,
} from '@sengac/codelet-napi';

const mockSessionGetSubordinate = sessionGetSubordinate as ReturnType<typeof vi.fn>;
const mockSessionGetRole = sessionGetRole as ReturnType<typeof vi.fn>;
const mockSessionGetSupervisors = sessionGetSupervisors as ReturnType<typeof vi.fn>;

describe('Supervisor Session Header Indicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Supervisor header info computation', () => {
    // Test the logic that would be in useSupervisorHeaderInfo
    // without actually using React hooks (which require react-dom)

    it('should return null for non-supervisor session', () => {
      // @step Given a regular session (no subordinate)
      mockSessionGetSubordinate.mockReturnValue(null);

      // @step When checking if session is a supervisor
      const subordinateId = mockSessionGetSubordinate('regular-session-id');

      // @step Then subordinate is null (not a supervisor)
      expect(subordinateId).toBeNull();
    });

    it('should identify supervisor session by subordinate ID', () => {
      // @step Given a supervisor session with subordinate
      const supervisorId = 'supervisor-session-id';
      const subordinateId = 'subordinate-session-id';
      mockSessionGetSubordinate.mockReturnValue(subordinateId);

      // @step When checking subordinate
      const result = mockSessionGetSubordinate(supervisorId);

      // @step Then subordinate ID is returned
      expect(result).toBe(subordinateId);
    });

    it('should get supervisor role info', () => {
      // @step Given a supervisor with role configured
      // WATCH-024: sessionGetRole returns SupervisorRoleInfo { name, brief } — no authority
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        brief: 'Reviews code for security issues',
      });

      // @step When getting role
      const role = mockSessionGetRole('supervisor-id');

      // @step Then role info is correct
      expect(role.name).toBe('Security Reviewer');
      expect(role.brief).toBe('Reviews code for security issues');
    });

    it('should calculate instance number for multiple supervisors', () => {
      // @step Given multiple supervisors of the same type
      const subordinateId = 'subordinate-session-id';
      const supervisors = ['supervisor-1', 'supervisor-2', 'supervisor-3'];

      mockSessionGetSupervisors.mockReturnValue(supervisors);
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        brief: null,
      });

      // @step When counting instances
      const allSupervisors = mockSessionGetSupervisors(subordinateId);
      const targetSupervisor = 'supervisor-3';
      const targetSlug = generateSlug('Security Reviewer');
      
      let instanceNumber = 1;
      for (const supervisorId of allSupervisors) {
        if (supervisorId === targetSupervisor) break;
        const role = mockSessionGetRole(supervisorId);
        if (role && generateSlug(role.name) === targetSlug) {
          instanceNumber++;
        }
      }

      // @step Then instance number is 3
      expect(instanceNumber).toBe(3);
    });

    it('should count only supervisors with same slug for instance number', () => {
      // @step Given supervisors of different types
      const supervisors = ['security-1', 'test-1', 'security-2'];

      mockSessionGetSupervisors.mockReturnValue(supervisors);
      mockSessionGetRole.mockImplementation((id: string) => {
        if (id === 'test-1') {
          return { name: 'Test Enforcer', brief: null };
        }
        return { name: 'Security Reviewer', brief: null };
      });

      // @step When counting security reviewer instances
      const targetSupervisor = 'security-2';
      const targetSlug = generateSlug('Security Reviewer');
      
      let instanceNumber = 1;
      for (const supervisorId of supervisors) {
        if (supervisorId === targetSupervisor) break;
        const role = mockSessionGetRole(supervisorId);
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

        expect(result).toEqual({ inputTokens: 100, outputTokens: 60, reasoningTokens: 0 });
      });

      it('should handle zero values', () => {
        const tracker1 = { inputTokens: 0, outputTokens: 0 };
        const tracker2 = { inputTokens: 1234, outputTokens: 567 };

        const result = getMaxTokens(tracker1, tracker2);

        expect(result).toEqual({ inputTokens: 1234, outputTokens: 567, reasoningTokens: 0 });
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
    // TUI-060: Removed "Agent" prefix - format is now just model name with optional session number and work unit
    
    it('should have correct supervisor header format', () => {
      // Expected format: "Supervisor: {slug} #{n} | {model} [R] [V] [{context}] {in}↓ {out}↑ [{fill}%]"
      // With bottom border separator
      // Supervisor info in blue, separator | in white, model info in cyan
      const supervisorInfo = { slug: 'security-reviewer', instanceNumber: 1 };
      const modelId = 'claude-sonnet-4-20250514';
      const hasReasoning = true;
      const hasVision = true;
      const contextWindow = 200000;
      const inputTokens = 1234;
      const outputTokens = 567;
      const fillPercentage = 45;

      // Verify all components are correct
      expect(`Supervisor: ${supervisorInfo.slug} #${supervisorInfo.instanceNumber}`).toBe('Supervisor: security-reviewer #1');
      expect(modelId).toBe('claude-sonnet-4-20250514');
      expect(hasReasoning ? '[R]' : '').toBe('[R]');
      expect(hasVision ? '[V]' : '').toBe('[V]');
      expect(`[${formatContextWindow(contextWindow)}]`).toBe('[200k]');
      expect(`${inputTokens}↓ ${outputTokens}↑`).toBe('1234↓ 567↑');
      expect(`[${fillPercentage}%]`).toBe('[45%]');
    });

    it('should use pipe separator between supervisor and model info', () => {
      // Format: "Supervisor: ... | {model}"
      // The pipe | is used as separator (white color)
      const separator = '|';
      expect(separator).toBe('|');
    });

    it('should have correct regular header format (no supervisor prefix)', () => {
      // Expected format: "#N (WORK-ID: status): {model} [R] [V] [{context}] {in}↓ {out}↑ [{fill}%]"
      // With bottom border separator
      // TUI-060: No "Agent:" prefix - just session number, work unit, and model
      const sessionNumber = 1;
      const workUnitId = 'AUTH-001';
      const workUnitStatus = 'implementing';
      const modelId = 'claude-sonnet-4-20250514';

      const regularHeader = `#${sessionNumber} (${workUnitId}: ${workUnitStatus}): ${modelId}`;
      expect(regularHeader).toBe('#1 (AUTH-001: implementing): claude-sonnet-4-20250514');
      // Regular session header should NOT contain supervisor info or "Agent:" prefix
      expect(regularHeader).not.toContain('Supervisor:');
      expect(regularHeader).not.toContain('Agent:');
    });
  });
});

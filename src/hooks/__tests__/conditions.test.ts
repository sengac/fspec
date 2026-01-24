/**
 * Feature: spec/features/hook-condition-evaluation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import type { HookDefinition, HookContext } from '../types';
import type { WorkUnit } from '../../types/index';
import { evaluateHookCondition } from '../conditions';

describe('Feature: Hook condition evaluation', () => {
  describe('Scenario: Hook with no condition always matches', () => {
    it('should match any context', () => {
      // Given I have a hook with no condition
      const hook: HookDefinition = {
        name: 'always-run',
        command: 'spec/hooks/always.sh',
      };

      // And I have a work unit context "AUTH-001"
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with tag condition matches work unit with matching tag', () => {
    it('should match when work unit has the tag', () => {
      // Given I have a hook with condition tags ["@security"]
      const hook: HookDefinition = {
        name: 'security-check',
        command: 'spec/hooks/security.sh',
        condition: {
          tags: ['@security'],
        },
      };

      // And I have a work unit "AUTH-001" with tags ["@security", "@critical"]
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        tags: ['@security', '@critical'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with tag condition does not match work unit without tag', () => {
    it('should not match when work unit lacks the tag', () => {
      // Given I have a hook with condition tags ["@security"]
      const hook: HookDefinition = {
        name: 'security-check',
        command: 'spec/hooks/security.sh',
        condition: {
          tags: ['@security'],
        },
      };

      // And I have a work unit "DASH-001" with tags ["@ui", "@critical"]
      const context: HookContext = {
        workUnitId: 'DASH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'DASH-001',
        title: 'Test',
        status: 'implementing',
        tags: ['@ui', '@critical'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should not match
      expect(shouldRun).toBe(false);
    });
  });

  describe('Scenario: Hook with prefix condition matches work unit with matching prefix', () => {
    it('should match when work unit ID starts with one of the prefixes', () => {
      // Given I have a hook with condition prefix ["AUTH", "SEC"]
      const hook: HookDefinition = {
        name: 'auth-hook',
        command: 'spec/hooks/auth.sh',
        condition: {
          prefix: ['AUTH', 'SEC'],
        },
      };

      // And I have a work unit "AUTH-001"
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with prefix condition does not match work unit with different prefix', () => {
    it('should not match when work unit ID has different prefix', () => {
      // Given I have a hook with condition prefix ["AUTH", "SEC"]
      const hook: HookDefinition = {
        name: 'auth-hook',
        command: 'spec/hooks/auth.sh',
        condition: {
          prefix: ['AUTH', 'SEC'],
        },
      };

      // And I have a work unit "DASH-001"
      const context: HookContext = {
        workUnitId: 'DASH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'DASH-001',
        title: 'Test',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should not match
      expect(shouldRun).toBe(false);
    });
  });

  describe('Scenario: Hook with epic condition matches work unit in that epic', () => {
    it('should match when work unit belongs to the epic', () => {
      // Given I have a hook with condition epic "user-management"
      const hook: HookDefinition = {
        name: 'epic-hook',
        command: 'spec/hooks/epic.sh',
        condition: {
          epic: 'user-management',
        },
      };

      // And I have a work unit "AUTH-001" in epic "user-management"
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        epic: 'user-management',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with estimate range matches work unit within range', () => {
    it('should match when work unit estimate is within range', () => {
      // Given I have a hook with condition estimateMin 5 and estimateMax 13
      const hook: HookDefinition = {
        name: 'large-story-hook',
        command: 'spec/hooks/large.sh',
        condition: {
          estimateMin: 5,
          estimateMax: 13,
        },
      };

      // And I have a work unit "AUTH-001" with estimate 8
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        estimate: 8,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with multiple conditions uses AND logic', () => {
    it('should match when all conditions are met', () => {
      // Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
      const hook: HookDefinition = {
        name: 'multi-condition-hook',
        command: 'spec/hooks/multi.sh',
        condition: {
          tags: ['@security'],
          prefix: ['AUTH'],
        },
      };

      // And I have a work unit "AUTH-001" with tags ["@security"]
      const context: HookContext = {
        workUnitId: 'AUTH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'AUTH-001',
        title: 'Test',
        status: 'implementing',
        tags: ['@security'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should match because both conditions are met
      expect(shouldRun).toBe(true);
    });
  });

  describe('Scenario: Hook with multiple conditions fails if any condition is not met', () => {
    it('should not match when any condition fails', () => {
      // Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
      const hook: HookDefinition = {
        name: 'multi-condition-hook',
        command: 'spec/hooks/multi.sh',
        condition: {
          tags: ['@security'],
          prefix: ['AUTH'],
        },
      };

      // And I have a work unit "DASH-001" with tags ["@security"]
      const context: HookContext = {
        workUnitId: 'DASH-001',
        event: 'post-implementing',
        timestamp: new Date().toISOString(),
      };

      const workUnit: WorkUnit = {
        id: 'DASH-001',
        title: 'Test',
        status: 'implementing',
        tags: ['@security'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, workUnit);

      // Then the hook should not match because prefix does not match
      expect(shouldRun).toBe(false);
    });
  });

  describe('Scenario: Context without work unit ID only matches unconditional hooks', () => {
    it('should not match conditional hooks when no work unit ID', () => {
      // Given I have a hook with condition tags ["@security"]
      const hook: HookDefinition = {
        name: 'security-check',
        command: 'spec/hooks/security.sh',
        condition: {
          tags: ['@security'],
        },
      };

      // And I have a context with no work unit ID
      const context: HookContext = {
        event: 'pre-start',
        timestamp: new Date().toISOString(),
      };

      // When I evaluate if the hook should run
      const shouldRun = evaluateHookCondition(hook, context, null);

      // Then the hook should not match
      expect(shouldRun).toBe(false);
    });
  });
});

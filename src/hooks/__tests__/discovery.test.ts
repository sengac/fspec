/**
 * Feature: spec/features/hook-discovery-and-event-naming.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import type { HookConfig, HookDefinition } from '../types.js';
import { discoverHooks, generateEventNames } from '../discovery.js';

describe('Feature: Hook discovery and event naming', () => {
  describe('Scenario: Discover hooks for specific event', () => {
    it('should return matching hooks in config order', () => {
      // Given I have a hook configuration with hooks for "post-implementing"
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            { name: 'lint', command: 'spec/hooks/lint.sh' },
            { name: 'test', command: 'spec/hooks/test.sh' },
          ],
        },
      };

      // When I discover hooks for event "post-implementing"
      const hooks = discoverHooks(config, 'post-implementing');

      // Then I should receive an array of matching hooks
      expect(hooks).toHaveLength(2);

      // And the hooks should be returned in config order
      expect(hooks[0].name).toBe('lint');
      expect(hooks[1].name).toBe('test');
    });
  });

  describe('Scenario: Discover special lifecycle hooks', () => {
    it('should return pre-start lifecycle hooks', () => {
      // Given I have a hook configuration with "pre-start" lifecycle hooks
      const config: HookConfig = {
        hooks: {
          'pre-start': [{ name: 'setup', command: 'spec/hooks/setup.sh' }],
        },
      };

      // When I discover hooks for event "pre-start"
      const hooks = discoverHooks(config, 'pre-start');

      // Then I should receive the pre-start hooks
      expect(hooks).toHaveLength(1);
      expect(hooks[0].name).toBe('setup');

      // And these hooks should execute before any command logic
      // (This is a documentation assertion - verified by integration tests)
    });
  });

  describe('Scenario: Discover hooks for non-existent event', () => {
    it('should return empty array without error', () => {
      // Given I have a hook configuration
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            { name: 'lint', command: 'spec/hooks/lint.sh' },
          ],
        },
      };

      // When I discover hooks for event "pre-xyz"
      const hooks = discoverHooks(config, 'pre-xyz');

      // Then I should receive an empty array
      expect(hooks).toEqual([]);

      // And no error should be thrown
      // (This test passing proves no error was thrown)
    });
  });

  describe('Scenario: Generate event names from command name', () => {
    it('should generate pre- and post- event names', () => {
      // Given the command name is "update-work-unit-status"
      const commandName = 'update-work-unit-status';

      // When I generate event names for this command
      const eventNames = generateEventNames(commandName);

      // Then the pre-event name should be "pre-update-work-unit-status"
      expect(eventNames.pre).toBe('pre-update-work-unit-status');

      // And the post-event name should be "post-update-work-unit-status"
      expect(eventNames.post).toBe('post-update-work-unit-status');
    });
  });

  describe('Scenario: Event names are case-sensitive', () => {
    it('should not match hooks with different case', () => {
      // Given I have a hook configuration with hooks for "post-implementing"
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            { name: 'lint', command: 'spec/hooks/lint.sh' },
          ],
        },
      };

      // When I discover hooks for event "post-Implementing"
      const hooks = discoverHooks(config, 'post-Implementing');

      // Then I should receive an empty array
      expect(hooks).toEqual([]);

      // And the hooks for "post-implementing" should not match
      const correctHooks = discoverHooks(config, 'post-implementing');
      expect(correctHooks).toHaveLength(1);
    });
  });

  describe('Scenario: Multiple hooks for same event', () => {
    it('should return all hooks in config order', () => {
      // Given I have a hook configuration with multiple hooks for "post-testing"
      // And the hooks are named "unit-tests", "integration-tests", "e2e-tests"
      const config: HookConfig = {
        hooks: {
          'post-testing': [
            { name: 'unit-tests', command: 'spec/hooks/unit.sh' },
            { name: 'integration-tests', command: 'spec/hooks/integration.sh' },
            { name: 'e2e-tests', command: 'spec/hooks/e2e.sh' },
          ],
        },
      };

      // When I discover hooks for event "post-testing"
      const hooks = discoverHooks(config, 'post-testing');

      // Then I should receive all three hooks
      expect(hooks).toHaveLength(3);

      // And the hooks should be in the order they appear in config
      expect(hooks[0].name).toBe('unit-tests');
      expect(hooks[1].name).toBe('integration-tests');
      expect(hooks[2].name).toBe('e2e-tests');
    });
  });
});

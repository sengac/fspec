/**
 * Feature: spec/features/implement-discover-foundation-command.feature
 *
 * This test file validates the acceptance criteria for discover-foundation command.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import {
  discoverFoundation,
  analyzeCodebase,
  emitDiscoveryReminder,
} from '../discover-foundation';

describe('Feature: Implement discover-foundation Command', () => {
  describe('Scenario: Emit system-reminder after code analysis detects personas', () => {
    it('should emit system-reminder with detected personas list', async () => {
      // Given I run discover-foundation command
      // And code analysis detects 3 personas: End User, Admin, API Consumer
      const discoveryResult = analyzeCodebase();
      expect(discoveryResult.personas).toHaveLength(3);

      // When code analysis completes
      const systemReminder = emitDiscoveryReminder(discoveryResult);

      // Then command should emit system-reminder with detected personas
      expect(systemReminder).toContain('<system-reminder>');
      expect(systemReminder).toContain('</system-reminder>');

      // And system-reminder should list all 3 detected personas
      expect(systemReminder).toContain('End User');
      expect(systemReminder).toContain('Admin');
      expect(systemReminder).toContain('API Consumer');

      // And system-reminder should guide AI to review in questionnaire
      expect(systemReminder).toContain('Review in questionnaire');
    });
  });

  describe('Scenario: Generate validated foundation.json after questionnaire', () => {
    it('should create validated foundation.json with questionnaire answers', async () => {
      // Given I complete the questionnaire with all required answers
      // When discover-foundation finishes
      const result = await discoverFoundation();

      // Then foundation.json should be created
      expect(result.foundation).toBeDefined();

      // And foundation.json should pass schema validation
      expect(result.valid).toBe(true);

      // And foundation.json should contain questionnaire answers
      expect(result.foundation.project.vision).toBeDefined();
      expect(result.foundation.problemSpace.primaryProblem).toBeDefined();
      expect(result.foundation.solutionSpace.capabilities).toBeDefined();
      expect(result.foundation.personas).toBeDefined();
    });
  });
});

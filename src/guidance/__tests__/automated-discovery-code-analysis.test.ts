/**
 * Feature: spec/features/project-type-detection-incorrectly-identifies-cli-tools-as-web-apps.feature
 * Bug: FOUND-010
 *
 * This test verifies that automated detection code has been REMOVED.
 * discover-foundation must be 100% interactive questionnaire.
 */

import { describe, it, expect } from 'vitest';
import { existsSync } from 'fs';
import { join } from 'path';

describe('Feature: Remove automated project type detection', () => {
  describe('Scenario: Remove automated detection code', () => {
    it('should verify automated-discovery-code-analysis.ts is deleted', () => {
      // Given automated detection code should be removed
      const filePath = join(__dirname, '../automated-discovery-code-analysis.ts');

      // When checking if file exists
      const fileExists = existsSync(filePath);

      // Then file should NOT exist (deleted)
      expect(fileExists).toBe(false);

      // And discover-foundation must work without it (verified by integration test)
    });
  });
});


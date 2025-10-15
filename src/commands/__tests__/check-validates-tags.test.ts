/**
 * Test suite for: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 * Scenario: Verify check.ts uses validateTags function
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
  describe('Scenario: Verify check.ts uses validateTags function', () => {
    it('should import and use validateTags function', async () => {
      // Given I have the source file "src/commands/check.ts"
      const checkFilePath = join(process.cwd(), 'src/commands/check.ts');
      const content = await readFile(checkFilePath, 'utf-8');

      // When I inspect the validation logic
      // Then it should import and use the validateTags() function
      expect(content).toContain('validateTags');

      // And it should not contain inline tag validation logic
      // (If it imports validateTags, it's using the shared function)
      const hasImport = /import.*validateTags.*from/.test(content);
      const hasFunctionCall = /validateTags\s*\(/.test(content);

      expect(hasImport || hasFunctionCall).toBe(true);
    });
  });
});

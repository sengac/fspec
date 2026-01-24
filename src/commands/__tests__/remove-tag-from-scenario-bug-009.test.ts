/**
 * Feature: spec/features/scenario-tag-removal.feature
 *
 * This test file validates the acceptance criteria for BUG-009: fix remove-tag-from-scenario command.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { removeTagFromScenario } from '../remove-tag-from-scenario';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Scenario Tag Removal (BUG-009)', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('bug-009');
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Remove single tag from scenario', () => {
    it('should actually remove the tag from the scenario line', async () => {
      // Given I have a feature file with a scenario that has a single tag @COV-010
      const featureContent = `Feature: User Login

  @COV-010
  Scenario: Login scenario
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
      const result = await removeTagFromScenario(
        'spec/features/test.feature',
        'Login scenario',
        ['@COV-010'],
        { cwd: testDir }
      );

      // Then the tag @COV-010 should be removed from the scenario line
      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@COV-010');

      // And the command should exit with code 0
      expect(result.success).toBe(true);

      // And the feature file should have valid Gherkin syntax
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Remove one of multiple tags from scenario', () => {
    it('should remove only the specified tag and preserve others', async () => {
      // Given I have a feature file with a scenario that has tags @COV-010 @wip @critical
      const featureContent = `Feature: User Login

  @COV-010
  @wip
  @critical
  Scenario: Login scenario
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
      const result = await removeTagFromScenario(
        'spec/features/test.feature',
        'Login scenario',
        ['@COV-010'],
        { cwd: testDir }
      );

      // Then the tag @COV-010 should be removed from the scenario
      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(updatedContent).not.toContain('@COV-010');

      // And the tags @wip and @critical should remain on the scenario
      expect(updatedContent).toContain('@wip');
      expect(updatedContent).toContain('@critical');

      // And the command should exit with code 0
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Remove tag from scenario when scenario not found', () => {
    it('should display warning and exit with code 0 (idempotent)', async () => {
      // Given I have a feature file without a scenario named "Nonexistent Scenario"
      const featureContent = `Feature: User Login

  Scenario: Existing Scenario
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Nonexistent Scenario" @COV-010'
      const result = await removeTagFromScenario(
        'spec/features/test.feature',
        'Nonexistent Scenario',
        ['@COV-010'],
        { cwd: testDir }
      );

      // Then the command should display a warning message
      // EXPECTED: success=true with warning message
      // ACTUAL: success=false with error
      expect(result.success).toBe(true); // BUG: Currently false, should be true
      expect(result.message).toContain('Nonexistent Scenario');
      expect(result.message).toContain('not found');

      // And the command should exit with code 0 (idempotent behavior)
      // This is Rule 7 from BUG-009
    });
  });

  describe("Scenario: Remove tag that doesn't exist on scenario", () => {
    it('should succeed (idempotent behavior)', async () => {
      // Given I have a feature file with a scenario that has only @wip tag
      const featureContent = `Feature: User Login

  @wip
  Scenario: Login scenario
    Given test
`;
      await writeFile(
        join(testDir, 'spec/features/test.feature'),
        featureContent
      );

      // When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
      const result = await removeTagFromScenario(
        'spec/features/test.feature',
        'Login scenario',
        ['@COV-010'],
        { cwd: testDir }
      );

      // Then the command should succeed (idempotent behavior)
      // BUG: Currently fails because tag doesn't exist
      // This is Rule 3 from BUG-009: "If tag doesn't exist on scenario, command should still succeed (idempotent)"
      expect(result.success).toBe(true); // BUG: Currently false, should be true

      // And the command should exit with code 0
      // (idempotent behavior - removing nonexistent tag is success)
      expect(result.message).toContain('No changes'); // Should indicate no changes made

      // And the @wip tag should remain on the scenario
      const updatedContent = await readFile(
        join(testDir, 'spec/features/test.feature'),
        'utf-8'
      );
      expect(updatedContent).toContain('@wip');
    });
  });
});

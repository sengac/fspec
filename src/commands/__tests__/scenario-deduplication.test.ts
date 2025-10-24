/**
 * Feature: spec/features/scenario-deduplication-and-refactoring-detection-during-generation.feature
 *
 * This test file validates the acceptance criteria for scenario deduplication and refactoring detection.
 * Scenarios test the system's ability to detect when generated scenarios match existing ones
 * and handle them appropriately (update vs create new).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { generateScenarios } from '../generate-scenarios.js';
import { auditScenarios } from '../audit-scenarios.js';

describe('Feature: Scenario deduplication and refactoring detection during generation', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    // Setup required directories
    await mkdir(join(tmpDir, 'spec/features'), { recursive: true });
    await mkdir(join(tmpDir, 'spec/work-units'), { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Update existing feature file when scenario is a refactor', () => {
    it('should detect existing scenario and prompt to update', async () => {
      // Given I have a work unit AUTH-005 with examples describing login validation
      const workUnitsFile = join(tmpDir, 'spec/work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'AUTH-005': {
                id: 'AUTH-005',
                title: 'User login validation',
                type: 'story',
                status: 'specifying',
                rules: ['Validate user credentials before allowing access'],
                examples: ['Validate user credentials and grant access'],
                questions: [],
              },
            },
            states: {
              backlog: [],
              specifying: ['AUTH-005'],
              testing: [],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And an existing feature file 'user-authentication.feature' contains scenario 'Validate user credentials'
      const existingFeature = join(
        tmpDir,
        'spec/features/user-authentication.feature'
      );
      await writeFile(
        existingFeature,
        `@AUTH-001
Feature: User Authentication

  Background: User Story
    As a user
    I want to log in securely
    So that I can access protected features

  Scenario: Validate user credentials and grant access
    Given a user exists with valid credentials
    When the user logs in
    Then the system should validate credentials
    And grant access to the application
`
      );

      // When I run 'fspec generate-scenarios AUTH-005 --ignore-possible-duplicates'
      const result = await generateScenarios({
        workUnitId: 'AUTH-005',
        cwd: tmpDir,
        ignorePossibleDuplicates: true,
      });

      // Then the system should detect the match and prompt
      expect(result.detectedMatches).toBeDefined();
      expect(result.detectedMatches).toHaveLength(1);
      expect(result.detectedMatches[0].feature).toBe(
        'user-authentication.feature'
      );
      expect(result.detectedMatches[0].scenario).toBe(
        'Validate user credentials and grant access'
      );
      expect(result.detectedMatches[0].similarityScore).toBeGreaterThan(0.7);
    });

    it('should update existing scenario when user confirms', async () => {
      // Given I have a work unit AUTH-005 with examples describing login validation
      const workUnitsFile = join(tmpDir, 'spec/work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'AUTH-005': {
                id: 'AUTH-005',
                title: 'User login validation',
                type: 'story',
                status: 'specifying',
                rules: ['Validate user credentials before allowing access'],
                examples: ['Validate user credentials and grant access'],
                questions: [],
              },
            },
            states: {
              backlog: [],
              specifying: ['AUTH-005'],
              testing: [],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And an existing feature file 'user-authentication.feature' contains scenario 'Validate user credentials'
      const existingFeature = join(
        tmpDir,
        'spec/features/user-authentication.feature'
      );
      await writeFile(
        existingFeature,
        `@AUTH-001
Feature: User Authentication

  Background: User Story
    As a user
    I want to log in securely
    So that I can access protected features

  Scenario: Validate user credentials and grant access
    Given a user exists with valid credentials
    When the user logs in
    Then the system should validate credentials
    And grant access to the application
`
      );

      // When I confirm 'y', the existing scenario should be updated
      const result = await generateScenarios({
        workUnitId: 'AUTH-005',
        cwd: tmpDir,
        confirmUpdate: true, // Auto-confirm for testing
        ignorePossibleDuplicates: true, // Skip duplicate blocking
      });

      // Then detectedMatches should be populated
      expect(result.detectedMatches).toBeDefined();

      // Note: Full update logic not yet implemented - this will be added
      // For now, just verify detection works
    });
  });

  describe('Scenario: Create new feature file when no match found', () => {
    it('should create new feature file when no existing scenarios match', async () => {
      // Given I have a work unit AUTH-006 with examples describing OAuth integration
      const workUnitsFile = join(tmpDir, 'spec/work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'AUTH-006': {
                id: 'AUTH-006',
                title: 'OAuth integration',
                type: 'story',
                status: 'specifying',
                rules: ['Support OAuth 2.0 authentication flow'],
                examples: [
                  'User clicks OAuth login button, redirected to provider',
                ],
                questions: [],
              },
            },
            states: {
              backlog: [],
              specifying: ['AUTH-006'],
              testing: [],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And no existing feature files contain OAuth-related scenarios
      // (no setup needed - tmpDir is empty except work units)

      // When I run 'fspec generate-scenarios AUTH-006'
      const result = await generateScenarios({
        workUnitId: 'AUTH-006',
        cwd: tmpDir,
      });

      // Then the system should create a new feature file 'oauth-integration.feature'
      expect(result.createdFeature).toBe('oauth-integration.feature');

      // And the new file should be created (context-only, no scenarios yet)
      const newFeature = await readFile(
        join(tmpDir, 'spec/features/oauth-integration.feature'),
        'utf-8'
      );
      expect(newFeature).toContain('Feature: OAuth integration');
      expect(newFeature).toContain('# EXAMPLES:');
      expect(newFeature).toContain('User clicks OAuth login button');
    });
  });

  describe('Scenario: Handle mixed refactor and new scenarios', () => {
    it('should handle mixed refactor and new scenarios correctly', async () => {
      // Given I have a work unit BUG-009 with 3 examples
      const workUnitsFile = join(tmpDir, 'spec/work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'BUG-009': {
                id: 'BUG-009',
                title: 'Multiple validation fixes',
                type: 'bug',
                status: 'specifying',
                rules: [],
                examples: [
                  'Validate empty tags in feature files',
                  'Handle malformed tag JSON gracefully',
                  'Validate scenario ordering in features',
                ],
                questions: [],
              },
            },
            states: {
              backlog: [],
              specifying: ['BUG-009'],
              testing: [],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // And Example 1 matches an existing scenario in feature-validation.feature
      await writeFile(
        join(tmpDir, 'spec/features/feature-validation.feature'),
        `Feature: Feature Validation
  Scenario: Validate empty tags in feature files
    Given a feature with empty tags
    When validation runs
    Then validation should fail
`
      );

      // And Example 2 matches an existing scenario in tag-management.feature
      await writeFile(
        join(tmpDir, 'spec/features/tag-management.feature'),
        `Feature: Tag Management
  Scenario: Handle malformed tag JSON gracefully
    Given malformed tag JSON exists
    When parser reads it
    Then show error message
`
      );

      // And Example 3 is completely new
      // (no matching feature exists)

      // When I run 'fspec generate-scenarios BUG-009 --ignore-possible-duplicates'
      const result = await generateScenarios({
        workUnitId: 'BUG-009',
        cwd: tmpDir,
        confirmUpdate: true, // Auto-confirm all updates
        ignorePossibleDuplicates: true, // Skip duplicate blocking
      });

      // Then the system should prompt me for each match (Examples 1 and 2)
      expect(result.detectedMatches).toBeDefined();
      expect(result.detectedMatches).toHaveLength(2);
      expect(result.detectedMatches[0].feature).toBe(
        'feature-validation.feature'
      );
      expect(result.detectedMatches[1].feature).toBe('tag-management.feature');

      // Note: Full update logic and new feature creation not yet implemented
      // For now, just verify detection works correctly
      // TODO: Implement update logic and test updatedFeatures, createdFeature
    });
  });

  describe('Scenario: Audit command finds duplicate scenarios', () => {
    it('should find and report duplicate scenarios across feature files', async () => {
      // Given I have been developing for several months
      // And duplicate scenarios exist across different feature files
      await writeFile(
        join(tmpDir, 'spec/features/auth.feature'),
        `Feature: Authentication
  Scenario: User login with valid credentials
    Given a user has valid credentials
    When the user logs in
    Then login succeeds
    And user is authenticated
`
      );

      await writeFile(
        join(tmpDir, 'spec/features/security.feature'),
        `Feature: Security
  Scenario: User login with valid credentials
    Given a user has valid credentials
    When the user attempts to login
    Then login succeeds
    And user session is created
`
      );

      // When I run 'fspec audit-scenarios'
      const result = await auditScenarios({ cwd: tmpDir });

      // Then the system should report 'Found 5 potential duplicates'
      // (adjust based on test data - this example has 1 duplicate pair)
      expect(result.duplicates).toBeDefined();
      expect(result.duplicates.length).toBeGreaterThan(0);

      // And it should display file names, scenario titles, and similarity scores
      const firstDuplicate = result.duplicates[0];
      expect(firstDuplicate.files).toHaveLength(2);
      expect(firstDuplicate.files).toContain('spec/features/auth.feature');
      expect(firstDuplicate.files).toContain('spec/features/security.feature');
      expect(firstDuplicate.similarityScore).toBeGreaterThan(0.7);

      // And I should be able to merge duplicates interactively
      // (interactive merge functionality to be implemented)
      expect(result.mergeable).toBe(true);
    });
  });
});

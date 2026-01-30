/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Register tag command auto-creates spec/tags.json when missing
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, access } from 'fs/promises';
import { join } from 'path';
import { registerTag } from '../register-tag';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  ensureTestDirectory,
} from '../../test-helpers/test-file-operations';

describe('Feature: Automatic JSON File Initialization', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('register-tag-ensure');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Register tag command auto-creates spec/tags.json when missing', () => {
    it('should create tags.json with valid structure when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await ensureTestDirectory(join(setup.testDir, 'spec'));

      // Given spec/tags.json does not exist
      const tagsFile = join(setup.testDir, 'spec/tags.json');

      // When I run "fspec register-tag @my-tag 'Phase Tags' 'My custom tag'"
      const result = await registerTag(
        '@my-tag',
        'Phase Tags',
        'My custom tag',
        { cwd: setup.testDir }
      );

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And spec/tags.json should be created
      await access(tagsFile);

      // And the file should contain valid Tags JSON structure with default categories
      const fileContent = await readFile(tagsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.categories).toBeDefined();
      expect(data.categories.length).toBeGreaterThan(0);

      // And the tag @my-tag should be added to the Phase Tags category
      const phaseCategory = data.categories.find(
        (cat: any) => cat.name === 'Phase Tags'
      );
      expect(phaseCategory).toBeDefined();
      expect(phaseCategory.tags).toBeDefined();

      const myTag = phaseCategory.tags.find((t: any) => t.name === '@my-tag');
      expect(myTag).toBeDefined();
      expect(myTag.description).toBe('My custom tag');
    });
  });
});

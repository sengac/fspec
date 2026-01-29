/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: List tags command auto-creates spec/tags.json instead of throwing error
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { access } from 'fs/promises';
import { join } from 'path';
import { listTags } from '../list-tags';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Automatic JSON File Initialization', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('list-tags-ensure');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: List tags command auto-creates spec/tags.json instead of throwing error', () => {
    it('should not throw error and auto-create tags.json when missing', async () => {
      // Given I have a fresh project with spec/ directory
      const specDir = join(setup.testDir, 'spec');
      await import('fs/promises').then(fs =>
        fs.mkdir(specDir, { recursive: true })
      );

      // Given spec/tags.json does not exist
      const tagsFile = join(setup.testDir, 'spec/tags.json');

      // When I run "fspec list-tags"
      const result = await listTags({ cwd: setup.testDir });

      // Then the command should not throw "tags.json not found" error
      // (no exception thrown)

      // And the command should succeed
      expect(result).toBeDefined();
      expect(result.categories).toBeDefined();

      // And spec/tags.json should be auto-created with default structure
      await access(tagsFile);

      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(tagsFile, 'utf-8');
      const data = JSON.parse(fileContent);

      expect(data.categories).toBeDefined();
      expect(data.categories.length).toBeGreaterThan(0);
    });
  });
});

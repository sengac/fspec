/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: List tags command auto-creates spec/tags.json instead of throwing error
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { listTags } from '../list-tags';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List tags command auto-creates spec/tags.json instead of throwing error', () => {
    it('should not throw error and auto-create tags.json when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Given spec/tags.json does not exist
      const tagsFile = join(testDir, 'spec/tags.json');

      // When I run "fspec list-tags"
      const result = await listTags({ cwd: testDir });

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

/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Register tag command auto-creates spec/tags.json when missing
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { registerTag } from '../register-tag';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Register tag command auto-creates spec/tags.json when missing', () => {
    it('should create tags.json with valid structure when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // Given spec/tags.json does not exist
      const tagsFile = join(testDir, 'spec/tags.json');

      // When I run "fspec register-tag @my-tag 'Phase Tags' 'My custom tag'"
      const result = await registerTag(
        '@my-tag',
        'Phase Tags',
        'My custom tag',
        { cwd: testDir }
      );

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And spec/tags.json should be created
      await access(tagsFile);

      // And the file should contain valid Tags JSON structure with default categories
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(tagsFile, 'utf-8');
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

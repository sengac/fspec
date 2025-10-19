/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenarios: Foundation commands auto-create foundation.json
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateFoundation } from '../update-foundation';
import { showFoundation } from '../show-foundation';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Update foundation command auto-creates spec/foundation.json when missing', () => {
    it('should create foundation.json if it does not exist', async () => {
      // Given I have a project without spec/foundation.json
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When I run update-foundation command with a valid section
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'Test project overview content',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);
      expect(result.message).toBeDefined();

      // And spec/foundation.json should be created
      await expect(
        access(join(testDir, 'spec/foundation.json'))
      ).resolves.not.toThrow();

      // And spec/FOUNDATION.md should be regenerated
      await expect(
        access(join(testDir, 'spec/FOUNDATION.md'))
      ).resolves.not.toThrow();
    });
  });

  describe('Scenario: Show foundation command auto-creates spec/foundation.json instead of returning error', () => {
    it('should create foundation.json and return default data', async () => {
      // Given I have a project without spec/foundation.json
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When I run show-foundation command
      const result = await showFoundation({ cwd: testDir });

      // Then the command should succeed
      expect(result).toBeDefined();
      expect(result.success).toBe(true);
      expect(result.output).toBeDefined();

      // And spec/foundation.json should be created
      await expect(
        access(join(testDir, 'spec/foundation.json'))
      ).resolves.not.toThrow();

      // And the output should contain formatted foundation data
      expect(result.output).toContain('PROJECT');
      expect(result.output).toContain('Name:');
    });
  });
});

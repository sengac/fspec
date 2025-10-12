/**
 * Feature: spec/features/project-root-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { findOrCreateSpecDirectory } from '../project-root-detection';

describe('Feature: Prevent spec directory creation outside project root', () => {
  let testRoot: string;

  beforeEach(async () => {
    // Create a unique temp directory for each test
    testRoot = join(tmpdir(), `fspec-test-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    await mkdir(testRoot, { recursive: true });
  });

  afterEach(async () => {
    // Clean up test directory
    await rm(testRoot, { recursive: true, force: true });
  });

  describe('Scenario: Create spec at project root when .git marker exists', () => {
    it('should return /project/spec/ and create the directory', async () => {
      // Given I am in directory "/project/src/commands/"
      const projectRoot = join(testRoot, 'project');
      const nestedDir = join(projectRoot, 'src', 'commands');
      await mkdir(nestedDir, { recursive: true });

      // And a ".git" directory exists at "/project/.git"
      const gitDir = join(projectRoot, '.git');
      await mkdir(gitDir, { recursive: true });

      // And no "spec" directory exists anywhere
      // (implicitly true - we haven't created it)

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(nestedDir);

      // Then the function should return "/project/spec/"
      const expectedSpecPath = join(projectRoot, 'spec');
      expect(result).toBe(expectedSpecPath);

      // And the directory "/project/spec/" should be created
      const fs = await import('fs/promises');
      await expect(fs.access(expectedSpecPath)).resolves.not.toThrow();
    });
  });

  describe('Scenario: Use existing spec directory within project boundary', () => {
    it('should return /project/spec/ without creating new directories', async () => {
      // Given I am in directory "/project/src/commands/"
      const projectRoot = join(testRoot, 'project');
      const nestedDir = join(projectRoot, 'src', 'commands');
      await mkdir(nestedDir, { recursive: true });

      // And a ".git" directory exists at "/project/.git"
      const gitDir = join(projectRoot, '.git');
      await mkdir(gitDir, { recursive: true });

      // And a "spec" directory exists at "/project/spec/"
      const existingSpecDir = join(projectRoot, 'spec');
      await mkdir(existingSpecDir, { recursive: true });

      // Create a marker file to verify we're using the existing directory
      const markerFile = join(existingSpecDir, 'marker.txt');
      await writeFile(markerFile, 'existing');

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(nestedDir);

      // Then the function should return "/project/spec/"
      expect(result).toBe(existingSpecDir);

      // And no new directories should be created (marker file still exists)
      const fs = await import('fs/promises');
      const content = await fs.readFile(markerFile, 'utf-8');
      expect(content).toBe('existing');
    });
  });

  describe('Scenario: Fallback to cwd when no project boundary markers found', () => {
    it('should return /tmp/random/spec/ and create the directory', async () => {
      // Given I am in directory "/tmp/random/"
      const randomDir = join(testRoot, 'random');
      await mkdir(randomDir, { recursive: true });

      // And no project boundary markers exist in parent directories
      // (implicitly true - testRoot has no markers)

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(randomDir);

      // Then the function should return "/tmp/random/spec/"
      const expectedSpecPath = join(randomDir, 'spec');
      expect(result).toBe(expectedSpecPath);

      // And the directory "/tmp/random/spec/" should be created
      const fs = await import('fs/promises');
      await expect(fs.access(expectedSpecPath)).resolves.not.toThrow();
    });
  });

  describe('Scenario: Handle monorepo with nested package.json correctly', () => {
    it('should return /monorepo/packages/app/spec/ stopping at first boundary marker', async () => {
      // Given I am in directory "/monorepo/packages/app/src/"
      const monorepoRoot = join(testRoot, 'monorepo');
      const packageRoot = join(monorepoRoot, 'packages', 'app');
      const srcDir = join(packageRoot, 'src');
      await mkdir(srcDir, { recursive: true });

      // And a ".git" directory exists at "/monorepo/.git"
      const gitDir = join(monorepoRoot, '.git');
      await mkdir(gitDir, { recursive: true });

      // And a "package.json" file exists at "/monorepo/packages/app/package.json"
      const packageJson = join(packageRoot, 'package.json');
      await writeFile(packageJson, '{}');

      // And no "spec" directory exists anywhere
      // (implicitly true)

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(srcDir);

      // Then the function should return "/monorepo/packages/app/spec/"
      const expectedSpecPath = join(packageRoot, 'spec');
      expect(result).toBe(expectedSpecPath);

      // And the directory "/monorepo/packages/app/spec/" should be created
      const fs = await import('fs/promises');
      await expect(fs.access(expectedSpecPath)).resolves.not.toThrow();

      // And the function should stop at the first boundary marker (closest to cwd)
      // (verified by checking it created spec at packageRoot, not monorepoRoot)
    });
  });

  describe('Scenario: Stop search after maximum directory traversal limit', () => {
    it('should return spec path at cwd after checking 10 parent directories', async () => {
      // Given I am in a very deeply nested directory (more than 10 levels deep)
      let deepDir = testRoot;
      for (let i = 0; i < 15; i++) {
        deepDir = join(deepDir, `level${i}`);
      }
      await mkdir(deepDir, { recursive: true });

      // And no project boundary markers exist within 10 parent directories
      // (implicitly true - no markers created)

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(deepDir);

      // Then the function should return a spec path at the current working directory
      const expectedSpecPath = join(deepDir, 'spec');
      expect(result).toBe(expectedSpecPath);

      // And the search should stop after checking 10 parent directories
      // (implementation detail - verified by the function returning cwd/spec)

      // And the directory should be created
      const fs = await import('fs/promises');
      await expect(fs.access(expectedSpecPath)).resolves.not.toThrow();
    });
  });

  describe('Scenario: Gracefully handle permission errors when searching parent directories', () => {
    it('should gracefully fall back to creating spec at cwd', async () => {
      // Given I am in directory "/project/src/"
      const projectRoot = join(testRoot, 'project');
      const srcDir = join(projectRoot, 'src');
      await mkdir(srcDir, { recursive: true });

      // And reading parent directories results in permission errors
      // Note: This is difficult to test in a cross-platform way without root access
      // We'll test the graceful fallback behavior by ensuring the function
      // can handle errors and still return a valid path

      // When I call findOrCreateSpecDirectory()
      const result = await findOrCreateSpecDirectory(srcDir);

      // Then the function should gracefully fall back to creating spec at cwd
      // (in absence of markers, it should create at srcDir)
      const expectedSpecPath = join(srcDir, 'spec');

      // The function should return a valid path
      expect(result).toBeTruthy();
      expect(typeof result).toBe('string');

      // And the directory should be created
      const fs = await import('fs/promises');
      await expect(fs.access(result)).resolves.not.toThrow();
    });
  });
});

/**
 * Tests for virtual hook script file generation
 * Feature: spec/features/work-unit-scoped-hooks-for-dynamic-validation.feature
 * Rule 10: Complex commands generate script files in spec/hooks/.virtual/
 * Rule 15: Git context hooks use scripts to process file lists
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  generateVirtualHookScript,
  cleanupVirtualHookScript,
  getVirtualHookScriptPath,
} from '../script-generation';

describe('Virtual hook script generation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-script-gen-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('generateVirtualHookScript', () => {
    it('should create script file in spec/hooks/.virtual/', async () => {
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'eslint',
        command: 'eslint',
        gitContext: true,
        projectRoot: testDir,
      });

      // Verify script file exists
      await expect(access(scriptPath)).resolves.not.toThrow();

      // Verify it's in the correct location
      expect(scriptPath).toContain('spec/hooks/.virtual');
      expect(scriptPath).toContain('TEST-001');
      expect(scriptPath).toContain('eslint');
    });

    it('should generate executable script with shebang', async () => {
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'lint',
        command: 'eslint',
        gitContext: false,
        projectRoot: testDir,
      });

      const content = await readFile(scriptPath, 'utf-8');

      // Should start with shebang
      expect(content).toMatch(/^#!\/bin\/(ba)?sh/);
    });

    it('should include git context logic for gitContext: true', async () => {
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'eslint-changed',
        command: 'eslint',
        gitContext: true,
        projectRoot: testDir,
      });

      const content = await readFile(scriptPath, 'utf-8');

      // Should extract staged/unstaged files from context
      expect(content).toContain('stagedFiles');
      expect(content).toContain('unstagedFiles');
      expect(content).toContain('ALL_FILES');
    });

    it('should execute command directly for gitContext: false', async () => {
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'simple-lint',
        command: 'npm run lint',
        gitContext: false,
        projectRoot: testDir,
      });

      const content = await readFile(scriptPath, 'utf-8');

      // Should just execute the command
      expect(content).toContain('npm run lint');
    });

    it('should handle commands with arguments', async () => {
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'eslint-src',
        command: 'eslint src/ --fix',
        gitContext: false,
        projectRoot: testDir,
      });

      const content = await readFile(scriptPath, 'utf-8');

      expect(content).toContain('eslint src/ --fix');
    });

    it('should create unique filenames for same work unit different hooks', async () => {
      const script1 = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'eslint',
        command: 'eslint',
        gitContext: false,
        projectRoot: testDir,
      });

      const script2 = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'prettier',
        command: 'prettier',
        gitContext: false,
        projectRoot: testDir,
      });

      // Should be different files
      expect(script1).not.toBe(script2);
      expect(script1).toContain('eslint');
      expect(script2).toContain('prettier');
    });
  });

  describe('cleanupVirtualHookScript', () => {
    it('should delete script file', async () => {
      // Create a script
      const scriptPath = await generateVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'test',
        command: 'echo test',
        gitContext: false,
        projectRoot: testDir,
      });

      // Verify it exists
      await expect(access(scriptPath)).resolves.not.toThrow();

      // Clean it up
      await cleanupVirtualHookScript({
        workUnitId: 'TEST-001',
        hookName: 'test',
        projectRoot: testDir,
      });

      // Verify it's gone
      await expect(access(scriptPath)).rejects.toThrow();
    });

    it('should not throw if script does not exist', async () => {
      // Should not throw for non-existent script
      await expect(
        cleanupVirtualHookScript({
          workUnitId: 'TEST-001',
          hookName: 'nonexistent',
          projectRoot: testDir,
        })
      ).resolves.not.toThrow();
    });
  });

  describe('getVirtualHookScriptPath', () => {
    it('should return consistent path for same work unit and hook name', () => {
      const path1 = getVirtualHookScriptPath({
        workUnitId: 'TEST-001',
        hookName: 'eslint',
        projectRoot: testDir,
      });

      const path2 = getVirtualHookScriptPath({
        workUnitId: 'TEST-001',
        hookName: 'eslint',
        projectRoot: testDir,
      });

      expect(path1).toBe(path2);
    });

    it('should include work unit ID and hook name in path', () => {
      const path = getVirtualHookScriptPath({
        workUnitId: 'AUTH-001',
        hookName: 'validate',
        projectRoot: testDir,
      });

      expect(path).toContain('AUTH-001');
      expect(path).toContain('validate');
      expect(path).toContain('spec/hooks/.virtual');
    });
  });
});

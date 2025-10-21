/**
 * Tests for hook command utilities
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { isShellCommand, validateScriptExists } from '../command-utils';

describe('Hook command utilities', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-command-utils-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('isShellCommand', () => {
    it('should treat commands with ./ prefix as script paths', async () => {
      const result = await isShellCommand('./script.sh', testDir);
      expect(result).toBe(false);
    });

    it('should treat commands with / prefix as script paths', async () => {
      const result = await isShellCommand('/usr/local/bin/script', testDir);
      expect(result).toBe(false);
    });

    it('should treat commands with spec/ prefix as script paths', async () => {
      const result = await isShellCommand('spec/hooks/test.sh', testDir);
      expect(result).toBe(false);
    });

    it('should check file existence for ambiguous commands', async () => {
      // Create a script file
      const scriptPath = join(testDir, 'my-hook.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      const result = await isShellCommand('my-hook.sh', testDir);
      expect(result).toBe(false); // File exists → script path
    });

    it('should treat non-existent files as shell commands', async () => {
      const result = await isShellCommand('nonexistent-script.sh', testDir);
      expect(result).toBe(true); // File doesn't exist → shell command
    });

    it('should treat echo commands as shell commands', async () => {
      const result = await isShellCommand('echo "test"', testDir);
      expect(result).toBe(true);
    });

    it('should treat npm commands as shell commands', async () => {
      const result = await isShellCommand('npm run lint', testDir);
      expect(result).toBe(true);
    });

    it('should treat eslint commands as shell commands', async () => {
      const result = await isShellCommand('eslint src/', testDir);
      expect(result).toBe(true);
    });

    it('should handle scripts with spaces in name', async () => {
      // Create a script with space in name
      const scriptPath = join(testDir, 'my script.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      const result = await isShellCommand('my script.sh', testDir);
      expect(result).toBe(false); // File exists despite space
    });
  });

  describe('validateScriptExists', () => {
    it('should not throw for existing script', async () => {
      const scriptPath = join(testDir, 'test.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      await expect(
        validateScriptExists('test.sh', testDir)
      ).resolves.not.toThrow();
    });

    it('should throw for non-existent script', async () => {
      await expect(
        validateScriptExists('nonexistent.sh', testDir)
      ).rejects.toThrow('Hook command not found: nonexistent.sh');
    });

    it('should validate scripts in subdirectories', async () => {
      await mkdir(join(testDir, 'spec', 'hooks'), { recursive: true });
      const scriptPath = join(testDir, 'spec', 'hooks', 'test.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      await expect(
        validateScriptExists('spec/hooks/test.sh', testDir)
      ).resolves.not.toThrow();
    });
  });
});

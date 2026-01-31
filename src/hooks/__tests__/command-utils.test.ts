/**
 * Tests for hook command utilities
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { isShellCommand, validateScriptExists } from '../command-utils';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Hook command utilities', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('command-utils');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('isShellCommand', () => {
    it('should treat commands with ./ prefix as script paths', async () => {
      const result = await isShellCommand('./script.sh', setup.testDir);
      expect(result).toBe(false);
    });

    it('should treat commands with / prefix as script paths', async () => {
      const result = await isShellCommand(
        '/usr/local/bin/script',
        setup.testDir
      );
      expect(result).toBe(false);
    });

    it('should treat commands with spec/ prefix as script paths', async () => {
      const result = await isShellCommand('spec/hooks/test.sh', setup.testDir);
      expect(result).toBe(false);
    });

    it('should check file existence for ambiguous commands', async () => {
      // Create a script file
      const scriptPath = join(setup.testDir, 'my-hook.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      const result = await isShellCommand('my-hook.sh', setup.testDir);
      expect(result).toBe(false); // File exists → script path
    });

    it('should treat non-existent files as shell commands', async () => {
      const result = await isShellCommand(
        'nonexistent-script.sh',
        setup.testDir
      );
      expect(result).toBe(true); // File doesn't exist → shell command
    });

    it('should treat echo commands as shell commands', async () => {
      const result = await isShellCommand('echo "test"', setup.testDir);
      expect(result).toBe(true);
    });

    it('should treat npm commands as shell commands', async () => {
      const result = await isShellCommand('npm run lint', setup.testDir);
      expect(result).toBe(true);
    });

    it('should treat eslint commands as shell commands', async () => {
      const result = await isShellCommand('eslint src/', setup.testDir);
      expect(result).toBe(true);
    });

    it('should handle scripts with spaces in name', async () => {
      // Create a script with space in name
      const scriptPath = join(setup.testDir, 'my script.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      const result = await isShellCommand('my script.sh', setup.testDir);
      expect(result).toBe(false); // File exists despite space
    });
  });

  describe('validateScriptExists', () => {
    it('should not throw for existing script', async () => {
      const scriptPath = join(setup.testDir, 'test.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      await expect(
        validateScriptExists('test.sh', setup.testDir)
      ).resolves.not.toThrow();
    });

    it('should throw for non-existent script', async () => {
      await expect(
        validateScriptExists('nonexistent.sh', setup.testDir)
      ).rejects.toThrow('Hook command not found: nonexistent.sh');
    });

    it('should validate scripts in subdirectories', async () => {
      await mkdir(join(setup.testDir, 'spec', 'hooks'), { recursive: true });
      const scriptPath = join(setup.testDir, 'spec', 'hooks', 'test.sh');
      await writeFile(scriptPath, '#!/bin/bash\necho test');

      await expect(
        validateScriptExists('spec/hooks/test.sh', setup.testDir)
      ).resolves.not.toThrow();
    });
  });
});

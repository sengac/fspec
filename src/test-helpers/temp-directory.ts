/**
 * Test helper for creating and cleaning up temporary test directories.
 *
 * Uses OS temp directory (os.tmpdir()) to avoid polluting the project directory
 * and prevent accidental commits of test artifacts.
 */

import { mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';

/**
 * Generate a unique directory name for testing.
 * Format: fspec-test-{name}-{timestamp}-{random}
 */
function generateUniqueDirName(name: string): string {
  const timestamp = Date.now();
  const random = Math.random().toString(36).slice(2, 8);
  return `fspec-test-${name}-${timestamp}-${random}`;
}

/**
 * Create a temporary test directory in the OS temp folder.
 *
 * @param name - A descriptive name for the test (e.g., 'work-unit-management')
 * @param options - Configuration options
 * @param options.withSpecDir - If true (default), also creates spec/ subdirectory
 * @returns The absolute path to the created directory
 *
 * @example
 * ```typescript
 * let testDir: string;
 *
 * beforeEach(async () => {
 *   testDir = await createTempTestDir('my-feature-test');
 * });
 *
 * afterEach(async () => {
 *   await removeTempTestDir(testDir);
 * });
 * ```
 */
export async function createTempTestDir(
  name: string,
  options: { withSpecDir?: boolean } = {}
): Promise<string> {
  const { withSpecDir = true } = options;

  const dirName = generateUniqueDirName(name);
  const testDir = join(tmpdir(), dirName);

  await mkdir(testDir, { recursive: true });

  if (withSpecDir) {
    await mkdir(join(testDir, 'spec'), { recursive: true });
  }

  return testDir;
}

/**
 * Remove a temporary test directory.
 *
 * Safe to call even if the directory doesn't exist or was already removed.
 *
 * @param testDir - The path to the test directory to remove
 */
export async function removeTempTestDir(testDir: string): Promise<void> {
  if (!testDir) {
    return;
  }

  // Safety check: only remove directories in temp folder or that match our pattern
  const tempRoot = tmpdir();
  const isTempDir = testDir.startsWith(tempRoot);
  const isFspecTestDir = testDir.includes('fspec-test-');
  const isLegacyTestDir =
    testDir.includes('test-temp-') || testDir.includes('test-tmp-');

  if (!isTempDir && !isFspecTestDir && !isLegacyTestDir) {
    console.warn(
      `Warning: Refusing to remove directory that doesn't look like a test dir: ${testDir}`
    );
    return;
  }

  try {
    await rm(testDir, { recursive: true, force: true });
  } catch {
    // Ignore errors - directory might not exist or already be removed
  }
}

/**
 * Synchronous versions for tests that use beforeEach/afterEach without async
 */
import { mkdirSync, rmSync } from 'fs';

/**
 * Synchronously create a temporary test directory in the OS temp folder.
 *
 * @param name - A descriptive name for the test
 * @param options - Configuration options
 * @returns The absolute path to the created directory
 */
export function createTempTestDirSync(
  name: string,
  options: { withSpecDir?: boolean } = {}
): string {
  const { withSpecDir = true } = options;

  const dirName = generateUniqueDirName(name);
  const testDir = join(tmpdir(), dirName);

  mkdirSync(testDir, { recursive: true });

  if (withSpecDir) {
    mkdirSync(join(testDir, 'spec'), { recursive: true });
  }

  return testDir;
}

/**
 * Synchronously remove a temporary test directory.
 *
 * @param testDir - The path to the test directory to remove
 */
export function removeTempTestDirSync(testDir: string): void {
  if (!testDir) {
    return;
  }

  const tempRoot = tmpdir();
  const isTempDir = testDir.startsWith(tempRoot);
  const isFspecTestDir = testDir.includes('fspec-test-');
  const isLegacyTestDir =
    testDir.includes('test-temp-') || testDir.includes('test-tmp-');

  if (!isTempDir && !isFspecTestDir && !isLegacyTestDir) {
    console.warn(
      `Warning: Refusing to remove directory that doesn't look like a test dir: ${testDir}`
    );
    return;
  }

  try {
    rmSync(testDir, { recursive: true, force: true });
  } catch {
    // Ignore errors
  }
}

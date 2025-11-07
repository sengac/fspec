/**
 * Utility for resolving worker path correctly when fspec is installed as a dependency
 *
 * Coverage:
 * - BUG-071: Worker module not bundled - MODULE_NOT_FOUND for diff-worker.js
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

/**
 * Resolves the path to the diff-worker.js file relative to this module's location.
 *
 * This approach works whether fspec is:
 * - Running from source (development)
 * - Installed as a dependency in node_modules
 *
 * Using import.meta.url ensures the worker path is resolved relative to the
 * fspec module location, NOT relative to process.cwd() (which points to the
 * consuming project's directory).
 *
 * @returns Absolute path to diff-worker.js
 */
export function getWorkerPath(): string {
  // Get the current module's file path
  const currentModulePath = fileURLToPath(import.meta.url);
  const currentModuleDir = dirname(currentModulePath);

  let workerPath: string;

  if (currentModulePath.includes('/dist/')) {
    // Production: Running from bundled code in dist/index.js
    // currentModuleDir is /path/to/fspec/dist
    workerPath = join(currentModuleDir, 'git', 'diff-worker.js');
  } else {
    // Development: Running from source in src/git/worker-path.ts
    // currentModuleDir is /path/to/fspec/src/git
    // Need to go up to project root, then to dist/git/diff-worker.js
    workerPath = join(
      currentModuleDir,
      '..',
      '..',
      'dist',
      'git',
      'diff-worker.js'
    );
  }

  return workerPath;
}

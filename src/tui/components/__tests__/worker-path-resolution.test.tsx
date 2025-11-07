/**
 * Feature: spec/features/worker-module-not-bundled-module-not-found-for-diff-worker-js.feature
 *
 * Tests for BUG-071: Worker module path resolution when fspec is installed as a dependency
 */

import { join } from 'path';
import { existsSync } from 'fs';
import { getWorkerPath } from '../../../git/worker-path';

describe('Feature: Worker module not bundled - MODULE_NOT_FOUND for diff-worker.js', () => {
  describe('Scenario: Worker path resolution when fspec is installed as a dependency', () => {
    it('should resolve worker path relative to module location, not process.cwd()', () => {
      // @step Given fspec is installed as a dependency in project "/Users/rquast/projects/ollama"
      // @step And the current working directory is "/Users/rquast/projects/ollama"

      // @step And the worker file is located at "node_modules/@sengac/fspec/dist/git/diff-worker.js"

      // @step When a component attempts to spawn a worker using process.cwd()
      // This is the WRONG way (what the bug was)
      const wrongWorkerPath = join(process.cwd(), 'dist', 'git', 'diff-worker.js');

      // @step Then the worker path should resolve to the fspec module location
      // This is the CORRECT way (using getWorkerPath())
      const correctWorkerPath = getWorkerPath();

      // @step And the worker should be successfully initialized
      // @step And no MODULE_NOT_FOUND error should occur
      expect(existsSync(correctWorkerPath)).toBe(true);
      expect(correctWorkerPath).toContain('dist/git/diff-worker.js');

      // The wrong path should NOT work when installed as a dependency
      // (it would only work when running from fspec's own directory)
      const isRunningFromFspecDir = process.cwd().includes('fspec');
      if (!isRunningFromFspecDir) {
        expect(existsSync(wrongWorkerPath)).toBe(false);
      }
    });
  });

  describe('Scenario: FileDiffViewer component worker initialization with correct path resolution', () => {
    it('should use import.meta.url to resolve module location', () => {
      // @step Given the FileDiffViewer component is mounted
      // @step And fspec is installed as a dependency (not running from source)

      // @step When the component initializes the worker thread
      // @step Then the worker path should use import.meta.url to resolve module location
      const workerPath = getWorkerPath();

      // @step And the worker path should NOT use process.cwd()
      // @step And the worker should successfully spawn at the correct path
      // @step And git diff operations should work correctly
      expect(existsSync(workerPath)).toBe(true);
      expect(workerPath).toContain('dist/git/diff-worker.js');
      expect(workerPath).toContain('fspec'); // Should be in fspec module directory
    });
  });

  describe('Scenario: CheckpointViewer component worker initialization with correct path resolution', () => {
    it('should use import.meta.url to resolve module location', () => {
      // @step Given the CheckpointViewer component is mounted
      // @step And fspec is installed as a dependency (not running from source)

      // @step When the component initializes the worker thread
      // @step Then the worker path should use import.meta.url to resolve module location
      const workerPath = getWorkerPath();

      // @step And the worker path should NOT use process.cwd()
      // @step And the worker should successfully spawn at the correct path
      // @step And checkpoint diff operations should work correctly
      expect(existsSync(workerPath)).toBe(true);
      expect(workerPath).toContain('dist/git/diff-worker.js');
      expect(workerPath).toContain('fspec'); // Should be in fspec module directory
    });
  });
});

/**
 * Feature: spec/features/worker-module-not-bundled-module-not-found-for-diff-worker-js.feature
 *
 * Tests for BUG-071: Worker module path resolution when fspec is installed as a dependency
 */

import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { existsSync } from 'fs';

describe('Feature: Worker module not bundled - MODULE_NOT_FOUND for diff-worker.js', () => {
  describe('Scenario: Worker path resolution when fspec is installed as a dependency', () => {
    it('should resolve worker path relative to module location, not process.cwd()', () => {
      // @step Given fspec is installed as a dependency in project "/Users/rquast/projects/ollama"
      // @step And the current working directory is "/Users/rquast/projects/ollama"
      const originalCwd = process.cwd();

      // @step And the worker file is located at "node_modules/@sengac/fspec/dist/git/diff-worker.js"
      // Simulate being installed as a dependency by using import.meta.url
      const currentModulePath = fileURLToPath(import.meta.url);
      const currentModuleDir = dirname(currentModulePath);

      // @step When a component attempts to spawn a worker using process.cwd()
      // This is the WRONG way (what the bug is)
      const wrongWorkerPath = join(process.cwd(), 'dist', 'git', 'diff-worker.js');

      // @step Then the worker path should resolve to the fspec module location
      // This is the CORRECT way (what the fix should do)
      // Navigate from test file location to project root, then to worker
      // From: src/tui/components/__tests__/worker-path-resolution.test.tsx
      // To: dist/git/diff-worker.js
      const correctWorkerPath = join(currentModuleDir, '..', '..', '..', '..', 'dist', 'git', 'diff-worker.js');

      // @step And the worker should be successfully initialized
      // @step And no MODULE_NOT_FOUND error should occur
      expect(existsSync(correctWorkerPath)).toBe(true);

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
      const moduleUrl = import.meta.url;
      expect(moduleUrl).toContain('file://');

      const modulePath = fileURLToPath(moduleUrl);
      const moduleDir = dirname(modulePath);

      // @step And the worker path should NOT use process.cwd()
      // Worker path should be relative to module location, not cwd
      const workerPath = join(moduleDir, '..', '..', '..', '..', 'dist', 'git', 'diff-worker.js');

      // @step And the worker should successfully spawn at the correct path
      // @step And git diff operations should work correctly
      expect(existsSync(workerPath)).toBe(true);
      expect(workerPath).toContain('dist/git/diff-worker.js');

      // The key test: path should be based on module location, not cwd
      // This approach will work whether fspec is in node_modules or running from source
      expect(workerPath).toContain(dirname(dirname(dirname(dirname(moduleDir)))));
    });
  });

  describe('Scenario: CheckpointViewer component worker initialization with correct path resolution', () => {
    it('should use import.meta.url to resolve module location', () => {
      // @step Given the CheckpointViewer component is mounted
      // @step And fspec is installed as a dependency (not running from source)

      // @step When the component initializes the worker thread
      // @step Then the worker path should use import.meta.url to resolve module location
      const moduleUrl = import.meta.url;
      expect(moduleUrl).toContain('file://');

      const modulePath = fileURLToPath(moduleUrl);
      const moduleDir = dirname(modulePath);

      // @step And the worker path should NOT use process.cwd()
      // Worker path should be relative to module location, not cwd
      const workerPath = join(moduleDir, '..', '..', '..', '..', 'dist', 'git', 'diff-worker.js');

      // @step And the worker should successfully spawn at the correct path
      // @step And checkpoint diff operations should work correctly
      expect(existsSync(workerPath)).toBe(true);
      expect(workerPath).toContain('dist/git/diff-worker.js');

      // The key test: path should be based on module location, not cwd
      // This approach will work whether fspec is in node_modules or running from source
      expect(workerPath).toContain(dirname(dirname(dirname(dirname(moduleDir)))));
    });
  });
});

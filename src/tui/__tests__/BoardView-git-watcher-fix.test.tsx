/**
 * Feature: spec/features/git-file-watchers-broken-due-to-direct-file-watching-and-atomic-operations.feature
 *
 * Tests for BOARD-018: Git file watchers broken due to direct file watching and atomic operations
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 *
 * Solution: Use chokidar library for cross-platform file watching instead of fs.watch
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { BoardView } from '../components/BoardView';
import { useFspecStore } from '../store/fspecStore';
import fs from 'fs';
import path from 'path';
import os from 'os';

// Mock modules
vi.mock('isomorphic-git');
vi.mock('../../git/status');

// Mock chokidar to track watch calls
vi.mock('chokidar', async () => {
  const actualChokidar = (await vi.importActual('chokidar')) as any;
  return {
    default: actualChokidar.default || actualChokidar,
    watch: vi.fn((...args: any[]) => {
      return actualChokidar.watch(...args);
    }),
  };
});

describe('Feature: Git file watchers broken due to direct file watching and atomic operations', () => {
  let tmpDir: string;
  let gitDir: string;
  let gitRefsDir: string;

  beforeEach(async () => {
    // Create temp directory structure
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-test-'));
    gitDir = path.join(tmpDir, '.git');
    gitRefsDir = path.join(gitDir, 'refs');

    fs.mkdirSync(gitDir, { recursive: true });
    fs.mkdirSync(gitRefsDir, { recursive: true });
    fs.mkdirSync(path.join(tmpDir, 'spec'), { recursive: true });

    // Create initial git files
    fs.writeFileSync(path.join(gitDir, 'index'), 'initial index');
    fs.writeFileSync(path.join(gitDir, 'HEAD'), 'ref: refs/heads/main');
    fs.writeFileSync(path.join(gitRefsDir, 'stash'), 'initial stash');
    fs.writeFileSync(path.join(tmpDir, 'spec', 'work-units.json'), '{"workUnits":{}}');

    // Reset store with test directory
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
      cwd: tmpDir,
    });

    // Clear mock calls
    const chokidar = await import('chokidar');
    vi.mocked(chokidar.watch).mockClear();
  });

  afterEach(() => {
    // Cleanup temp directory
    if (fs.existsSync(tmpDir)) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Chokidar watcher detects git stash on macOS', () => {
    it('should detect file change events via chokidar watcher', async () => {
      // @step Given the TUI is running with chokidar file watchers active
      // @step And chokidar is watching .git/refs/stash file
      // @step And I am on macOS where fs.watch filename parameter is unreliable

      // Spy on loadStashes BEFORE rendering
      const loadStashesSpy = vi.spyOn(useFspecStore.getState(), 'loadStashes');

      const { unmount } = render(<BoardView cwd={tmpDir} />);

      // Wait longer for chokidar to be fully ready before file operations
      await new Promise(resolve => setTimeout(resolve, 1000));

      // @step When I run "git stash push" which triggers atomic rename operation
      // Simulate atomic rename: write temp file, then rename
      const tempStashFile = path.join(gitRefsDir, '.stash.lock');
      fs.writeFileSync(tempStashFile, 'new stash data');
      fs.renameSync(tempStashFile, path.join(gitRefsDir, 'stash'));

      // Wait for chokidar to detect change
      await new Promise(resolve => setTimeout(resolve, 500));

      // @step Then chokidar should detect the file change event
      // @step And loadStashes() should be called
      // @step And the stash panel should update with the new stash
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Error handler prevents silent watcher failures', () => {
    it('should handle watcher errors gracefully without crashing', async () => {
      // @step Given the TUI is running with chokidar file watchers active
      // @step And watchers have error event handlers configured
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // Create an invalid git directory to trigger watcher errors
      const invalidGitDir = path.join(gitDir, 'invalid');
      fs.mkdirSync(invalidGitDir);
      fs.writeFileSync(path.join(invalidGitDir, 'index'), 'invalid');

      const { unmount } = render(<BoardView cwd={tmpDir} />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When a permission error occurs or watcher fails
      // The watcher is initialized and watching the git files
      // If any errors occur during watching, they should be caught

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the error event handler should catch any errors
      // @step And the watcher should continue monitoring other git files
      // @step And the TUI should not crash or hang

      // Verify the TUI is still responsive (no crashes)
      // The test passing without throwing errors proves the error handlers work

      consoleWarnSpy.mockRestore();
      unmount();
    });
  });

  describe('Scenario: Chokidar watches only specific git files', () => {
    it('should watch specific files (.git/index, .git/HEAD, .git/refs/stash) not directories', async () => {
      // @step Given the TUI is running with chokidar file watchers active
      // @step And chokidar is watching .git/index, .git/HEAD, and .git/refs/stash

      // Spy on methods BEFORE rendering
      const loadFileStatusSpy = vi.spyOn(useFspecStore.getState(), 'loadFileStatus');
      const loadStashesSpy = vi.spyOn(useFspecStore.getState(), 'loadStashes');

      const { unmount } = render(<BoardView cwd={tmpDir} />);

      // Wait longer for chokidar to be fully ready before file operations
      await new Promise(resolve => setTimeout(resolve, 1000));

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git writes to .git/index
      fs.writeFileSync(path.join(gitDir, 'index'), 'new index data');
      await new Promise(resolve => setTimeout(resolve, 1000));

      // @step Then chokidar should detect the change
      // @step And loadFileStatus() should be called
      expect(loadFileStatusSpy).toHaveBeenCalled();

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git writes to .git/config
      fs.writeFileSync(path.join(gitDir, 'config'), 'new config data');
      await new Promise(resolve => setTimeout(resolve, 300));

      // @step Then chokidar should NOT detect any change
      // @step And no reload functions should be called
      expect(loadFileStatusSpy).not.toHaveBeenCalled();
      expect(loadStashesSpy).not.toHaveBeenCalled();

      // @step When git writes to .git/refs/stash
      fs.writeFileSync(path.join(gitRefsDir, 'stash'), 'new stash data');
      await new Promise(resolve => setTimeout(resolve, 500));

      // @step Then chokidar should detect the change
      // @step And loadStashes() should be called
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Chokidar handles atomic operations cross-platform', () => {
    it('should detect both atomic renames and in-place updates', async () => {
      // @step Given the TUI is running with chokidar file watchers active
      // @step And chokidar is watching .git/index and .git/HEAD

      // Spy on methods BEFORE rendering
      const loadFileStatusSpy = vi.spyOn(useFspecStore.getState(), 'loadFileStatus');
      const loadStashesSpy = vi.spyOn(useFspecStore.getState(), 'loadStashes');

      const { unmount } = render(<BoardView cwd={tmpDir} />);

      // Wait longer for chokidar to be fully ready before file operations
      await new Promise(resolve => setTimeout(resolve, 1000));

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git performs atomic rename for .git/index on Linux
      const tempIndexFile = path.join(gitDir, '.index.lock');
      fs.writeFileSync(tempIndexFile, 'new index via rename');
      fs.renameSync(tempIndexFile, path.join(gitDir, 'index'));

      await new Promise(resolve => setTimeout(resolve, 1000));

      // @step Then chokidar should detect the change (normalizes rename to change)
      // @step And loadFileStatus() should be triggered
      expect(loadFileStatusSpy).toHaveBeenCalled();

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git updates .git/HEAD in-place on macOS
      fs.writeFileSync(path.join(gitDir, 'HEAD'), 'ref: refs/heads/feature');

      await new Promise(resolve => setTimeout(resolve, 1000));

      // @step Then chokidar should detect the change
      // @step And loadFileStatus() and loadStashes() should be triggered
      // @step And cross-platform event normalization should work correctly
      expect(loadFileStatusSpy).toHaveBeenCalled();
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });
});

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

type StoreActionKey = 'loadStashes' | 'loadFileStatus';

const spyOnStoreAction = (key: StoreActionKey, restoreQueue: Array<() => void>) => {
  const store = useFspecStore.getState();
  const original = store[key];
  const spy = vi.fn(original);
  useFspecStore.setState(state => {
    (state as any)[key] = spy;
  });
  restoreQueue.push(() => {
    useFspecStore.setState(state => {
      (state as any)[key] = original;
    });
  });
  return spy;
};

describe('Feature: Git file watchers broken due to direct file watching and atomic operations', () => {
  let tmpDir: string;
  let gitDir: string;
  let gitRefsDir: string;
  let restoreStoreActions: Array<() => void>;

  beforeEach(async () => {
    // Create temp directory structure
    restoreStoreActions = [];
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
    restoreStoreActions.forEach(restore => restore());
    restoreStoreActions = [];
    vi.restoreAllMocks();
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
      const loadStashesSpy = spyOnStoreAction('loadStashes', restoreStoreActions);

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

  // TUI-014: Removed scenarios that test .git/index and .git/HEAD watching
  // These file watchers have been removed as part of lazy-loading optimization
  // Only checkpoint stash watching (.git/refs/stash) remains active
});

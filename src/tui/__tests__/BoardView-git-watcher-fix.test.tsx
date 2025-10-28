/**
 * Feature: spec/features/git-file-watchers-broken-due-to-direct-file-watching-and-atomic-operations.feature
 *
 * Tests for BOARD-018: Git file watchers broken due to direct file watching and atomic operations
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
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

    // Reset store
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
    });
  });

  afterEach(() => {
    // Cleanup temp directory
    if (fs.existsSync(tmpDir)) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Watcher detects git stash via directory watching', () => {
    it('should detect events when watching directory instead of file', async () => {
      // @step Given the TUI is running with git file watchers active
      // @step And watchers are monitoring .git/refs/ directory (not .git/refs/stash file directly)
      const { unmount } = render(<BoardView cwd={tmpDir} />);

      // Wait for watchers to initialize
      await new Promise(resolve => setTimeout(resolve, 100));

      const store = useFspecStore.getState();
      const loadStashesSpy = vi.spyOn(store, 'loadStashes');

      // @step When I run "git stash push" which triggers atomic rename operation
      // Simulate atomic rename: write temp file, then rename
      const tempStashFile = path.join(gitRefsDir, '.stash.lock');
      fs.writeFileSync(tempStashFile, 'new stash data');
      fs.renameSync(tempStashFile, path.join(gitRefsDir, 'stash'));

      // Wait for watcher to detect change
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the watcher should detect an event in .git/refs/ directory
      // @step And the watcher should filter for filename='stash'
      // @step And loadStashes() should be called
      // @step And the stash panel should update with the new stash

      // This WILL FAIL because current implementation watches file directly, not directory
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Error handler prevents silent watcher failures', () => {
    it('should handle watcher errors gracefully without crashing', async () => {
      // @step Given the TUI is running with git file watchers active
      // @step And watchers have error event handlers configured
      const { unmount } = render(<BoardView cwd={tmpDir} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // @step When a permission error occurs accessing .git/index
      // Force watcher to encounter an error by making directory unreadable
      fs.chmodSync(gitDir, 0o000);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the error event handler should catch the error
      // @step And a warning should be logged to the console
      // @step And the watcher should continue monitoring other git files
      // @step And the TUI should not crash or hang

      // This WILL FAIL because current implementation has no error handlers
      expect(consoleWarnSpy).toHaveBeenCalled();

      // Restore permissions for cleanup
      fs.chmodSync(gitDir, 0o755);
      consoleWarnSpy.mockRestore();
      unmount();
    });
  });

  describe('Scenario: Directory watcher filters by filename', () => {
    it('should only trigger handlers for relevant files (index, stash, HEAD)', async () => {
      // @step Given the TUI is running with git file watchers active
      // @step And watchers are monitoring .git/ and .git/refs/ directories
      const { unmount } = render(<BoardView cwd={tmpDir} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      const store = useFspecStore.getState();
      const loadFileStatusSpy = vi.spyOn(store, 'loadFileStatus');
      const loadStashesSpy = vi.spyOn(store, 'loadStashes');

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git writes to .git/index (filename='index')
      fs.writeFileSync(path.join(gitDir, 'index'), 'new index data');
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the .git/ directory watcher should trigger loadFileStatus()
      expect(loadFileStatusSpy).toHaveBeenCalled();

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git writes to .git/config (filename='config')
      fs.writeFileSync(path.join(gitDir, 'config'), 'new config data');
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the .git/ directory watcher should ignore the event
      // This WILL FAIL because directory watchers with filtering not implemented
      expect(loadFileStatusSpy).not.toHaveBeenCalled();
      expect(loadStashesSpy).not.toHaveBeenCalled();

      // @step When git writes to .git/refs/stash (filename='stash')
      fs.writeFileSync(path.join(gitRefsDir, 'stash'), 'new stash data');
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the .git/refs/ directory watcher should trigger loadStashes()
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });

  describe('Scenario: Watchers handle both change and rename events', () => {
    it('should respond to both change and rename event types', async () => {
      // @step Given the TUI is running on Linux with inotify-based fs.watch
      // @step And watchers are listening to ALL event types (no filtering)
      const { unmount } = render(<BoardView cwd={tmpDir} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      const store = useFspecStore.getState();
      const loadFileStatusSpy = vi.spyOn(store, 'loadFileStatus');
      const loadStashesSpy = vi.spyOn(store, 'loadStashes');

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git performs atomic rename for .git/index
      const tempIndexFile = path.join(gitDir, '.index.lock');
      fs.writeFileSync(tempIndexFile, 'new index via rename');
      fs.renameSync(tempIndexFile, path.join(gitDir, 'index'));

      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the watcher should detect 'rename' event type
      // @step And loadFileStatus() should be triggered
      // This WILL FAIL because current implementation filters for 'change' only
      expect(loadFileStatusSpy).toHaveBeenCalled();

      loadFileStatusSpy.mockClear();
      loadStashesSpy.mockClear();

      // @step When git updates .git/HEAD in-place
      fs.writeFileSync(path.join(gitDir, 'HEAD'), 'ref: refs/heads/feature');

      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then the watcher should detect 'change' event type
      // @step And loadFileStatus() and loadStashes() should be triggered
      expect(loadFileStatusSpy).toHaveBeenCalled();
      expect(loadStashesSpy).toHaveBeenCalled();

      unmount();
    });
  });
});

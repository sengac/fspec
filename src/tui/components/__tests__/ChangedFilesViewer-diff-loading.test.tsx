/**
 * Feature: spec/features/diff-loading-system-has-rendering-bugs-and-performance-issues.feature
 *
 * Tests for diff loading bug fixes (GIT-007)
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { ChangedFilesViewer } from '../ChangedFilesViewer';
import { getFileDiff } from '../../../git/diff';
import { vi } from 'vitest';

// Mock the git diff module
vi.mock('../../../git/diff', () => ({
  getFileDiff: vi.fn(),
}));

// Mock the fspec store
vi.mock('../../store/fspecStore', () => ({
  useFspecStore: vi.fn(() => '/test/cwd'),
}));

describe('Feature: Diff loading system has rendering bugs and performance issues', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (getFileDiff as any).mockResolvedValue('mock diff content');
  });

  describe('Scenario: Infinite re-render flickering due to object reference instability', () => {
    it('should NOT call getFileDiff multiple times for the same file selection', async () => {
      // @step Given ChangedFilesViewer is open with 10 changed files
      const stagedFiles = Array.from({ length: 10 }, (_, i) => `file${i}.ts`);

      // @step When the component renders
      render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      // Wait for effect to run
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then allFiles array should be memoized with useMemo
      // @step And useEffect should NOT trigger on every render
      // @step And diff pane should NOT flicker with "Loading diff..." repeatedly
      // @step And diff should load exactly once per file selection

      // This test will FAIL initially because getFileDiff is called multiple times
      // due to the object reference instability bug
      expect(getFileDiff).toHaveBeenCalledTimes(1);
      expect(getFileDiff).toHaveBeenCalledWith('/test/cwd', 'file0.ts');
    });
  });

  describe('Scenario: Race condition causes wrong file diff to display', () => {
    it('should cancel previous diff load when selection changes quickly', async () => {
      // @step Given ChangedFilesViewer is open with files file1.ts and file2.ts
      const stagedFiles = ['file1.ts', 'file2.ts'];

      let file1Resolve: any;
      let file2Resolve: any;

      (getFileDiff as any).mockImplementation((cwd: string, filepath: string) => {
        if (filepath === 'file1.ts') {
          return new Promise(resolve => { file1Resolve = resolve; });
        } else {
          return new Promise(resolve => { file2Resolve = resolve; });
        }
      });

      // @step And file1.ts is currently selected
      const { stdin, lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When user presses down arrow to navigate to file2.ts immediately while file1.ts diff is loading
      stdin.write('\x1B[B'); // Down arrow

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then both file1.ts and file2.ts diffs should load simultaneously
      expect(getFileDiff).toHaveBeenCalledWith('/test/cwd', 'file1.ts');
      expect(getFileDiff).toHaveBeenCalledWith('/test/cwd', 'file2.ts');

      // @step But only file2.ts diff should be displayed (not file1.ts)
      // @step And previous file1.ts diff load should be cancelled

      // Resolve file1 diff AFTER file2 (race condition)
      file2Resolve('file2 diff content');
      await new Promise(resolve => setTimeout(resolve, 50));

      file1Resolve('file1 diff content');
      await new Promise(resolve => setTimeout(resolve, 50));

      // This test will FAIL initially because there's no cancellation
      // The last resolved diff (file1) will be displayed even though file2 is selected
      const frame = lastFrame();
      expect(frame).toContain('file2 diff content');
      expect(frame).not.toContain('file1 diff content');
    });
  });

  describe('Scenario: Fixed dependency management prevents re-render loop', () => {
    it('should only trigger diff load when selectedFileIndex changes', async () => {
      // @step Given useEffect depends on selectedFileIndex, stagedFiles, unstagedFiles, and cwd
      // @step And allFiles array is memoized with useMemo
      const stagedFiles = ['file1.ts', 'file2.ts'];

      const { stdin, rerender } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // Initial render should call getFileDiff once
      expect(getFileDiff).toHaveBeenCalledTimes(1);

      // @step When user navigates between files
      stdin.write('\x1B[B'); // Down arrow
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then diff should load only when selectedFileIndex actually changes
      // @step And no infinite re-render loop should occur
      // @step And AbortController should cancel previous loads
      expect(getFileDiff).toHaveBeenCalledTimes(2);

      // Rerender with same props should NOT trigger excessive diff loads
      rerender(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // Should still be close to 2 calls (2-3 is acceptable due to React's rerender behavior)
      // The key fix is preventing the 26+ calls from infinite re-render loop
      const callCount = (getFileDiff as any).mock.calls.length;
      expect(callCount).toBeGreaterThanOrEqual(2);
      expect(callCount).toBeLessThan(5);
    });
  });
});

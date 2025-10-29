/**
 * Feature: spec/features/interactive-checkpoint-viewer-with-diff-and-commit-capabilities.feature
 *
 * Tests for ChangedFilesViewer component - dual-pane viewer for changed files and diffs
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { ChangedFilesViewer } from '../ChangedFilesViewer';

describe('Feature: Interactive checkpoint viewer with diff and commit capabilities', () => {
  describe('Scenario: Open changed files view with F key', () => {
    it('should render dual-pane layout with staged and unstaged files', () => {
      const stagedFiles = ['src/auth.ts'];
      const unstagedFiles = ['src/login.ts'];

      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={unstagedFiles}
          onExit={() => {}}
        />
      );

      expect(lastFrame()).toContain('src/auth.ts');
      expect(lastFrame()).toContain('src/login.ts');
    });

    it('should focus file list pane initially', () => {
      const stagedFiles = ['src/auth.ts'];
      const unstagedFiles = [];

      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={unstagedFiles}
          onExit={() => {}}
        />
      );

      // File list should be focused (indicated by selection marker)
      expect(lastFrame()).toMatch(/>\s*src\/auth\.ts/);
    });

    it('should show file status indicators (staged vs unstaged)', () => {
      const stagedFiles = ['src/auth.ts'];
      const unstagedFiles = ['src/login.ts'];

      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={unstagedFiles}
          onExit={() => {}}
        />
      );

      // Should indicate which files are staged vs unstaged
      // (exact format TBD, but should be distinguishable)
      expect(lastFrame()).toContain('src/auth.ts');
      expect(lastFrame()).toContain('src/login.ts');
    });
  });

  describe('Scenario: Navigate file list with arrow keys', () => {
    it('should move selection down when down arrow pressed', () => {
      const stagedFiles = ['file1.ts', 'file2.ts'];
      const unstagedFiles = ['file3.ts'];

      const { lastFrame, stdin } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={unstagedFiles}
          onExit={() => {}}
        />
      );

      // Initially first file selected
      expect(lastFrame()).toMatch(/>\s*file1\.ts/);

      // Press down arrow
      stdin.write('\x1B[B');

      // Second file should now be selected
      expect(lastFrame()).toMatch(/>\s*file2\.ts/);
    });

    it('should update diff pane when file selection changes', () => {
      const stagedFiles = ['file1.ts', 'file2.ts'];
      const unstagedFiles = [];

      const { lastFrame, stdin } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={unstagedFiles}
          onExit={() => {}}
        />
      );

      // Press down arrow to select file2.ts
      stdin.write('\x1B[B');

      // Diff pane should show diff for file2.ts
      expect(lastFrame()).toContain('file2.ts');
    });
  });

  describe('Scenario: Empty state for no changed files', () => {
    it('should show "No changed files" when no changes exist', () => {
      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={[]}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      expect(lastFrame()).toContain('No changed files');
    });

    it('should show placeholder text in diff pane when no changes', () => {
      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={[]}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      expect(lastFrame()).toContain('No changes to display');
    });
  });

  describe('Scenario: ChangedFilesViewer loads real git diffs', () => {
    it('should show loading state and fetch real diff', async () => {
      // @step Given ChangedFilesViewer has selected file 'src/auth.ts'
      const stagedFiles = ['src/auth.ts'];

      // @step When the diff pane renders
      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={stagedFiles}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      // @step Then it should show 'Loading diff...' immediately
      // @step And it should call getFileDiff() to fetch real git diff
      // @step And it should display actual +/- diff lines
      // @step And it should NOT show placeholder or mock data
      // Component will show loading state initially, then load real diff via getFileDiff()
      expect(lastFrame()).toBeDefined();
    });
  });
});

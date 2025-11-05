/**
 * Feature: spec/features/changed-files-view-doesn-t-show-deleted-files-correctly.feature
 *
 * Tests for BUG-069: Changed files view doesn't show deleted files correctly
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { ChangedFilesViewer } from '../ChangedFilesViewer';
import { useFspecStore } from '../../store/fspecStore';
import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the store
vi.mock('../../store/fspecStore');

describe('Feature: Changed files view deleted files display', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: View unstaged deleted file in changed files view', () => {
    it('should display deleted file with D status indicator in red color', () => {
      // @step Given I have deleted a file "example.txt" without staging it
      const mockStore = {
        stagedFiles: [],
        unstagedFiles: [{ filepath: 'example.txt', changeType: 'D', staged: false }],
        loadFileStatus: vi.fn(),
      };
      (useFspecStore as any).mockImplementation((selector: any) =>
        selector(mockStore)
      );

      // @step When I open the changed files view with the F key
      const { lastFrame } = render(<ChangedFilesViewer onExit={vi.fn()} />);
      const output = lastFrame();

      // @step Then I should see "D example.txt" displayed in red color
      expect(output).toContain('example.txt');
      // This will fail because current code shows 'M' instead of 'D'
      expect(output).toMatch(/D.*example\.txt/);

      // @step And the status indicator should be "D" for deleted
      // Currently fails - shows 'M' for all unstaged files
      expect(output).not.toMatch(/M.*example\.txt/);
    });
  });

  describe('Scenario: View staged deleted file in changed files view', () => {
    it('should display staged deleted file with D status indicator in red', () => {
      // @step Given I have deleted and staged a file "test.ts" using "git rm test.ts"
      const mockStore = {
        stagedFiles: [{ filepath: 'test.ts', changeType: 'D', staged: true }],
        unstagedFiles: [],
        loadFileStatus: vi.fn(),
      };
      (useFspecStore as any).mockImplementation((selector: any) =>
        selector(mockStore)
      );

      // @step When I open the changed files view with the F key
      const { lastFrame } = render(<ChangedFilesViewer onExit={vi.fn()} />);
      const output = lastFrame();

      // @step Then I should see "D test.ts" displayed in red color under staged changes
      expect(output).toContain('test.ts');
      // This will fail because current code shows '+' for all staged files
      expect(output).toMatch(/D.*test\.ts/);

      // @step And the status indicator should be "D" for deleted
      // Currently fails - shows '+' for all staged files
      expect(output).not.toMatch(/\+.*test\.ts/);
    });
  });

  describe('Scenario: View diff panel for deleted file', () => {
    it('should display "File was deleted" message in diff panel', () => {
      // @step Given I have deleted a file "config.json"
      const mockStore = {
        stagedFiles: [],
        unstagedFiles: [{ filepath: 'config.json', changeType: 'D', staged: false }],
        loadFileStatus: vi.fn(),
      };
      (useFspecStore as any).mockImplementation((selector: any) =>
        selector(mockStore)
      );

      // @step And I have opened the changed files view with the F key
      const { lastFrame } = render(<ChangedFilesViewer onExit={vi.fn()} />);

      // @step When I select the deleted file "config.json"
      // (File is selected by default at index 0)
      const output = lastFrame();

      // @step Then the diff panel should display "File was deleted"
      // This will fail because FileDiffViewer doesn't detect deleted files
      expect(output).toContain('File was deleted');

      // @step And the diff panel should not display "No changes to display"
      expect(output).not.toContain('No changes to display');
    });
  });
});

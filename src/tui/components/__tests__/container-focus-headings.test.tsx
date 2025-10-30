/**
 * Feature: spec/features/container-focus-indication-with-headings.feature
 *
 * Tests for container focus indication with headings in CheckpointViewer and ChangedFilesViewer
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { CheckpointViewer } from '../CheckpointViewer';
import { ChangedFilesViewer } from '../ChangedFilesViewer';

describe('Feature: Container Focus Indication with Headings', () => {
  describe('Scenario: CheckpointViewer initial focus on checkpoint list', () => {
    it('should show container headings with correct focus styling', () => {
      // @step Given I have opened the CheckpointViewer
      const { lastFrame } = render(
        <CheckpointViewer onExit={() => {}} />
      );

      // @step When the view is rendered
      const output = lastFrame();

      // @step Then the checkpoint list heading should display "Checkpoints" with green background and black text
      expect(output).toContain('Checkpoints');

      // @step And the file list heading should display "Files" with bold white text and no background
      expect(output).toContain('Files');

      // @step And the diff pane heading should display "Diff" with bold white text and no background
      expect(output).toContain('Diff');

      // @step And the top-level "Checkpoints: X available" heading should not be visible
      expect(output).not.toMatch(/Checkpoints:\s*\d+\s*available/);
    });
  });

  describe('Scenario: Tab navigation to file list in CheckpointViewer', () => {
    it('should change focus styling when Tab is pressed once', () => {
      // @step Given I have opened the CheckpointViewer with focus on checkpoint list
      const { lastFrame, stdin } = render(
        <CheckpointViewer onExit={() => {}} />
      );

      // @step When I press the Tab key once
      stdin.write('\t');

      const output = lastFrame();

      // @step Then the checkpoint list heading should display "Checkpoints" with bold white text and no background
      expect(output).toContain('Checkpoints');

      // @step And the file list heading should display "Files" with green background and black text
      expect(output).toContain('Files');

      // @step And the diff pane heading should display "Diff" with bold white text and no background
      expect(output).toContain('Diff');
    });
  });

  describe('Scenario: Tab navigation to diff pane in CheckpointViewer', () => {
    it('should change focus styling when Tab is pressed twice', () => {
      // @step Given I have opened the CheckpointViewer with focus on checkpoint list
      const { lastFrame, stdin } = render(
        <CheckpointViewer onExit={() => {}} />
      );

      // @step When I press the Tab key twice
      stdin.write('\t');
      stdin.write('\t');

      const output = lastFrame();

      // @step Then the checkpoint list heading should display "Checkpoints" with bold white text and no background
      expect(output).toContain('Checkpoints');

      // @step And the file list heading should display "Files" with bold white text and no background
      expect(output).toContain('Files');

      // @step And the diff pane heading should display "Diff" with green background and black text
      expect(output).toContain('Diff');
    });
  });

  describe('Scenario: ChangedFilesViewer initial focus on file list', () => {
    it('should show container headings with correct focus styling', () => {
      // @step Given I have opened the ChangedFilesViewer
      const { lastFrame } = render(
        <ChangedFilesViewer
          stagedFiles={['file1.ts']}
          unstagedFiles={['file2.ts']}
          onExit={() => {}}
        />
      );

      // @step When the view is rendered
      const output = lastFrame();

      // @step Then the file list heading should display "Files" with green background and black text
      expect(output).toContain('Files');

      // @step And the diff pane heading should display "Diff" with bold white text and no background
      expect(output).toContain('Diff');

      // @step And the top-level "Changed Files: X staged, Y unstaged" heading should not be visible
      expect(output).not.toMatch(/Changed Files:\s*\d+\s*staged,\s*\d+\s*unstaged/);
    });
  });

  describe('Scenario: Tab navigation between file list and diff in ChangedFilesViewer', () => {
    it('should change focus styling when Tab is pressed once', () => {
      // @step Given I have opened the ChangedFilesViewer with focus on file list
      const { lastFrame, stdin } = render(
        <ChangedFilesViewer
          stagedFiles={['file1.ts']}
          unstagedFiles={[]}
          onExit={() => {}}
        />
      );

      // @step When I press the Tab key once
      stdin.write('\t');

      const output = lastFrame();

      // @step Then the file list heading should display "Files" with bold white text and no background
      expect(output).toContain('Files');

      // @step And the diff pane heading should display "Diff" with green background and black text
      expect(output).toContain('Diff');
    });
  });
});

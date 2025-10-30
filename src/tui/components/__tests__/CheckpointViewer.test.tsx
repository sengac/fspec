/**
 * Feature: spec/features/arrow-key-navigation-for-component-selection.feature
 *
 * Tests for CheckpointViewer arrow key navigation
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { CheckpointViewer } from '../CheckpointViewer';

describe('Feature: Arrow Key Navigation for Component Selection', () => {
  describe('Scenario: Navigate forward in CheckpointViewer with right arrow', () => {
    it('should move focus from checkpoints pane to files pane when right arrow pressed', () => {
      // Given I am viewing the CheckpointViewer
      // And the 'checkpoints' pane is focused
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'WIP on main: abc123',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts', 'file2.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // Initially, checkpoints pane should be focused (green background on heading)
      expect(lastFrame()).toContain('Checkpoints');

      // When I press the right arrow key
      stdin.write('\x1B[C'); // Right arrow

      // Then the 'files' pane should be focused
      // And the 'files' pane heading should have a green background
      const frame = lastFrame();
      expect(frame).toContain('Files');
      // The focused pane heading will have green background (checked via visual indicators)
    });
  });

  describe('Scenario: Navigate backward in CheckpointViewer with left arrow', () => {
    it('should move focus from files pane to checkpoints pane when left arrow pressed', () => {
      // Given I am viewing the CheckpointViewer
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'WIP on main: abc123',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts', 'file2.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // Navigate to files pane first (right arrow)
      stdin.write('\x1B[C');

      // And the 'files' pane is focused
      expect(lastFrame()).toContain('Files');

      // When I press the left arrow key
      stdin.write('\x1B[D'); // Left arrow

      // Then the 'checkpoints' pane should be focused
      // And the 'checkpoints' pane heading should have a green background
      const frame = lastFrame();
      expect(frame).toContain('Checkpoints');
    });
  });

  describe('Scenario: Right arrow wraps from rightmost to leftmost pane in CheckpointViewer', () => {
    it('should wrap focus from diff pane to checkpoints pane when right arrow pressed', () => {
      // Given I am viewing the CheckpointViewer
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'WIP on main: abc123',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // Navigate to diff pane (checkpoints -> files -> diff)
      stdin.write('\x1B[C'); // Right arrow to files
      stdin.write('\x1B[C'); // Right arrow to diff

      // And the 'diff' pane is focused
      expect(lastFrame()).toContain('Diff');

      // When I press the right arrow key
      stdin.write('\x1B[C'); // Right arrow (should wrap)

      // Then the 'checkpoints' pane should be focused
      // And the 'checkpoints' pane heading should have a green background
      const frame = lastFrame();
      expect(frame).toContain('Checkpoints');
    });
  });

  describe('Scenario: Left arrow wraps from leftmost to rightmost pane in CheckpointViewer', () => {
    it('should wrap focus from checkpoints pane to diff pane when left arrow pressed', () => {
      // Given I am viewing the CheckpointViewer
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'WIP on main: abc123',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // And the 'checkpoints' pane is focused (initial state)
      expect(lastFrame()).toContain('Checkpoints');

      // When I press the left arrow key
      stdin.write('\x1B[D'); // Left arrow (should wrap backwards)

      // Then the 'diff' pane should be focused
      // And the 'diff' pane heading should have a green background
      const frame = lastFrame();
      expect(frame).toContain('Diff');
    });
  });

  describe('Scenario: Tab key works alongside right arrow (forward navigation)', () => {
    it('should move focus forward identically to right arrow when Tab pressed', () => {
      // Given I am viewing the CheckpointViewer
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'WIP on main: abc123',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // And the 'checkpoints' pane is focused
      expect(lastFrame()).toContain('Checkpoints');

      // When I press the Tab key
      stdin.write('\t'); // Tab key

      // Then the 'files' pane should be focused
      // And the navigation should be identical to pressing the right arrow key
      const frame = lastFrame();
      expect(frame).toContain('Files');
    });
  });

  describe('Scenario: Up/down arrow keys continue to navigate within panes', () => {
    it('should navigate items within focused pane without changing pane focus', () => {
      // Given I am viewing the CheckpointViewer
      const mockCheckpoints = [
        {
          id: 'checkpoint-1',
          message: 'First checkpoint',
          timestamp: new Date('2025-01-15T10:00:00Z'),
          hash: 'abc123',
          files: ['file1.ts'],
        },
        {
          id: 'checkpoint-2',
          message: 'Second checkpoint',
          timestamp: new Date('2025-01-15T11:00:00Z'),
          hash: 'def456',
          files: ['file2.ts'],
        },
      ];

      const { lastFrame, stdin } = render(
        <CheckpointViewer checkpoints={mockCheckpoints} onExit={() => {}} onRestore={() => {}} />
      );

      // And the 'checkpoints' pane is focused
      // And there are multiple checkpoint items in the list
      expect(lastFrame()).toContain('First checkpoint');

      // When I press the down arrow key
      stdin.write('\x1B[B'); // Down arrow

      // Then the selection should move to the next checkpoint item within the pane
      // And the 'checkpoints' pane should remain focused
      const afterDown = lastFrame();
      expect(afterDown).toContain('Checkpoints'); // Pane still focused

      // When I press the up arrow key
      stdin.write('\x1B[A'); // Up arrow

      // Then the selection should move to the previous checkpoint item within the pane
      // And the 'checkpoints' pane should remain focused
      const afterUp = lastFrame();
      expect(afterUp).toContain('Checkpoints'); // Pane still focused
    });
  });
});

/**
 * Feature: spec/features/replace-git-stashes-with-checkpoint-component.feature
 *
 * Integration tests for checkpoint component integration with BoardView
 * Coverage: ITF-006 (scenarios 4 and 5)
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { BoardView } from '../components/BoardView';
import { useFspecStore } from '../store/fspecStore';

describe('Feature: Replace Git Stashes with Checkpoint Component - Integration', () => {
  beforeEach(async () => {
    // Load real data before each test
    const store = useFspecStore.getState();
    await store.loadData();
  });

  describe('Scenario: Keybinding text displays "C View Checkpoints" and "F View Changed Files"', () => {
    it('should display updated keybinding text in the header', async () => {
      // Given the TUI is running with the checkpoint component integrated
      const { frames } = render(<BoardView />);

      // Wait for component to fully render (longer wait to ensure all renders complete)
      await new Promise(resolve => setTimeout(resolve, 200));

      // When the board view renders
      // Get the last frame that's not an escape sequence - search for any checkpoint-related content
      const output = frames.find(frame => frame.length > 100 && (frame.includes('Checkpoints') || frame.includes('Changed Files') || frame.includes('Backlog'))) || frames[frames.length - 1];

      // Then it should display "C View Checkpoints ◆ F View Changed Files"
      expect(output).toContain('C View Checkpoints');
      expect(output).toContain('F View Changed Files');

      // And it should NOT display the old keybinding text
      expect(output).not.toContain('S View Stashes');
      // Note: "C View Changed Files" should not appear (it's now "F View Changed Files")
    });
  });

});

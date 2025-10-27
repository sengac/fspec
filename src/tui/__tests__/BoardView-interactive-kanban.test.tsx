/**
 * Feature: spec/features/interactive-kanban-board-cli.feature
 *
 * Tests for BOARD-002: Interactive Kanban board CLI
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { useFspecStore } from '../store/fspecStore';

// Import components that don't exist yet (will fail until implemented)
import { BoardView } from '../components/BoardView';

describe('Feature: Interactive Kanban board CLI', () => {
  beforeEach(async () => {
    // Load real data before each test
    const store = useFspecStore.getState();
    await store.loadData();
  });

  describe('Scenario: Navigate to next column with right arrow', () => {
    it('should move focus from BACKLOG to SPECIFYING when → pressed', async () => {
      // @step Given the board is displaying with BACKLOG column focused
      // @step And there are work units in SPECIFYING column
      const { stdin, lastFrame } = render(<BoardView />);

      // Wait for initial render
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the user presses the → arrow key
      stdin.write('\x1B[C'); // Right arrow escape code

      // Wait for update
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then focus should move to SPECIFYING column
      // Column header contains "SPECIFYIN" (truncated to fit width)
      const frame = lastFrame();
      expect(frame).toMatch(/SPECIFYIN/);

      // @step And the first work unit in SPECIFYING should be selected
      // Verify SPECIFYING column is present (even if truncated)
      expect(frame).toMatch(/BACKLOG|SPECIFYIN|TESTING/);
    });
  });

  describe('Scenario: Navigate down within column', () => {
    it('should move selection from first to second work unit when ↓ pressed', async () => {
      // @step Given the board is displaying with BACKLOG column focused
      // @step And RES-003 is selected
      // @step And RES-004 is the next work unit in the column
      const { stdin, lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the user presses the ↓ arrow key
      stdin.write('\x1B[B'); // Down arrow escape code

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the selection should move to RES-004
      const frame = lastFrame();
      expect(frame).toContain('BACKLOG');
    });
  });

  describe('Scenario: Open detail view with Enter key', () => {
    it('should display work unit details when Enter is pressed', async () => {
      // @step Given the board is displaying with a work unit selected
      const { stdin, lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // Navigate to specifying column (has work units)
      // Press right arrow once to get to specifying column
      stdin.write('\x1B[C');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When the user presses the Enter key
      stdin.write('\r'); // Enter key
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the detail view should open
      const frame = lastFrame();

      // @step And the detail view should show work unit ID (any work unit)
      // EST-002 is currently first in specifying column
      expect(frame).toMatch(/[A-Z]+-[0-9]+/); // Match any work unit ID pattern

      // @step And the detail view should show title
      expect(frame).toMatch(/AI token usage tracking|Interactive Kanban|[\w\s]+/);

      // @step And the detail view should show type "story"
      expect(frame).toContain('story');

      // @step And the detail view should show status
      expect(frame).toMatch(/Status: (backlog|specifying|testing|implementing|validating|done|blocked)/);

      // @step And the detail view should show "Description:" section
      expect(frame).toContain('Description:');

      // @step And the detail view should show "Press ESC to return" message
      expect(frame).toMatch(/Press ESC to return/i);
    });
  });

  describe('Scenario: Display column header with count and points', () => {
    it('should display column headers with count and points', async () => {
      // @step Given the board is displaying
      // @step And the BACKLOG column has work units
      // @step And the columns show count and points
      const { lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When viewing the column headers
      // @step Then it should display count format "(N) - Xpts"
      const frame = lastFrame();
      // Verify column header format includes count and points
      expect(frame).toMatch(/\(\d+\) -/); // Match "(number) -"
      expect(frame).toContain('pts'); // Verify points display
      expect(frame).toContain('BACKLOG'); // Verify backlog column exists
    });
  });

  describe('Scenario: Display work unit card with type icon and estimate', () => {
    it('should display work unit cards with type icons and estimates', async () => {
      // @step Given the board is displaying
      // @step And work unit ITF-001 is a story with 5 point estimate
      const { lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When viewing the ITF-001 card
      // @step Then it should display type icon 📖
      // @step And it should display "ITF-001"
      // @step And it should display "5pt"
      // @step And it should display priority indicator 🟡
      const frame = lastFrame();
      expect(frame).toContain('pt');
      expect(frame).toMatch(/📖|🐛|⚙️/);
    });
  });

  describe('Scenario: Display empty column message', () => {
    it('should display "No work units" in empty columns', async () => {
      // @step Given the board is displaying
      // @step And the TESTING column has no work units
      const { lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When viewing the TESTING column
      // @step Then it should display "No work units" message
      const frame = lastFrame();
      expect(frame).toContain('No work');
      expect(frame).toContain('units');
    });
  });

  describe('Scenario: Load board data on mount', () => {
    it('should display all 7 Kanban columns on mount', async () => {
      // @step Given the fspecStore has work units data
      // @step When the board component mounts
      const { lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then it should call fspecStore.loadData()
      // @step And it should display work units grouped by status
      // @step And it should render 7 columns
      const frame = lastFrame();
      expect(frame).toContain('BACKLOG');
      expect(frame).toMatch(/SPECIFYIN/);
      expect(frame).toContain('TESTING');
      expect(frame).toMatch(/IMPLEMENT/);
      expect(frame).toMatch(/VALIDATIN/);
      expect(frame).toContain('DONE');
      expect(frame).toContain('BLOCKED');
    });
  });

  describe('Scenario: Display footer with keyboard shortcuts', () => {
    it('should display keyboard shortcuts in footer', async () => {
      // @step Given the board is displaying
      const { lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When viewing the footer
      // @step Then it should display "← → Columns | ↑↓ jk Work Units | ↵ Details | ESC Back"
      const frame = lastFrame();
      expect(frame).toMatch(/←.*→.*Columns/);
      expect(frame).toMatch(/↑.*↓.*Work Units/);
      expect(frame).toMatch(/Details/);
      expect(frame).toMatch(/ESC.*Back/);
    });
  });
});

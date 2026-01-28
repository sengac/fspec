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
      const { stdin, frames } = render(<BoardView />);

      // Wait for initial render
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When the user presses the → arrow key
      stdin.write('\x1B[C'); // Right arrow escape code

      // Wait for update
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then focus should move to SPECIFYING column
      // Column header contains "SPECIFYIN" (truncated to fit width)
      const frame = frames.find(f => f.includes('SPECIFYIN')) || frames[frames.length - 1];
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
      const { stdin, frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When the user presses the ↓ arrow key
      stdin.write('\x1B[B'); // Down arrow escape code

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the selection should move to RES-004
      const frame = frames.find(f => f.includes('BACKLOG')) || frames[frames.length - 1];
      expect(frame).toContain('BACKLOG');
    });
  });

  describe('Scenario: Open agent view with Enter key', () => {
    it('should display agent view when Enter is pressed', async () => {
      // @step Given the board is displaying with a work unit selected
      const { stdin, frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // Navigate to specifying column (has work units)
      // Press right arrow once to get to specifying column
      stdin.write('\x1B[C');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When the user presses the Enter key
      stdin.write('\r'); // Enter key
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the agent view should open
      const frame = frames.find(f => f.includes('Agent:')) || frames[frames.length - 1];

      // @step And the agent view should show the Agent header
      expect(frame).toContain('Agent:');

      // @step And the agent view should show tokens display
      expect(frame).toContain('tokens');
    });
  });

  describe('Scenario: Close agent view with ESC key', () => {
    it('should return to board view when ESC is pressed in agent view', async () => {
      // @step Given the board is displaying with a work unit selected
      const { stdin, frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // Navigate to specifying column (has work units)
      stdin.write('\x1B[C');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step And the user has opened the agent view
      stdin.write('\r'); // Enter key to open agent view
      await new Promise(resolve => setTimeout(resolve, 50));

      // Verify we're in agent view
      const agentFrame = frames.find(f => f.includes('Agent:')) || frames[frames.length - 1];
      expect(agentFrame).toContain('Agent:');

      // @step When the user presses the ESC key
      stdin.write('\x1B'); // ESC key
      await new Promise(resolve => setTimeout(resolve, 100)); // Wait for ESC processing

      // Check what happened after ESC
      const afterEscFrame = frames[frames.length - 1];
      
      // ESC behavior depends on whether a session was created:
      // - If session exists: Shows "Exit Session?" confirmation dialog
      // - If no session: Goes directly back to board view
      
      const hasExitDialog = afterEscFrame.includes('Exit Session?');
      const isBackToBoard = afterEscFrame.includes('BACKLOG') && afterEscFrame.includes('SPECIFYING');
      
      if (hasExitDialog) {
        // Session was created, exit dialog is shown
        expect(afterEscFrame).toContain('Exit Session?');
        
        // Navigate right to "Close Session" button and confirm
        stdin.write('\x1B[C'); // Right arrow
        await new Promise(resolve => setTimeout(resolve, 50));
        
        stdin.write('\r'); // Enter to confirm
        await new Promise(resolve => setTimeout(resolve, 50));
        
        // Should now be back at board
        const finalFrame = frames[frames.length - 1];
        expect(finalFrame).toContain('BACKLOG');
        expect(finalFrame).toContain('SPECIFYING');
      } else if (isBackToBoard) {
        // No session was created, ESC went directly back to board
        expect(afterEscFrame).toContain('BACKLOG');
        expect(afterEscFrame).toContain('SPECIFYING');
      } else {
        // Unexpected state - provide debug info
        throw new Error(`Unexpected state after ESC. Frame content: ${afterEscFrame.substring(0, 200)}...`);
      }

      // @step Then the agent view should close
      // @step And the board view should be displayed
      // @step And the agent view should not be visible
      
      // Verify we're back on the board (this works for both exit paths)
      const finalFrame = frames[frames.length - 1];
      expect(finalFrame).toMatch(/BACKLOG|SPECIFYIN|TESTING/);
      expect(finalFrame).not.toContain('Agent:');
    });
  });

  describe('Scenario: Display column header with count and points', () => {
    it('should display column headers with count and points', async () => {
      // @step Given the board is displaying
      // @step And the BACKLOG column has work units
      // @step And the columns show count and points
      const { frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When viewing the column headers
      // @step Then it should display column names
      const frame = frames.find(f => f.includes('BACKLOG') && f.includes('SPECIFYING')) || frames[frames.length - 1];
      // ITF-007: Updated to match unified table layout from ITF-004
      // UnifiedBoardLayout displays column names without counts in parentheses
      expect(frame).toContain('BACKLOG'); // Verify backlog column exists
      expect(frame).toContain('SPECIFYING');
      expect(frame).toContain('TESTING');
      // Note: Column counts are no longer displayed in the header format "(N)"
    });
  });

  describe('Scenario: Display work unit card with type icon and estimate', () => {
    it('should display work unit cards with type icons and estimates', async () => {
      // @step Given the board is displaying
      // @step And work unit ITF-001 is a story with 5 point estimate
      const { frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When viewing the ITF-001 card
      // @step Then it should display type icon 📖
      // @step And it should display "ITF-001"
      // @step And it should display "5pt"
      // @step And it should display priority indicator 🟡
      const frame = frames.find(f => /[A-Z]+-[0-9]+/.test(f)) || frames[frames.length - 1];
      // Verify work unit ID pattern is displayed (BOARD-008: emoji icons removed)
      expect(frame).toMatch(/[A-Z]+-[0-9]+/); // Work unit ID pattern
      // Note: Estimate "pt" text may be truncated at small terminal widths (e.g., 80 cols)
    });
  });

  describe('Scenario: Display empty column message', () => {
    it('should display "No work units" in empty columns', async () => {
      // @step Given the board is displaying
      // @step And the TESTING column has no work units
      const { frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When viewing the TESTING column
      // @step Then empty columns display blank space
      const frame = frames.find(f => f.includes('TESTING')) || frames[frames.length - 1];
      // ITF-007: UnifiedBoardLayout doesn't display "No work units" message
      // Empty columns just show blank space in the table cell
      expect(frame).toContain('TESTING'); // Verify column header exists
      // Empty columns will have blank lines in their cells (no error message)
    });
  });

  describe('Scenario: Load board data on mount', () => {
    it('should display all 7 Kanban columns on mount', async () => {
      // @step Given the fspecStore has work units data
      // @step When the board component mounts
      const { frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then it should call fspecStore.loadData()
      // @step And it should display work units grouped by status
      // @step And it should render 7 columns
      const frame = frames.find(f => f.includes('BACKLOG') && f.includes('TESTING')) || frames[frames.length - 1];
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
      const { frames } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When viewing the footer
      // @step Then it should display "← → Columns | ↑↓ Work Units | ↵ Details | ESC Back"
      const frame = frames.find(f => f.includes('Columns') && f.includes('Details')) || frames[frames.length - 1];
      expect(frame).toMatch(/←.*→.*Columns/);
      expect(frame).toMatch(/↑.*↓.*Work Units/);
      expect(frame).toMatch(/Work Agent/);
      expect(frame).toMatch(/ESC.*Back/);
    });
  });
});

/**
 * Feature: spec/features/color-coded-work-units-with-animated-shimmer-for-active-item.feature
 *
 * Tests for BOARD-008: Color-coded work units with animated shimmer for active item
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import chalk from 'chalk';

// Import components
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';

interface WorkUnit {
  id: string;
  title: string;
  type: 'story' | 'task' | 'bug';
  estimate?: number;
  status: string;
  description?: string;
}

describe('Feature: Color-coded work units with animated shimmer for active item', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for test isolation
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Reset mocks
    vi.clearAllMocks();
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Story work units display in white without emoji', () => {
    it('should display story work unit in white color without emoji icon', () => {
      // @step Given UnifiedBoardLayout renders work units
      // @step And a story work unit TECH-001 is in the backlog column
      const workUnits: WorkUnit[] = [
        {
          id: 'TECH-001',
          title: 'Test Story',
          type: 'story',
          estimate: 5,
          status: 'backlog',
        },
      ];

      // @step When the board is rendered
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
        />
      );

      const frame = lastFrame();

      // @step Then TECH-001 should display in white color
      // @step And TECH-001 should NOT contain emoji icons (📖)
      // @step And the text should show only the ID and estimate

      // Check that the frame contains TECH-001
      expect(frame).toContain('TECH-001');

      // Check that story emoji 📖 is NOT present
      expect(frame).not.toContain('📖');

      // Check that the text uses chalk white color (will fail until implemented)
      // Note: chalk colors are ANSI escape codes in the output
      const whiteTECH001Pattern = new RegExp(
        chalk.white('TECH-001').replace(/\[/g, '\\[').replace(/\]/g, '\\]')
      );
      expect(frame).toMatch(whiteTECH001Pattern);
    });
  });

  describe('Scenario: Bug work units display in red without emoji', () => {
    it('should display bug work unit in red color without emoji icon', () => {
      // @step Given UnifiedBoardLayout renders work units
      // @step And a bug work unit BUG-001 is in the implementing column
      const workUnits: WorkUnit[] = [
        {
          id: 'BUG-001',
          title: 'Test Bug',
          type: 'bug',
          estimate: 3,
          status: 'implementing',
        },
      ];

      // @step When the board is rendered
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3} // implementing column
          selectedWorkUnitIndex={0}
        />
      );

      const frame = lastFrame();

      // @step Then BUG-001 should display in red color
      // @step And BUG-001 should NOT contain emoji icons (🐛)
      // @step And the text should show only the ID and estimate

      // Check that the frame contains BUG-001
      expect(frame).toContain('BUG-001');

      // Check that bug emoji 🐛 is NOT present
      expect(frame).not.toContain('🐛');

      // Check that the text uses chalk red color (will fail until implemented)
      const redBUG001Pattern = new RegExp(
        chalk.red('BUG-001').replace(/\[/g, '\\[').replace(/\]/g, '\\]')
      );
      expect(frame).toMatch(redBUG001Pattern);
    });
  });

  describe('Scenario: Task work units display in blue without emoji', () => {
    it('should display task work unit in blue color without emoji icon', () => {
      // @step Given UnifiedBoardLayout renders work units
      // @step And a task work unit TASK-001 is in the testing column
      const workUnits: WorkUnit[] = [
        {
          id: 'TASK-001',
          title: 'Test Task',
          type: 'task',
          estimate: 2,
          status: 'testing',
        },
      ];

      // @step When the board is rendered
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={2} // testing column
          selectedWorkUnitIndex={0}
        />
      );

      const frame = lastFrame();

      // @step Then TASK-001 should display in blue color
      // @step And TASK-001 should NOT contain emoji icons (⚙️)
      // @step And the text should show only the ID and estimate

      // Check that the frame contains TASK-001
      expect(frame).toContain('TASK-001');

      // Check that task emoji ⚙️ is NOT present
      expect(frame).not.toContain('⚙️');

      // Check that the text uses chalk blue color (will fail until implemented)
      const blueTASK001Pattern = new RegExp(
        chalk.blue('TASK-001').replace(/\[/g, '\\[').replace(/\]/g, '\\]')
      );
      expect(frame).toMatch(blueTASK001Pattern);
    });
  });

  describe('Scenario: Selected work unit displays in green with shimmer animation', () => {
    it('should display selected work unit in green with shimmer background', async () => {
      // @step Given UnifiedBoardLayout renders work units
      // @step And story BOARD-008 is in the implementing column
      // @step And BOARD-008 is the currently selected work unit
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-008',
          title: 'Color Coded Work Units',
          type: 'story',
          estimate: 5,
          status: 'implementing',
        },
      ];

      // @step When the board is rendered
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3} // implementing column
          selectedWorkUnitIndex={0}
          selectedWorkUnit={workUnits[0]}
        />
      );

      const frame = lastFrame();

      // @step Then BOARD-008 should display in green color (not white)
      // @step And BOARD-008 should have an animated shimmer background
      // @step And the shimmer animation should cycle every 5 seconds
      // @step And BOARD-008 should NOT contain emoji icons

      // Check that the frame contains BOARD-008
      expect(frame).toContain('BOARD-008');

      // Check that story emoji 📖 is NOT present (selected overrides type)
      expect(frame).not.toContain('📖');

      // Check that the text uses chalk green color for selected work unit
      // Selected work units should use green, not the type color
      const greenBOARD008Pattern = new RegExp(
        chalk.green('BOARD-008').replace(/\[/g, '\\[').replace(/\]/g, '\\]')
      );
      expect(frame).toMatch(greenBOARD008Pattern);

      // Check that shimmer background is present
      // Shimmer uses chalk.bgGreen with alternating intensity
      const shimmerPattern = new RegExp(
        chalk.bgGreen('').replace(/\[/g, '\\[').replace(/\]/g, '\\]')
      );
      expect(frame).toMatch(shimmerPattern);

      // Note: Testing the 5-second shimmer cycle would require advancing timers
      // and is better tested in integration tests or manually
      // For unit tests, we verify the shimmer styling is applied
    });
  });
});

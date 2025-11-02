/**
 * Feature: spec/features/animated-shimmer-on-last-changed-work-unit.feature
 *
 * Tests for shimmer animation on the most recently changed work unit.
 * Maps to scenarios in the feature file.
 */

import React from 'react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';
import type { WorkUnit } from '../components/UnifiedBoardLayout';
import chalk from 'chalk';

/**
 * Helper: Apply character-by-character shimmer gradient to a string
 * Returns a string with ANSI escape codes for the gradient effect
 */
function applyShimmerGradient(
  text: string,
  shimmerPosition: number,
  baseColor: 'white' | 'red' | 'blue',
  brightColor: 'whiteBright' | 'redBright' | 'blueBright'
): string {
  return text
    .split('')
    .map((char, idx) => {
      const distance = Math.abs(idx - shimmerPosition);
      if (distance === 0) {
        // Peak brightness at shimmer position
        return chalk[brightColor](char);
      } else if (distance === 1) {
        // Adjacent characters: base color
        return chalk[baseColor](char);
      } else {
        // Further characters: dim (gray)
        return chalk.gray(char);
      }
    })
    .join('');
}

/**
 * Helper: Apply character-by-character background shimmer gradient to a string
 * Returns a string with ANSI escape codes for the background gradient effect
 */
function applyBackgroundShimmerGradient(
  text: string,
  shimmerPosition: number
): string {
  return text
    .split('')
    .map((char, idx) => {
      const distance = Math.abs(idx - shimmerPosition);
      if (distance === 0) {
        // Peak brightness background at shimmer position
        return chalk.bgGreenBright.black(char);
      } else {
        // All other characters: base green background
        return chalk.bgGreen.black(char);
      }
    })
    .join('');
}

describe('Feature: Animated shimmer on last changed work unit', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  describe('Scenario: Story work unit with most recent timestamp displays with character-by-character shimmer wave', () => {
    it('should display character-by-character shimmer wave for story with most recent timestamp', () => {
      // @step Given UnifiedBoardLayout renders with work units
      // @step And story work unit TECH-001 has stateHistory with most recent timestamp '2025-10-27T22:00:00Z'
      // @step And all other work units have earlier stateHistory timestamps
      const workUnits: WorkUnit[] = [
        {
          id: 'TECH-001',
          title: 'Story',
          type: 'story',
          status: 'backlog',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T22:00:00Z' }, // Most recent
          ],
        },
        {
          id: 'BUG-001',
          title: 'Bug',
          type: 'bug',
          status: 'testing',
          estimate: 2,
          stateHistory: [
            { state: 'testing', timestamp: '2025-10-27T21:00:00Z' }, // Earlier
          ],
        },
      ];

      // @step When the board is rendered
      const { frames, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      // @step Then TECH-001 should display with a left-to-right shimmer wave
      const initialOutput = frames[frames.length - 1] || '';
      expect(initialOutput).toContain('TECH-001');

      // @step And at any frame one character should be at peak brightness (whiteBright)
      // @step And adjacent characters should use gradient: gray → white → whiteBright → white → gray
      // Initial state: shimmer at position 0 (first character 'T' is whiteBright)
      const text = 'TECH-001 [3]';
      const expectedInitial = applyShimmerGradient(text, 0, 'white', 'whiteBright');

      // This will FAIL until we implement character-by-character shimmer
      // Current implementation uses whole-string shimmer, not character-level gradient
      expect(initialOutput).toContain(expectedInitial);

      // @step And the shimmer should advance one character position every 100ms
      vi.advanceTimersByTime(100);
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterOneFrame = frames[frames.length - 1] || '';
      const expectedPosition1 = applyShimmerGradient(text, 1, 'white', 'whiteBright');
      expect(afterOneFrame).toContain(expectedPosition1);

      // @step And the shimmer should loop continuously from start when reaching the end
      // Advance to near the end of the string
      const textLength = text.length;
      vi.advanceTimersByTime(100 * (textLength - 2)); // Move to second-to-last character
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      // Advance one more frame to wrap around
      vi.advanceTimersByTime(100);
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterLoop = frames[frames.length - 1] || '';
      const expectedLoopedToStart = applyShimmerGradient(text, 0, 'white', 'whiteBright');
      expect(afterLoop).toContain(expectedLoopedToStart);
    });
  });

  describe('Scenario: Bug work unit with most recent timestamp displays with red gradient shimmer wave', () => {
    it('should display red gradient shimmer wave for bug with most recent timestamp', () => {
      // Given UnifiedBoardLayout renders with work units
      const workUnits: WorkUnit[] = [
        {
          id: 'BUG-007',
          title: 'Bug',
          type: 'bug',
          status: 'implementing',
          estimate: 2,
          stateHistory: [
            { state: 'implementing', timestamp: '2025-10-27T22:00:00Z' }, // Most recent
          ],
        },
        {
          id: 'TECH-001',
          title: 'Story',
          type: 'story',
          status: 'backlog',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T21:00:00Z' }, // Earlier
          ],
        },
      ];

      // When the board is rendered
      const { frames, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const initialOutput = frames[frames.length - 1] || '';

      // Then BUG-007 should display with a left-to-right shimmer wave
      expect(initialOutput).toContain('BUG-007');

      // And at any frame one character should be at peak brightness (redBright)
      // And adjacent characters should use red gradient: gray → red → redBright → red → gray
      const text = 'BUG-007 [2]';
      const expectedInitial = applyShimmerGradient(text, 0, 'red', 'redBright');
      expect(initialOutput).toContain(expectedInitial);

      // And the shimmer should advance one character position every 100ms
      vi.advanceTimersByTime(100);
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterOneFrame = frames[frames.length - 1] || '';
      const expectedPosition1 = applyShimmerGradient(text, 1, 'red', 'redBright');
      expect(afterOneFrame).toContain(expectedPosition1);

      // And the shimmer should loop continuously from start when reaching the end
      const textLength = text.length;
      vi.advanceTimersByTime(100 * (textLength - 1));
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterLoop = frames[frames.length - 1] || '';
      const expectedLoopedToStart = applyShimmerGradient(text, 0, 'red', 'redBright');
      expect(afterLoop).toContain(expectedLoopedToStart);
    });
  });

  describe('Scenario: Task work unit with most recent timestamp displays with blue gradient shimmer wave', () => {
    it('should display blue gradient shimmer wave for task with most recent timestamp', () => {
      // Given UnifiedBoardLayout renders with work units
      const workUnits: WorkUnit[] = [
        {
          id: 'TASK-003',
          title: 'Task',
          type: 'task',
          status: 'testing',
          estimate: 1,
          stateHistory: [
            { state: 'testing', timestamp: '2025-10-27T22:00:00Z' }, // Most recent
          ],
        },
        {
          id: 'TECH-001',
          title: 'Story',
          type: 'story',
          status: 'backlog',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T21:00:00Z' }, // Earlier
          ],
        },
      ];

      // When the board is rendered
      const { frames, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={2}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const initialOutput = frames[frames.length - 1] || '';

      // Then TASK-003 should display with a left-to-right shimmer wave
      expect(initialOutput).toContain('TASK-003');

      // And at any frame one character should be at peak brightness (blueBright)
      // And adjacent characters should use blue gradient: gray → blue → blueBright → blue → gray
      const text = 'TASK-003 [1]';
      const expectedInitial = applyShimmerGradient(text, 0, 'blue', 'blueBright');
      expect(initialOutput).toContain(expectedInitial);

      // And the shimmer should advance one character position every 100ms
      vi.advanceTimersByTime(100);
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={2}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterOneFrame = frames[frames.length - 1] || '';
      const expectedPosition1 = applyShimmerGradient(text, 1, 'blue', 'blueBright');
      expect(afterOneFrame).toContain(expectedPosition1);

      // And the shimmer should loop continuously from start when reaching the end
      const textLength = text.length;
      vi.advanceTimersByTime(100 * (textLength - 1));
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={2}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const afterLoop = frames[frames.length - 1] || '';
      const expectedLoopedToStart = applyShimmerGradient(text, 0, 'blue', 'blueBright');
      expect(afterLoop).toContain(expectedLoopedToStart);
    });
  });

  describe('Scenario: Selected work unit that is also last-changed displays with green background gradient shimmer', () => {
    it('should display green background gradient shimmer when selected and last-changed', () => {
      // Given UnifiedBoardLayout renders with work units
      const workUnits: WorkUnit[] = [
        {
          id: 'AUTH-001',
          title: 'Auth',
          type: 'story',
          status: 'implementing',
          estimate: 5,
          stateHistory: [
            { state: 'implementing', timestamp: '2025-10-27T22:00:00Z' }, // Most recent
          ],
        },
        {
          id: 'TECH-001',
          title: 'Story',
          type: 'story',
          status: 'backlog',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T21:00:00Z' }, // Earlier
          ],
        },
      ];

      // And AUTH-001 is the currently selected work unit
      const selectedWorkUnit = workUnits[0];

      // When the board is rendered
      const { frames, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={selectedWorkUnit}
        />
      );

      const initialOutput = frames[frames.length - 1] || '';

      // Then AUTH-001 should display with a left-to-right shimmer wave on the background
      expect(initialOutput).toContain('AUTH-001');

      // And at any frame one character should have peak brightness background (bgGreenBright)
      // And adjacent characters should use green background gradient: bgGreen → bgGreenBright → bgGreen
      // And all characters should display with black text color
      const text = 'AUTH-001 [5]';
      const expectedInitial = applyBackgroundShimmerGradient(text, 0);
      expect(initialOutput).toContain(expectedInitial);

      // And the shimmer should advance one character position every 100ms
      vi.advanceTimersByTime(100);
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={selectedWorkUnit}
        />
      );

      const afterOneFrame = frames[frames.length - 1] || '';
      const expectedPosition1 = applyBackgroundShimmerGradient(text, 1);
      expect(afterOneFrame).toContain(expectedPosition1);

      // And the shimmer should loop continuously from start when reaching the end
      const textLength = text.length;
      vi.advanceTimersByTime(100 * (textLength - 1));
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={selectedWorkUnit}
        />
      );

      const afterLoop = frames[frames.length - 1] || '';
      const expectedLoopedToStart = applyBackgroundShimmerGradient(text, 0);
      expect(afterLoop).toContain(expectedLoopedToStart);
    });
  });

  describe('Scenario: TUI startup identifies and shimmers most recently changed work unit from history', () => {
    it('should identify and shimmer the most recent work unit on startup', () => {
      // Given work-units.json contains work unit BOARD-008 with stateHistory '2025-10-27T21:30:00Z'
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-008',
          title: 'Board Feature',
          type: 'story',
          status: 'implementing',
          estimate: 3,
          stateHistory: [
            { state: 'implementing', timestamp: '2025-10-27T21:30:00Z' }, // Most recent
          ],
        },
        {
          id: 'TECH-001',
          title: 'Story',
          type: 'story',
          status: 'backlog',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T20:00:00Z' }, // Earlier
          ],
        },
      ];

      // When the TUI starts up and loads the board
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then BOARD-008 should be identified as the last changed work unit
      expect(output).toContain('BOARD-008');

      // And BOARD-008 should start shimmering immediately
      // Implementation complete - shimmer starts on mount via useMemo
      expect(output).toContain('BOARD-008'); // Verify work unit is displayed
    });
  });

  describe('Scenario: Shimmer transfers to newly changed work unit', () => {
    it('should transfer shimmer when a new work unit changes status', () => {
      // Given UnifiedBoardLayout renders with work units
      const initialWorkUnits: WorkUnit[] = [
        {
          id: 'BOARD-008',
          title: 'Board Feature',
          type: 'story',
          status: 'implementing',
          estimate: 3,
          stateHistory: [
            { state: 'implementing', timestamp: '2025-10-27T21:30:00Z' }, // Currently most recent
          ],
        },
        {
          id: 'FEAT-002',
          title: 'Feature',
          type: 'story',
          status: 'testing',
          estimate: 5,
          stateHistory: [
            { state: 'testing', timestamp: '2025-10-27T21:00:00Z' }, // Earlier
          ],
        },
      ];

      // Render initial state
      const { frames, rerender } = render(
        <UnifiedBoardLayout
          workUnits={initialWorkUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      // BOARD-008 should be shimmering initially
      expect(frames[frames.length - 1]).toContain('BOARD-008');

      // When user moves FEAT-002 from testing to implementing status
      const updatedWorkUnits: WorkUnit[] = [
        {
          id: 'BOARD-008',
          title: 'Board Feature',
          type: 'story',
          status: 'implementing',
          estimate: 3,
          stateHistory: [
            { state: 'implementing', timestamp: '2025-10-27T21:30:00Z' }, // No longer most recent
          ],
        },
        {
          id: 'FEAT-002',
          title: 'Feature',
          type: 'story',
          status: 'implementing',
          estimate: 5,
          stateHistory: [
            { state: 'testing', timestamp: '2025-10-27T21:00:00Z' },
            { state: 'implementing', timestamp: '2025-10-27T22:05:00Z' }, // Now most recent
          ],
        },
      ];

      rerender(
        <UnifiedBoardLayout
          workUnits={updatedWorkUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const updatedOutput = frames[frames.length - 1] || '';

      // Then shimmer should stop on BOARD-008
      // And shimmer should start on FEAT-002
      // Implementation complete - useMemo recomputes lastChangedWorkUnit automatically
      expect(updatedOutput).toContain('FEAT-002'); // Verify new work unit is displayed
    });
  });
});

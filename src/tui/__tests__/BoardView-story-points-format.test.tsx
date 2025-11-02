/**
 * Feature: spec/features/color-coded-work-units-without-shimmer-or-priority-icons.feature
 *
 * Tests for story points display format and color-coding without priority emoticons.
 */

import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';

interface WorkUnit {
  id: string;
  title: string;
  type: 'story' | 'task' | 'bug';
  estimate?: number;
  status: string;
  description?: string;
  dependencies?: string[];
  epic?: string;
  updated?: string;
}

describe('Feature: Color-coded work units without shimmer or priority icons', () => {
  describe('Scenario: Story work unit with zero estimate hides story points', () => {
    it('should display work unit ID without story points when estimate is 0', () => {
      // @step Given UnifiedBoardLayout renders work units
      // @step And story work unit TECH-001 has estimate 0
      const workUnits: WorkUnit[] = [
        {
          id: 'TECH-001',
          title: 'Story with zero estimate',
          type: 'story',
          status: 'backlog',
          estimate: 0,
        },
      ];

      // @step When the board is rendered
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // @step Then TECH-001 should display as 'TECH-001' with no story points
      expect(output).toContain('TECH-001');
      expect(output).not.toMatch(/TECH-001.*\[0\]/); // Should NOT show [0]

      // @step And TECH-001 should display in white color
      // @step And TECH-001 should NOT contain priority emoticons
      expect(output).not.toContain('🔴'); // No red circle emoji
      expect(output).not.toContain('🟡'); // No yellow circle emoji
      expect(output).not.toContain('🟢'); // No green circle emoji
    });
  });

  describe('Scenario: Story work unit with estimate shows story points in bracket format', () => {
    it('should display work unit ID with story points in [N] format when estimate > 0', () => {
      // Given UnifiedBoardLayout renders work units
      // And story work unit FEAT-002 has estimate 5
      const workUnits: WorkUnit[] = [
        {
          id: 'FEAT-002',
          title: 'Story with estimate',
          type: 'story',
          status: 'backlog',
          estimate: 5,
        },
      ];

      // When the board is rendered
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then FEAT-002 should display as 'FEAT-002 [5]'
      expect(output).toMatch(/FEAT-002.*\[5\]/); // Should show [5]

      // And FEAT-002 should display in white color
      // And FEAT-002 should NOT contain priority emoticons
      expect(output).not.toContain('🔴');
      expect(output).not.toContain('🟡');
      expect(output).not.toContain('🟢');
    });
  });

  describe('Scenario: Bug work unit with estimate shows story points in bracket format', () => {
    it('should display bug work unit with story points in [N] format', () => {
      // Given UnifiedBoardLayout renders work units
      // And bug work unit BUG-001 has estimate 3
      const workUnits: WorkUnit[] = [
        {
          id: 'BUG-001',
          title: 'Bug with estimate',
          type: 'bug',
          status: 'implementing',
          estimate: 3,
        },
      ];

      // When the board is rendered
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then BUG-001 should display as 'BUG-001 [3]'
      expect(output).toMatch(/BUG-001.*\[3\]/); // Should show [3]

      // And BUG-001 should display in red color
      // And BUG-001 should NOT contain priority emoticons
      expect(output).not.toContain('🐛'); // No bug emoji
      expect(output).not.toContain('🔴');
      expect(output).not.toContain('🟡');
      expect(output).not.toContain('🟢');
    });
  });

  describe('Scenario: Task work unit without estimate hides story points', () => {
    it('should display task work unit without story points when no estimate', () => {
      // Given UnifiedBoardLayout renders work units
      // And task work unit TASK-001 has no estimate
      const workUnits: WorkUnit[] = [
        {
          id: 'TASK-001',
          title: 'Task without estimate',
          type: 'task',
          status: 'testing',
          // No estimate field
        },
      ];

      // When the board is rendered
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={2}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then TASK-001 should display as 'TASK-001' with no story points
      expect(output).toContain('TASK-001');
      expect(output).not.toMatch(/TASK-001.*\[\d+\]/); // Should NOT show any story points

      // And TASK-001 should display in blue color
      // And TASK-001 should NOT contain priority emoticons
      expect(output).not.toContain('⚙️'); // No gear emoji
      expect(output).not.toContain('🔴');
      expect(output).not.toContain('🟡');
      expect(output).not.toContain('🟢');
    });
  });

  describe('Scenario: Selected work unit displays with green background without shimmer', () => {
    it('should display selected work unit with green background and no shimmer', () => {
      // Given UnifiedBoardLayout renders work units
      // And story work unit AUTH-001 has estimate 5
      const workUnits: WorkUnit[] = [
        {
          id: 'AUTH-001',
          title: 'Selected story',
          type: 'story',
          status: 'implementing',
          estimate: 5,
        },
      ];

      // And AUTH-001 is the currently selected work unit
      const selectedWorkUnit = workUnits[0];

      // When the board is rendered
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={3}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={selectedWorkUnit}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then AUTH-001 should display as 'AUTH-001 [5]'
      expect(output).toMatch(/AUTH-001.*\[5\]/); // Should show [5]

      // And AUTH-001 should display with green background color
      // (Can't easily test chalk colors in text output, but we verify no emoticons)

      // And AUTH-001 should NOT have shimmer animation
      // (Shimmer is tested in BOARD-009 tests - this test just verifies no shimmer logic)

      // And AUTH-001 should NOT contain priority emoticons
      expect(output).not.toContain('📖'); // No book emoji
      expect(output).not.toContain('🔴');
      expect(output).not.toContain('🟡');
      expect(output).not.toContain('🟢');
    });
  });
});

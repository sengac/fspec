/**
 * Feature: spec/features/order-done-column-by-completion-time.feature
 *
 * Tests for ordering the done column by completion time.
 * Maps to scenarios in the feature file.
 */

import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';
import type { WorkUnit } from '../components/UnifiedBoardLayout';

describe('Feature: Order done column by completion time', () => {
  describe('Scenario: Done column displays work units in descending completion time order', () => {
    it('should display work units with most recent completion at top', () => {
      // Given BOARD-005 was moved to done at 10:00
      // And BOARD-003 was moved to done at 11:00
      // And BOARD-007 was moved to done at 09:00
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-005',
          title: 'Feature 5',
          type: 'story',
          status: 'done',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-27T10:00:00Z' }, // 10:00
          ],
        },
        {
          id: 'BOARD-003',
          title: 'Feature 3',
          type: 'story',
          status: 'done',
          estimate: 5,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-27T11:00:00Z' }, // 11:00 (most recent)
          ],
        },
        {
          id: 'BOARD-007',
          title: 'Feature 7',
          type: 'story',
          status: 'done',
          estimate: 2,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-27T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-27T09:00:00Z' }, // 09:00 (oldest)
          ],
        },
      ];

      // When I view the done column
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={5} // done column
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = lastFrame() || '';

      // Then the work units should be displayed in order: BOARD-003, BOARD-005, BOARD-007
      // Extract all lines from the output
      const lines = output.split('\n');

      // Find the DONE column index by finding the header row
      const headerRow = lines.find(line => line.includes('DONE'));
      expect(headerRow).toBeTruthy();

      // Find done column boundaries (column starts after | and ends before |)
      const doneStart = headerRow!.indexOf('DONE');
      const doneEnd = headerRow!.indexOf('│BLOCKED') || headerRow!.length;

      // Extract done column content from all rows
      const doneColumnLines = lines
        .slice(lines.indexOf(headerRow!) + 2) // Skip header and separator
        .filter(line => line.includes('│') && !line.includes('├') && !line.includes('└'))
        .map(line => line.substring(doneStart, doneEnd).trim())
        .filter(line => line.length > 0 && line !== '│');

      const doneColumnContent = doneColumnLines.join('\n');

      const board003Index = doneColumnContent.indexOf('BOARD-003');
      const board005Index = doneColumnContent.indexOf('BOARD-005');
      const board007Index = doneColumnContent.indexOf('BOARD-007');

      // And BOARD-003 should be at the top (most recent completion)
      expect(board003Index).toBeLessThan(board005Index);
      expect(board005Index).toBeLessThan(board007Index);
      // And BOARD-007 should be at the bottom (oldest completion)
    });
  });

  describe('Scenario: More recently completed work units appear above older ones', () => {
    it('should show Tuesday completion above Monday completion', () => {
      // Given BOARD-001 was moved to done on Monday
      // And BOARD-002 was moved to done on Tuesday
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-001',
          title: 'Monday work',
          type: 'story',
          status: 'done',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-20T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-21T10:00:00Z' }, // Monday
          ],
        },
        {
          id: 'BOARD-002',
          title: 'Tuesday work',
          type: 'story',
          status: 'done',
          estimate: 5,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-20T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-22T10:00:00Z' }, // Tuesday (more recent)
          ],
        },
      ];

      // When I view the done column
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={5}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = lastFrame() || '';

      // Then BOARD-002 should appear above BOARD-001
      const lines = output.split('\n');
      const headerRow = lines.find(line => line.includes('DONE'));
      expect(headerRow).toBeTruthy();

      const doneStart = headerRow!.indexOf('DONE');
      const doneEnd = headerRow!.indexOf('│BLOCKED') || headerRow!.length;

      const doneColumnLines = lines
        .slice(lines.indexOf(headerRow!) + 2)
        .filter(line => line.includes('│') && !line.includes('├') && !line.includes('└'))
        .map(line => line.substring(doneStart, doneEnd).trim())
        .filter(line => line.length > 0 && line !== '│');

      const doneColumnContent = doneColumnLines.join('\n');

      const board001Index = doneColumnContent.indexOf('BOARD-001');
      const board002Index = doneColumnContent.indexOf('BOARD-002');

      expect(board002Index).toBeLessThan(board001Index);
      // And the order should reflect completion chronology (newest first)
    });
  });

  describe('Scenario: Re-completing a work unit updates its position to top of done column', () => {
    it('should move re-completed work unit to top with new timestamp', () => {
      // Given BOARD-003 was completed yesterday
      // And BOARD-003 is currently in the done column
      // When user moves BOARD-003 back to implementing and then back to done today
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-001',
          title: 'Other work',
          type: 'story',
          status: 'done',
          estimate: 3,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-26T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-26T12:00:00Z' },
          ],
        },
        {
          id: 'BOARD-003',
          title: 'Re-completed work',
          type: 'story',
          status: 'done',
          estimate: 5,
          stateHistory: [
            { state: 'backlog', timestamp: '2025-10-26T08:00:00Z' },
            { state: 'done', timestamp: '2025-10-26T10:00:00Z' }, // Yesterday (old)
            { state: 'implementing', timestamp: '2025-10-27T09:00:00Z' },
            { state: 'done', timestamp: '2025-10-27T15:00:00Z' }, // Today (new - most recent)
          ],
        },
      ];

      // When I view the done column
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={5}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = lastFrame() || '';

      // Then BOARD-003 should appear at the top of the done column
      const lines = output.split('\n');
      const headerRow = lines.find(line => line.includes('DONE'));
      expect(headerRow).toBeTruthy();

      const doneStart = headerRow!.indexOf('DONE');
      const doneEnd = headerRow!.indexOf('│BLOCKED') || headerRow!.length;

      const doneColumnLines = lines
        .slice(lines.indexOf(headerRow!) + 2)
        .filter(line => line.includes('│') && !line.includes('├') && !line.includes('└'))
        .map(line => line.substring(doneStart, doneEnd).trim())
        .filter(line => line.length > 0 && line !== '│');

      const doneColumnContent = doneColumnLines.join('\n');

      const board001Index = doneColumnContent.indexOf('BOARD-001');
      const board003Index = doneColumnContent.indexOf('BOARD-003');

      // And the new timestamp should be used for sorting (not the old one)
      expect(board003Index).toBeLessThan(board001Index);
    });
  });

  describe('Scenario: Other columns maintain existing order behavior', () => {
    it('should only sort done column, leaving other columns unchanged', () => {
      // Given the backlog column has work units in a specific order
      // And the implementing column has work units in a specific order
      const workUnits: WorkUnit[] = [
        // Backlog - should maintain array order
        { id: 'BACK-002', title: 'B2', type: 'story', status: 'backlog', estimate: 3 },
        { id: 'BACK-001', title: 'B1', type: 'story', status: 'backlog', estimate: 2 },
        // Implementing - should maintain array order
        { id: 'IMPL-003', title: 'I3', type: 'story', status: 'implementing', estimate: 5 },
        { id: 'IMPL-001', title: 'I1', type: 'story', status: 'implementing', estimate: 3 },
        // Done - should be sorted by completion time
        {
          id: 'DONE-002',
          title: 'D2',
          type: 'story',
          status: 'done',
          estimate: 3,
          stateHistory: [{ state: 'done', timestamp: '2025-10-27T11:00:00Z' }],
        },
        {
          id: 'DONE-001',
          title: 'D1',
          type: 'story',
          status: 'done',
          estimate: 2,
          stateHistory: [{ state: 'done', timestamp: '2025-10-27T10:00:00Z' }],
        },
      ];

      // When I view the board
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = lastFrame() || '';

      const lines = output.split('\n');
      const headerRow = lines.find(line => line.includes('BACKLOG'));
      expect(headerRow).toBeTruthy();

      // Extract column boundaries
      const backlogStart = headerRow!.indexOf('BACKLOG');
      const backlogEnd = headerRow!.indexOf('│SPECIFYING');
      const implementingStart = headerRow!.indexOf('IMPLEMENTING');
      const implementingEnd = headerRow!.indexOf('│VALIDATING');
      const doneStart = headerRow!.indexOf('DONE');
      const doneEnd = headerRow!.indexOf('│BLOCKED');

      const dataLines = lines
        .slice(lines.indexOf(headerRow!) + 2)
        .filter(line => line.includes('│') && !line.includes('├') && !line.includes('└'));

      // Extract backlog column
      const backlogLines = dataLines
        .map(line => line.substring(backlogStart, backlogEnd).trim())
        .filter(line => line.length > 0 && line !== '│');
      const backlogContent = backlogLines.join('\n');

      // Extract implementing column
      const implementingLines = dataLines
        .map(line => line.substring(implementingStart, implementingEnd).trim())
        .filter(line => line.length > 0 && line !== '│');
      const implementingContent = implementingLines.join('\n');

      // Extract done column
      const doneLines = dataLines
        .map(line => line.substring(doneStart, doneEnd).trim())
        .filter(line => line.length > 0 && line !== '│');
      const doneContent = doneLines.join('\n');

      // Then the backlog column order should remain unchanged
      expect(backlogContent.indexOf('BACK-002')).toBeLessThan(backlogContent.indexOf('BACK-001'));

      // And the implementing column order should remain unchanged
      expect(implementingContent.indexOf('IMPL-003')).toBeLessThan(implementingContent.indexOf('IMPL-001'));

      // And only the done column should be sorted by completion time
      expect(doneContent.indexOf('DONE-002')).toBeLessThan(doneContent.indexOf('DONE-001'));
    });
  });
});

/**
 * Feature: spec/features/work-unit-details-panel-shows-incorrect-work-unit-after-reordering.feature
 *
 * Tests that done column displays work units in EXACT file order without runtime sorting.
 * Maps to BOARD-016 scenarios.
 */

import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';
import type { WorkUnit } from '../components/UnifiedBoardLayout';

describe('Feature: Work unit details panel shows incorrect work unit after reordering', () => {
  describe('Scenario: TUI displays done column in exact file array order', () => {
    it('should display work units in exact array order WITHOUT runtime sorting', () => {
      // Given the work units array has done items in this specific order: BOARD-003, BOARD-005, BOARD-001
      // (This represents the file order from work-units.json states.done array)
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-003',
          title: 'Feature 3',
          type: 'story',
          status: 'done',
          estimate: 5,
          updated: '2025-10-27T11:00:00Z', // 11:00
          stateHistory: [
            { state: 'done', timestamp: '2025-10-27T11:00:00Z' },
          ],
        },
        {
          id: 'BOARD-005',
          title: 'Feature 5',
          type: 'story',
          status: 'done',
          estimate: 3,
          updated: '2025-10-27T12:00:00Z', // 12:00 (most recent, but in middle of array)
          stateHistory: [
            { state: 'done', timestamp: '2025-10-27T12:00:00Z' },
          ],
        },
        {
          id: 'BOARD-001',
          title: 'Feature 1',
          type: 'story',
          status: 'done',
          estimate: 2,
          updated: '2025-10-27T09:00:00Z', // 09:00 (oldest)
          stateHistory: [
            { state: 'done', timestamp: '2025-10-27T09:00:00Z' },
          ],
        },
      ];

      // When I view the done column
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={5} // done column
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then the work units should be displayed in EXACT ARRAY ORDER: BOARD-003, BOARD-005, BOARD-001
      // NOT sorted by timestamp (which would be BOARD-005, BOARD-003, BOARD-001)
      const lines = output.split('\n');

      // Find the DONE column index by finding the header row
      const headerRow = lines.find(line => line.includes('DONE'));
      expect(headerRow).toBeTruthy();

      // Find done column boundaries
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
      const board001Index = doneColumnContent.indexOf('BOARD-001');

      // Verify EXACT file order is preserved (not sorted by timestamp)
      expect(board003Index).toBeLessThan(board005Index);
      expect(board005Index).toBeLessThan(board001Index);

      // If runtime sorting was happening, BOARD-005 (12:00 - most recent) would be first
      // But we WANT file order: BOARD-003 first, BOARD-005 second, BOARD-001 last
    });
  });

  describe('Scenario: File order matches display order exactly', () => {
    it('should display work units in exact array order regardless of timestamps', () => {
      // Given the work units array has a specific order that does NOT match timestamp order
      const workUnits: WorkUnit[] = [
        {
          id: 'BOARD-001',
          title: 'Oldest',
          type: 'story',
          status: 'done',
          estimate: 2,
          updated: '2025-10-27T09:00:00Z', // Oldest timestamp
        },
        {
          id: 'BOARD-003',
          title: 'Newest',
          type: 'story',
          status: 'done',
          estimate: 5,
          updated: '2025-10-27T12:00:00Z', // Newest timestamp
        },
        {
          id: 'BOARD-002',
          title: 'Middle',
          type: 'story',
          status: 'done',
          estimate: 3,
          updated: '2025-10-27T10:00:00Z', // Middle timestamp
        },
      ];

      // When I view the done column
      const { frames } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={5}
          selectedWorkUnitIndex={0}
          selectedWorkUnit={null}
        />
      );

      const output = frames[frames.length - 1] || '';

      // Then the work units should display in EXACT array order: BOARD-001, BOARD-003, BOARD-002
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
      const board002Index = doneColumnContent.indexOf('BOARD-002');

      // Verify array order is preserved
      expect(board001Index).toBeLessThan(board003Index);
      expect(board003Index).toBeLessThan(board002Index);

      // This proves NO runtime sorting is happening
    });
  });
});

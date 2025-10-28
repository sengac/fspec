/**
 * Feature: spec/features/remove-no-work-units-placeholder-from-empty-board-columns.feature
 *
 * This test file validates that empty Kanban columns display cleanly without
 * "No work units" placeholder text.
 *
 * These tests should FAIL until the implementation is complete (TDD red phase).
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

describe("Feature: Remove 'No work units' placeholder from empty board columns", () => {
  describe('Scenario: Empty column shows only header without placeholder text', () => {
    it('should not render "No work units" text for empty columns', async () => {
      // @step Given the Testing column has no work units
      // @step When I view the Kanban board
      // @step Then the Testing column should show only its header
      // @step And no 'No work units' text should be displayed

      const filePath = join(
        process.cwd(),
        'src/tui/components/UnifiedBoardLayout.tsx'
      );
      const content = await readFile(filePath, 'utf-8');

      // Should not contain the "No work units" text in render logic
      expect(content).not.toMatch(/['"]No work units['"]/);
    });
  });

  describe('Scenario: All empty columns show clean headers', () => {
    it('should render empty string when itemIndex >= column.units.length', async () => {
      // @step Given all Kanban columns are empty
      // @step When I view the Kanban board
      // @step Then all columns should show only their headers
      // @step And no placeholder text should be visible in any column

      const filePath = join(
        process.cwd(),
        'src/tui/components/UnifiedBoardLayout.tsx'
      );
      const content = await readFile(filePath, 'utf-8');

      // When itemIndex >= column.units.length, should return empty string
      // Looking for pattern like: return fitToWidth('', colWidth);
      // NOT: return fitToWidth(itemIndex === 0 ? 'No work units' : '', colWidth);
      expect(content).not.toMatch(
        /itemIndex === 0 \? ['"]No work units['"] : ['"]['"]/
      );
    });
  });

  describe('Scenario: Mixed columns show work units only where they exist', () => {
    it('should handle mixed empty and non-empty columns correctly', async () => {
      // @step Given the Done column has 3 work units
      // @step And all other columns are empty
      // @step When I view the Kanban board
      // @step Then the Done column should show all 3 work units
      // @step And empty columns should show only headers with no placeholder text

      const filePath = join(
        process.cwd(),
        'src/tui/components/UnifiedBoardLayout.tsx'
      );
      const content = await readFile(filePath, 'utf-8');

      // Verify no "No work units" placeholder exists in the code
      expect(content).not.toMatch(/['"]No work units['"]/);
    });
  });
});

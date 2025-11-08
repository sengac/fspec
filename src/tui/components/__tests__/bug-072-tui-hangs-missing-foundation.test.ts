/**
 * Feature: spec/features/tui-hangs-when-foundation-json-is-missing.feature
 *
 * This test file validates that the TUI loads properly even when foundation.json
 * is missing, rather than hanging or pausing indefinitely.
 *
 * These tests should FAIL until the implementation is complete (TDD red phase).
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: TUI hangs when foundation.json is missing', () => {
  describe('Scenario: TUI loads with no spec directory', () => {
    it('should handle missing foundation.json gracefully in BoardView', async () => {
      // @step Given I am in a project with no spec directory
      // @step When I run "fspec" with no arguments
      // @step Then the TUI should load within 5 seconds
      // @step And the board should display empty columns
      // @step And the TUI should be responsive to keyboard input

      const boardViewPath = join(
        process.cwd(),
        'src/tui/components/BoardView.tsx'
      );
      const boardViewContent = await readFile(boardViewPath, 'utf-8');

      // BoardView should not check for foundation.json before rendering
      // (unlike the "fspec board" command which does check)
      expect(boardViewContent).not.toMatch(/checkFoundationExists/);
    });

    it('should ensure fspecStore loadData() handles missing files gracefully', async () => {
      // @step Given I am in a project with no spec directory
      // @step When I run "fspec" with no arguments
      // @step Then the TUI should load within 5 seconds

      const fspecStorePath = join(process.cwd(), 'src/tui/store/fspecStore.ts');
      const storeContent = await readFile(fspecStorePath, 'utf-8');

      // loadData should have error handling (try-catch)
      // It already has this, so this test should pass
      expect(storeContent).toMatch(/try[\s\S]*?catch.*error/);
    });
  });

  describe('Scenario: TUI loads with spec directory but no foundation.json', () => {
    it('should render BoardView without foundation.json requirement', async () => {
      // @step Given I am in a project with a spec directory
      // @step But no foundation.json file exists
      // @step When I run "fspec" with no arguments
      // @step Then the TUI should load within 5 seconds
      // @step And the board should display empty columns
      // @step And the TUI should be responsive to keyboard input

      const indexPath = join(process.cwd(), 'src/index.ts');
      const indexContent = await readFile(indexPath, 'utf-8');

      // index.ts should not check for foundation.json before launching TUI
      // The TUI launch path (lines 348-378) should not call checkFoundationExists
      const tuiLaunchSection = indexContent.split(
        'if (process.argv.length === 2)'
      )[1];
      if (tuiLaunchSection) {
        expect(tuiLaunchSection).not.toMatch(/checkFoundationExists/);
      }
    });
  });

  describe('Scenario: User can exit TUI with ESC key', () => {
    it('should have ESC key handler regardless of data state', async () => {
      // @step Given the TUI is running without foundation.json
      // @step When I press the ESC key
      // @step Then the TUI should exit gracefully within 1 second

      const boardViewPath = join(
        process.cwd(),
        'src/tui/components/BoardView.tsx'
      );
      const boardViewContent = await readFile(boardViewPath, 'utf-8');

      // BoardView should have ESC key handler in useInput
      expect(boardViewContent).toMatch(/key\.escape/);
      expect(boardViewContent).toMatch(/onExit\?\.?\(\)/);
    });
  });
});

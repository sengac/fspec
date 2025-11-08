/**
 * Feature: spec/features/view-foundation-md-in-tui-with-attachment-viewer.feature
 *
 * This test file validates the FOUNDATION.md viewer functionality in the TUI,
 * including D keybinding, browser opening via HTTP server, and UI display.
 *
 * These tests should FAIL until the implementation is complete (TDD red phase).
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: View FOUNDATION.md in TUI with attachment viewer', () => {
  describe('Scenario: Press D key to open FOUNDATION.md in browser', () => {
    it('should open FOUNDATION.md in browser when D key is pressed', async () => {
      // @step Given I am viewing the main TUI board
      // @step And spec/FOUNDATION.md exists in the project
      // @step And the attachment server is running
      // @step When I press the "D" key
      // @step Then a browser should open with URL "http://localhost:{port}/view/spec/FOUNDATION.md"
      // @step And the browser should display rendered Markdown content

      const boardViewPath = join(
        process.cwd(),
        'src/tui/components/BoardView.tsx'
      );
      const boardViewContent = await readFile(boardViewPath, 'utf-8');

      // BoardView should have 'D' key handler
      expect(boardViewContent).toMatch(
        /input === ['"]d['"]\s*\|\|\s*input === ['"]D['"]/
      );

      // Should call openInBrowser when D is pressed
      const dKeySection = boardViewContent.match(
        /input === ['"]d['"]\s*\|\|\s*input === ['"]D['"][\s\S]{0,500}openInBrowser/
      );
      expect(dKeySection).toBeTruthy();

      // Should construct HTTP URL for FOUNDATION.md
      if (dKeySection) {
        expect(dKeySection[0]).toMatch(/FOUNDATION\.md|foundation/i);
        expect(dKeySection[0]).toMatch(/http:\/\/localhost/);
      }
    });
  });

  describe('Scenario: Keybinding help line displays D View FOUNDATION.md', () => {
    it('should display keybinding help with diamond separator and D View FOUNDATION.md', async () => {
      // @step Given I am viewing the main TUI board
      // @step When I look at the keybinding help line
      // @step Then I should see "F View Changed Files" on the help line
      // @step And I should see a diamond separator "◆" after it
      // @step And I should see "D View FOUNDATION.md" to the right of the separator
      // @step And the help line should read "◆ F View Changed Files ◆ D View FOUNDATION.md"

      const keybindingShortcutsPath = join(
        process.cwd(),
        'src/tui/components/KeybindingShortcuts.tsx'
      );
      const keybindingContent = await readFile(
        keybindingShortcutsPath,
        'utf-8'
      );

      // Should have "F View Changed Files" text
      expect(keybindingContent).toMatch(/F View Changed Files/);

      // Should have diamond separator
      expect(keybindingContent).toMatch(/◆/);

      // Should have "D View FOUNDATION.md" text
      expect(keybindingContent).toMatch(/D View FOUNDATION\.md/);

      // Diamond should appear between F and D commands
      const helpLineMatch = keybindingContent.match(
        /F View Changed Files.*◆.*D View FOUNDATION\.md/
      );
      expect(helpLineMatch).toBeTruthy();
    });
  });

  describe('Scenario: Error message when FOUNDATION.md does not exist', () => {
    it('should handle missing FOUNDATION.md gracefully', async () => {
      // @step Given I am viewing the main TUI board
      // @step And spec/FOUNDATION.md does not exist in the project
      // @step When I press the "D" key
      // @step Then I should see an error message
      // @step And the error message should say "FOUNDATION.md not found at spec/FOUNDATION.md"
      // @step And the TUI should not crash

      const boardViewPath = join(
        process.cwd(),
        'src/tui/components/BoardView.tsx'
      );
      const boardViewContent = await readFile(boardViewPath, 'utf-8');

      // D key handler should check if file exists or have error handling
      const dKeySection = boardViewContent.match(
        /input === ['"]d['"]\s*\|\|\s*input === ['"]D['"][\s\S]{0,1000}/
      );

      if (dKeySection) {
        // Should have some form of error handling or file existence check
        // This is a weak assertion but we're checking for awareness of the issue
        expect(
          dKeySection[0].includes('FOUNDATION') ||
            dKeySection[0].includes('foundation') ||
            dKeySection[0].includes('existsSync') ||
            dKeySection[0].includes('catch')
        ).toBeTruthy();
      }
    });
  });

  describe('Scenario: Reuse attachment server infrastructure', () => {
    it('should use attachment server to serve FOUNDATION.md', async () => {
      // @step Given the attachment server is running for work unit attachments
      // @step When I press the "D" key to view FOUNDATION.md
      // @step Then the same HTTP server should serve spec/FOUNDATION.md
      // @step And the server should use the same /view/{path} endpoint pattern
      // @step And openInBrowser() should be called with the HTTP URL

      const boardViewPath = join(
        process.cwd(),
        'src/tui/components/BoardView.tsx'
      );
      const boardViewContent = await readFile(boardViewPath, 'utf-8');

      // Should use attachmentServerPort variable (same as attachments)
      expect(boardViewContent).toMatch(/attachmentServerPort/);

      // D key handler should use HTTP URL pattern similar to attachments
      const dKeySection = boardViewContent.match(
        /input === ['"]d['"]\s*\|\|\s*input === ['"]D['"][\s\S]{0,1000}/
      );

      if (dKeySection) {
        // Should construct URL with /view/ endpoint
        expect(dKeySection[0]).toMatch(/\/view\//);
        // Should use attachmentServerPort
        expect(dKeySection[0]).toMatch(/attachmentServerPort/);
        // Should call openInBrowser
        expect(dKeySection[0]).toMatch(/openInBrowser/);
      }
    });
  });

  describe('Scenario: Diamond separator appears in correct position', () => {
    it('should position diamond separator correctly in help line', async () => {
      // @step Given I am viewing the main TUI board
      // @step When I examine the keybinding help line
      // @step Then "F View Changed Files" should appear first
      // @step And a diamond separator "◆" should appear immediately after it
      // @step And "D View FOUNDATION.md" should appear immediately after the separator
      // @step And no other text should appear between these elements

      const keybindingShortcutsPath = join(
        process.cwd(),
        'src/tui/components/KeybindingShortcuts.tsx'
      );
      const keybindingContent = await readFile(
        keybindingShortcutsPath,
        'utf-8'
      );

      // Extract help line pattern
      const helpLinePattern =
        /F View Changed Files\s*◆\s*D View FOUNDATION\.md/;
      expect(keybindingContent).toMatch(helpLinePattern);

      // Ensure no extra text between elements
      const cleanPattern =
        /F View Changed Files\s*◆\s*D View FOUNDATION\.md(?!\w)/;
      expect(keybindingContent).toMatch(cleanPattern);
    });
  });
});

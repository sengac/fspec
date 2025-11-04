/**
 * Feature: spec/features/open-attachments-in-browser-from-details-panel.feature
 *
 * Tests for TUI-013: Open attachments in browser from details panel
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { openInBrowser } from '../../utils/openBrowser';
import open from 'open';

// Mock the 'open' package that openInBrowser uses
vi.mock('open', () => ({
  default: vi.fn().mockResolvedValue(undefined),
}));

describe('Feature: Open attachments in browser from details panel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Open image attachment in browser', () => {
    it('should open image attachment with file:// protocol and keep TUI active', async () => {
      // @step Given I am viewing work unit details with an attachment 'diagram.png'
      const fileUrl = 'file:///home/rquast/projects/fspec/spec/attachments/TUI-012/diagram.png';

      // @step When I select the attachment and press 'o'
      // This triggers openInBrowser function
      await openInBrowser({ url: fileUrl, wait: false });

      // @step Then the default browser should open the file with file:// protocol
      // Verify that the 'open' package was called with file:// URL
      expect(open).toHaveBeenCalled();

      const callArgs = (open as any).mock.calls[0];
      expect(callArgs[0]).toMatch(/^file:\/\//);
      expect(callArgs[0]).toContain('diagram.png');

      // @step And the TUI should remain active
      // Verify wait: false option was used
      expect(callArgs[1]).toEqual({ wait: false });
    });
  });

  describe('Scenario: Open PDF attachment in default viewer', () => {
    it('should open PDF attachment in default viewer', async () => {
      // @step Given I am viewing work unit details with a PDF attachment 'requirements.pdf'
      const fileUrl = 'file:///home/rquast/projects/fspec/spec/attachments/AUTH-001/requirements.pdf';

      // @step When I select the attachment and press 'o'
      await openInBrowser({ url: fileUrl, wait: false });

      // @step Then the PDF should open in the default PDF viewer/browser
      expect(open).toHaveBeenCalled();

      const callArgs = (open as any).mock.calls[0];
      expect(callArgs[0]).toMatch(/^file:\/\//);
      expect(callArgs[0]).toContain('requirements.pdf');
    });
  });

  describe('Scenario: Handle missing attachment file gracefully', () => {
    it('should handle errors when attachment file does not exist', async () => {
      // @step Given I am viewing work unit details with an attachment that doesn't exist on disk
      const fileUrl = 'file:///home/rquast/projects/fspec/spec/attachments/MISSING/nonexistent.pdf';

      // Mock 'open' to throw an error (file doesn't exist)
      (open as any).mockRejectedValueOnce(new Error('File not found'));

      // @step When I select the attachment and press 'o'
      // @step Then the TUI should show an error message
      // @step And the browser should not open
      // The 'open' call should have been attempted but failed
      await expect(openInBrowser({ url: fileUrl, wait: false })).rejects.toThrow('File not found');

      expect(open).toHaveBeenCalled();
    });
  });
});

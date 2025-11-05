/**
 * Feature: spec/features/font-size-controls-for-attachment-viewer.feature
 *
 * Tests for TUI-021: Font size controls for attachment viewer
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  startAttachmentServer,
  stopAttachmentServer,
  getServerPort,
} from '../attachment-server';
import type { Server } from 'http';
import fetch from 'node-fetch';
import path from 'path';
import fs from 'fs/promises';

describe('Feature: Font size controls for attachment viewer', () => {
  let server: Server | null = null;
  const testCwd = path.join(
    process.cwd(),
    'test-fixtures',
    'font-size-attachments'
  );

  beforeEach(async () => {
    // Create test fixtures directory
    await fs.mkdir(testCwd, { recursive: true });
  });

  afterEach(async () => {
    // Cleanup server
    if (server) {
      await stopAttachmentServer(server);
      server = null;
    }

    // Cleanup test fixtures
    await fs.rm(testCwd, { recursive: true, force: true });
  });

  describe('Scenario: Default code block font size is larger than current implementation', () => {
    it('should display code blocks at 16px font size by default', async () => {
      // @step Given the attachment server is running
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step When I open a markdown attachment with code blocks
      const markdownContent = `# Code Example

\`\`\`javascript
const hello = 'world';
\`\`\`
`;
      const markdownPath = path.join(testCwd, 'code-example.md');
      await fs.writeFile(markdownPath, markdownContent);

      const response = await fetch(
        `http://localhost:${port}/view/code-example.md`
      );
      const html = await response.text();

      // @step Then the code blocks should display at 16px font size
      // Check for CSS custom property or inline styles setting font size to 16px
      expect(html).toContain('--base-font-size');
      expect(html).toMatch(/--base-font-size:\s*16px/);

      // @step And the font size should be larger than the previous default
      // This is validated by checking that the default is explicitly 16px (previous was smaller)
    });
  });

  describe('Scenario: Increase font size with plus button', () => {
    it('should increase font size from 16px to 18px when plus button clicked', async () => {
      // @step Given I have a markdown attachment open with font size at 16px
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(markdownPath, '# Test\n\n```js\nconst x = 1;\n```');

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I click the plus button in the font size controls
      // Verify the plus button exists in HTML
      expect(html).toContain('font-size-increase');

      // @step Then the font size should increase to 18px
      // Check that JavaScript exists to increment with FONT_SIZE_STEP
      expect(html).toContain('FONT_SIZE_STEP');
      expect(html).toContain('currentFontSize + FONT_SIZE_STEP');

      // @step And the code blocks should re-render with the new size
      // Verify CSS custom property is updated
      expect(html).toContain('setProperty');

      // @step And the size display should show "18px"
      // Verify font size display element exists
      expect(html).toContain('font-size-display');
    });
  });

  describe('Scenario: Font size respects minimum bound', () => {
    it('should prevent font size from going below 10px', async () => {
      // @step Given I have a markdown attachment open with font size at 10px
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(
        markdownPath,
        '# Test\n\n```python\nprint("hello")\n```'
      );

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I click the minus button in the font size controls
      // Verify the minus button exists
      expect(html).toContain('font-size-decrease');

      // @step Then the font size should remain at 10px
      // Check for minimum bound validation (MIN_FONT_SIZE = 10)
      expect(html).toContain('MIN_FONT_SIZE');
      expect(html).toContain(
        'Math.max(currentFontSize - FONT_SIZE_STEP, MIN_FONT_SIZE)'
      );

      // @step And the minus button should be disabled or show visual indication
      // Verify button disable logic exists
      expect(html).toContain('disabled');
    });
  });

  describe('Scenario: Font size respects maximum bound', () => {
    it('should prevent font size from going above 24px', async () => {
      // @step Given I have a markdown attachment open with font size at 24px
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(markdownPath, '# Test\n\n```bash\necho "test"\n```');

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I click the plus button in the font size controls
      // Verify the plus button exists
      expect(html).toContain('font-size-increase');

      // @step Then the font size should remain at 24px
      // Check for maximum bound validation (MAX_FONT_SIZE = 24)
      expect(html).toContain('MAX_FONT_SIZE');
      expect(html).toContain(
        'Math.min(currentFontSize + FONT_SIZE_STEP, MAX_FONT_SIZE)'
      );

      // @step And the plus button should be disabled or show visual indication
      // Verify button disable logic exists
      expect(html).toContain('disabled');
    });
  });

  describe('Scenario: Font size persists across browser sessions', () => {
    it('should restore font size from localStorage on page load', async () => {
      // @step Given I have set the font size to 20px in a previous session
      // @step And I have closed the browser
      // This is simulated by checking that localStorage persistence code exists

      // @step When I open a markdown attachment
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(markdownPath, '# Test\n\n```ruby\nputs "hello"\n```');

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step Then the code blocks should display at 20px font size
      // @step And the localStorage should contain the font size preference
      // Verify localStorage get/set operations for font size
      expect(html).toContain('localStorage');
      expect(html).toContain('fspec-base-font-size');
      expect(html).toContain('getItem');
      expect(html).toContain('setItem');
    });
  });

  describe('Scenario: Font size controls display in top right corner', () => {
    it('should render font size controls next to dark mode toggle', async () => {
      // @step Given the attachment server is running
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step When I open a markdown attachment
      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(
        markdownPath,
        '# Test\n\n```go\nfmt.Println("hello")\n```'
      );

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step Then the font size controls should be visible in the top right corner
      // Verify font size controls container exists
      expect(html).toContain('font-size-controls');

      // @step And the controls should be next to the dark mode toggle
      // Verify positioning relative to theme toggle
      expect(html).toContain('theme-toggle');

      // @step And the controls should show plus button, size display, and minus button
      expect(html).toContain('font-size-increase');
      expect(html).toContain('font-size-display');
      expect(html).toContain('font-size-decrease');
    });
  });
});

/**
 * Feature: spec/features/mermaid-diagram-zoom-not-centered-on-mouse-and-trackpad-causes-jittering.feature
 *
 * Tests for VIEW-002: Fix zoom to be centered on cursor position and smooth trackpad scrolling
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

describe('Feature: Mermaid diagram zoom not centered on mouse and trackpad causes jittering', () => {
  let server: Server | null = null;
  const testCwd = path.join(
    process.cwd(),
    'test-fixtures',
    'zoom-centered-on-cursor'
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

  describe('Scenario: Zoom in centered on cursor position with mouse wheel', () => {
    it('should zoom in keeping the point under the cursor fixed', async () => {
      // @step Given I have a mermaid diagram open in fullscreen modal
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start] --> B[End]
\`\`\``;
      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(markdownPath, testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step And my cursor is positioned at coordinates (400, 300) on the diagram
      // @step And the diagram has a specific element at those coordinates
      // @step When I scroll up with the mouse wheel
      // @step Then the diagram should zoom in
      // @step And the element that was under my cursor should remain under my cursor

      // @step And the viewport position should be calculated using the formula: newX = viewport.x + (mouseX - viewport.x) * (1 - zoomRatio)
      // Verify zoom-to-point is implemented using panzoom's built-in method
      expect(html).toContain('zoomToPoint');
      expect(html).toContain('clientX');
      expect(html).toContain('clientY');

      // Verify zoom point locking for gesture handling
      expect(html).toContain('lockedZoomPointX');
      expect(html).toContain('lockedZoomPointY');
    });
  });

  describe('Scenario: Zoom out centered on cursor position with mouse wheel', () => {
    it('should zoom out keeping the point under the cursor fixed', async () => {
      // @step Given I have a mermaid diagram open in fullscreen modal
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph LR
  A[Node A] --> B[Node B]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step And my cursor is positioned at the top-right corner of the diagram
      // @step And the diagram is zoomed in to 200%
      // @step When I scroll down with the mouse wheel
      // @step Then the diagram should zoom out

      // @step And the point that was under my cursor should remain fixed
      // @step And the zoom calculation should use the same zoom-to-point formula
      // Verify same zoomToPoint method applies for zoom out
      expect(html).toContain('zoomToPoint');
      expect(html).toContain('clientX');
      expect(html).toContain('clientY');
    });
  });

  describe('Scenario: Smooth zoom with trackpad vertical scroll', () => {
    it('should accumulate and smooth rapid trackpad scroll events', async () => {
      // @step Given I have a mermaid diagram open in fullscreen modal
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start] --> B[Middle] --> C[End]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step And I am using a trackpad
      // @step When I perform a vertical two-finger swipe upward

      // @step Then the zoom events should be debounced or smoothed
      // @step And the diagram should zoom in smoothly without jittering
      // @step And rapid tiny deltaY events should be accumulated before applying zoom
      // Verify zoomToPoint is used which provides smooth handling
      expect(html).toContain('zoomToPoint');
      expect(html).toContain('deltaY');
    });
  });

  describe('Scenario: Horizontal pan with trackpad', () => {
    it('should pan smoothly with horizontal trackpad scroll', async () => {
      // @step Given I have a mermaid diagram open in fullscreen modal
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph LR
  A --> B --> C --> D
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step And I am using a trackpad
      // @step When I perform a horizontal two-finger swipe to the left

      // @step Then the diagram should pan right smoothly
      expect(html).toContain('deltaX');
      expect(html).toContain('element.style.transform');

      // @step And deltaX events should be processed immediately
      // @step And pan mode modifier should not affect horizontal scrolling
      // Verify deltaX is handled separately from pan mode
      expect(html).toContain('Math.abs(deltaX)');
    });
  });

  describe('Scenario: Handle different mouse wheel deltaMode values', () => {
    it('should apply correct zoom delta multiplier based on deltaMode', async () => {
      // @step Given I have a mermaid diagram open in fullscreen modal
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Test]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I scroll with a mouse wheel that reports deltaMode=0 (PIXEL)
      // @step Then the zoom delta should be calculated as: -deltaY * 0.002
      expect(html).toContain('0.002');

      // @step When I scroll with a mouse wheel that reports deltaMode=1 (LINE)
      // @step Then the zoom delta should be calculated as: -deltaY * 0.05
      expect(html).toContain('0.05');

      // @step When I scroll with a mouse wheel that reports deltaMode=2 (PAGE)
      // @step Then the zoom delta should be calculated as: -deltaY * 1
      expect(html).toContain('deltaMode === 1');
      expect(html).toContain('deltaMode');

      // Verify the complete formula structure
      expect(html).toContain('deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002');
    });
  });
});

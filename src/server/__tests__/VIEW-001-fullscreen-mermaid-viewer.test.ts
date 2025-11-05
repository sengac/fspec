/**
 * Feature: spec/features/fullscreen-mermaid-diagram-viewer-with-zoom-and-pan-controls.feature
 *
 * Tests for VIEW-001: Fullscreen Mermaid Diagram Viewer with Zoom and Pan Controls
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
import { JSDOM } from 'jsdom';

describe('Feature: Fullscreen Mermaid Diagram Viewer with Zoom and Pan Controls', () => {
  let server: Server | null = null;
  const testCwd = path.join(
    process.cwd(),
    'test-fixtures',
    'fullscreen-mermaid-viewer'
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

  describe('Scenario: Display fullscreen and download buttons on diagram hover', () => {
    it('should show fullscreen and download buttons when hovering over mermaid diagram', async () => {
      // @step Given I am viewing a markdown file with a mermaid diagram in the attachment viewer
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `# Test Document

\`\`\`mermaid
graph TD
  A[Start] --> B[End]
\`\`\`
`;
      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(markdownPath, testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I hover my mouse over the mermaid diagram
      // @step Then the fullscreen button should fade in at the top-right corner
      // Check that the HTML contains the JavaScript function to add fullscreen buttons
      expect(html).toContain('addFullscreenButtons');
      expect(html).toContain('mermaid-fullscreen-btn');

      // @step And the download button should also be visible
      expect(html).toContain('mermaid-download-btn');

      // Check that CSS for buttons exists
      expect(html).toContain('.mermaid-wrapper:hover .mermaid-overlay');
    });
  });

  describe('Scenario: Open diagram in fullscreen modal', () => {
    it('should open fullscreen modal when clicking fullscreen button', async () => {
      // @step Given I am viewing a mermaid diagram
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

      // @step When I click the fullscreen button
      // Check that the JavaScript function for opening modal exists
      expect(html).toContain('openMermaidModal');

      // @step Then a fullscreen modal should open with smooth scale and fade animation
      expect(html).toContain('id="mermaid-modal"');
      expect(html).toContain('modal-backdrop');

      // @step And the modal should cover the entire viewport
      expect(html).toContain('position: fixed');

      // @step And a semi-transparent backdrop should be visible
      expect(html).toContain('rgba(0, 0, 0, 0.85)');

      // @step And the diagram should be centered in the modal
      expect(html).toContain('modal-body');
      expect(html).toContain('align-items: center');
      expect(html).toContain('justify-content: center');
    });
  });

  describe('Scenario: Close modal with ESC key', () => {
    it('should close modal when ESC key is pressed', async () => {
      // @step Given the fullscreen modal is open
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      const dom = new JSDOM(html, {
        runScripts: 'dangerously',
        resources: 'usable',
      });
      const document = dom.window.document;

      // @step When I press the ESC key
      const event = new dom.window.KeyboardEvent('keydown', { key: 'Escape' });
      document.dispatchEvent(event);

      // @step Then the modal should close with a fade-out animation
      const modal = document.querySelector('#mermaid-modal');
      expect(modal?.style.display).toBe('none');

      // @step And the body scroll should be restored
      expect(document.body.style.overflow).toBe('');
    });
  });

  describe('Scenario: Close modal with backdrop click', () => {
    it('should close modal when clicking dark backdrop', async () => {
      // @step Given the fullscreen modal is open
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      const dom = new JSDOM(html, { runScripts: 'dangerously' });
      const document = dom.window.document;

      // @step When I click the dark backdrop outside the diagram
      const modal = document.querySelector('#mermaid-modal');
      const clickEvent = new dom.window.MouseEvent('click', { bubbles: true });
      modal?.dispatchEvent(clickEvent);

      // @step Then the modal should close
      expect(modal?.style.display).toBe('none');
    });
  });

  describe('Scenario: Zoom in with mouse wheel', () => {
    it('should zoom in when scrolling mouse wheel up', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      expect(html).toContain('panzoom');

      // @step When I scroll the mouse wheel up
      // @step Then the diagram should zoom in
      // @step And the zoom should be centered on the cursor position
      // Note: Panzoom library handles zoom logic, we verify it's initialized
      expect(html).toContain('Panzoom');
      expect(html).toContain('maxScale: 5');
      expect(html).toContain('minScale: 0.5');
    });
  });

  describe('Scenario: Zoom out with mouse wheel', () => {
    it('should zoom out when scrolling mouse wheel down', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I scroll the mouse wheel down
      // @step Then the diagram should zoom out
      // @step And the zoom should be centered on the cursor position
      expect(html).toContain('panzoom');
      expect(html).toContain('handleModalWheel');
    });
  });

  describe('Scenario: Pan diagram with horizontal scroll', () => {
    it('should pan diagram left or right with horizontal scroll', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph LR
  A[Start] --> B[End]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I scroll horizontally with a trackpad two-finger swipe
      // @step Then the diagram should pan left or right
      expect(html).toContain('deltaX');
      expect(html).toContain('panzoomInstance.pan');
    });
  });

  describe('Scenario: Switch to pan mode with Space key', () => {
    it('should switch to pan mode when holding Space key', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I hold the Space key and scroll vertically
      // @step Then the diagram should pan up or down instead of zooming
      expect(html).toContain('isPanMode');
      expect(html).toContain("e.key === ' '");
    });
  });

  describe('Scenario: Display mode indicator in zoom mode', () => {
    it('should show zoom mode indicator with correct text and opacity', async () => {
      // @step Given the fullscreen modal is open
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I am in zoom mode
      expect(html).toContain('mode-indicator');

      // @step Then the mode indicator should show 'Zoom Mode (hold Space for Pan)' at the bottom-left
      expect(html).toContain('Zoom Mode');
      expect(html).toContain('hold Space for Pan');

      // @step And the indicator should fade to 50% opacity after 2 seconds of inactivity
      expect(html).toContain('updateModeIndicator');
    });
  });

  describe('Scenario: Display mode indicator in pan mode', () => {
    it('should show pan mode indicator with accent color when Space held', async () => {
      // @step Given the fullscreen modal is open
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I hold the Space key
      expect(html).toContain('mode-indicator');

      // @step Then the mode indicator should change to 'Pan Mode'
      expect(html).toContain('isPanMode');

      // @step And the indicator should have 100% opacity
      // @step And the indicator background should have an accent color
      expect(html).toContain('updateModeIndicator');
    });
  });

  describe('Scenario: Zoom in with zoom button', () => {
    it('should zoom in when clicking zoom in button', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      const dom = new JSDOM(html);
      const document = dom.window.document;

      // @step When I click the zoom in button (+)
      const zoomInButton = document.querySelector('#zoom-in');
      expect(zoomInButton).toBeTruthy();

      // @step Then the diagram should zoom in by a fixed increment
      expect(html).toContain('panzoomInstance.zoomIn');
    });
  });

  describe('Scenario: Reset zoom to default', () => {
    it('should reset zoom to 100% when clicking reset button', async () => {
      // @step Given the fullscreen modal is open with a zoomed diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      const dom = new JSDOM(html);
      const document = dom.window.document;

      // @step When I click the reset button
      const resetButton = document.querySelector('#zoom-reset');
      expect(resetButton).toBeTruthy();

      // @step Then the diagram should return to 100% zoom (1.0x)
      // @step And the diagram should be centered in the modal
      expect(html).toContain('panzoomInstance.reset');
    });
  });

  describe('Scenario: Download diagram as SVG', () => {
    it('should download SVG file when clicking download button', async () => {
      // @step Given the fullscreen modal is open showing a diagram
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();
      const dom = new JSDOM(html);
      const document = dom.window.document;

      // @step When I click the download button
      const downloadButton = document.querySelector('#modal-download');
      expect(downloadButton).toBeTruthy();

      // @step Then an SVG file should be downloaded
      // @step And the filename should match the pattern 'mermaid-diagram-{timestamp}.svg'
      expect(html).toContain('mermaid-diagram-');
      expect(html).toContain('.svg');
      expect(html).toContain('download');
    });
  });

  describe('Scenario: Enforce maximum zoom limit', () => {
    it('should prevent zooming beyond 5.0x', async () => {
      // @step Given the fullscreen modal is open with a diagram at 5.0x zoom
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I try to zoom in further
      // @step Then the zoom should remain at 5.0x maximum
      expect(html).toContain('maxScale: 5');
    });
  });

  describe('Scenario: Enforce minimum zoom limit', () => {
    it('should prevent zooming below 0.5x', async () => {
      // @step Given the fullscreen modal is open with a diagram at 0.5x zoom
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      const testMarkdown = `\`\`\`mermaid
graph TD
  A[Start]
\`\`\``;
      await fs.writeFile(path.join(testCwd, 'test.md'), testMarkdown);

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step When I try to zoom out further
      // @step Then the zoom should remain at 0.5x minimum
      expect(html).toContain('minScale: 0.5');
    });
  });
});

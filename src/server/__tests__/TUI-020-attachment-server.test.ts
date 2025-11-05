/**
 * Feature: spec/features/local-web-server-for-attachment-viewing-with-markdown-and-mermaid-rendering.feature
 *
 * Tests for TUI-020: Local web server for attachment viewing with markdown and mermaid rendering
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

describe('Feature: Local web server for attachment viewing with markdown and mermaid rendering', () => {
  let server: Server | null = null;
  const testCwd = path.join(process.cwd(), 'test-fixtures', 'attachments');

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

  describe('Scenario: Server starts automatically when TUI opens', () => {
    it('should start attachment server on random port when TUI opens', async () => {
      // @step Given the TUI is not running
      expect(server).toBeNull();

      // @step When I run the TUI with 'fspec'
      server = await startAttachmentServer({ cwd: testCwd });

      // @step Then the attachment server should start on a random available port
      expect(server).not.toBeNull();
      const port = getServerPort(server);
      expect(port).toBeGreaterThan(0);

      // @step And the server should be accessible at http://localhost:<port>
      const response = await fetch(`http://localhost:${port}/health`);
      expect(response.ok).toBe(true);
      const data = await response.json();
      expect(data).toEqual({ status: 'ok' });
    });
  });

  describe('Scenario: Open markdown attachment with mermaid diagram in browser', () => {
    it('should serve rendered HTML with mermaid diagram', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step And I have a work unit with a markdown attachment containing a mermaid diagram
      const markdownContent = `# Test Diagram

\`\`\`mermaid
graph TD
  A[Start] --> B[End]
\`\`\`
`;
      const attachmentPath = path.join(testCwd, 'test-diagram.md');
      await fs.writeFile(attachmentPath, markdownContent);

      // @step When I press 'o' on the work unit
      const relativePath = 'test-diagram.md';
      const response = await fetch(
        `http://localhost:${port}/view/${encodeURIComponent(relativePath)}`
      );

      // @step Then the browser should open showing the formatted HTML
      expect(response.ok).toBe(true);
      expect(response.headers.get('content-type')).toContain('text/html');
      const html = await response.text();

      // @step And the mermaid diagram should be rendered as an SVG
      expect(html).toContain('<pre class="mermaid">');
      expect(html).toContain('graph TD');
    });
  });

  describe('Scenario: Server stops automatically when TUI exits', () => {
    it('should stop server and release port when TUI exits', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);
      expect(port).toBeGreaterThan(0);

      // Verify server is accessible
      const response1 = await fetch(`http://localhost:${port}/health`);
      expect(response1.ok).toBe(true);

      // @step When I exit the TUI
      await stopAttachmentServer(server);
      server = null;

      // @step Then the attachment server should stop
      // @step And the server port should be released
      // Attempt to connect should fail
      await expect(async () => {
        await fetch(`http://localhost:${port}/health`);
      }).rejects.toThrow();
    });
  });

  describe('Scenario: Block directory traversal attacks', () => {
    it('should return 403 Forbidden for directory traversal attempts', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step When a request is made for '../../../etc/passwd'
      const response = await fetch(
        `http://localhost:${port}/view/${encodeURIComponent('../../../etc/passwd')}`
      );

      // @step Then the server should return 403 Forbidden
      expect(response.status).toBe(403);

      // @step And the file should not be served
      const text = await response.text();
      expect(text).toContain('Forbidden');
    });
  });

  describe('Scenario: Serve non-markdown files directly', () => {
    it('should serve PNG images with correct content-type', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step And I have a work unit with a PNG image attachment
      const imagePath = path.join(testCwd, 'test-image.png');
      // Create a minimal 1x1 PNG
      const pngBuffer = Buffer.from(
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
        'base64'
      );
      await fs.writeFile(imagePath, pngBuffer);

      // @step When I press 'o' on the work unit
      const response = await fetch(
        `http://localhost:${port}/view/test-image.png`
      );

      // @step Then the browser should open displaying the image directly
      expect(response.ok).toBe(true);

      // @step And the content-type should be 'image/png'
      const contentType = response.headers.get('content-type');
      expect(contentType).toContain('image/png');
    });
  });

  describe('Scenario: Theme detection and persistence', () => {
    it('should include theme detection and localStorage persistence in HTML', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step And my OS is set to dark mode
      // @step When I open a markdown attachment with mermaid diagrams
      const markdownPath = path.join(testCwd, 'test.md');
      await fs.writeFile(
        markdownPath,
        '# Test\n\n```mermaid\ngraph TD\n  A-->B\n```'
      );

      const response = await fetch(`http://localhost:${port}/view/test.md`);
      const html = await response.text();

      // @step Then the viewer should display with dark theme
      expect(html).toContain('prefers-color-scheme: dark');

      // @step When I click the theme toggle button
      // @step Then the theme should switch to light mode
      expect(html).toContain('theme-toggle');

      // @step And the preference should be saved in localStorage
      expect(html).toContain('localStorage');
      expect(html).toContain('fspec-theme');

      // @step When I reopen the attachment later
      // @step Then the light theme should be restored from localStorage
      expect(html).toContain('getItem');
    });
  });

  describe('Scenario: Syntax-highlighted code blocks with UI features', () => {
    it('should render code blocks with syntax highlighting, copy button, and language badge', async () => {
      // @step Given the TUI is running with attachment server active
      server = await startAttachmentServer({ cwd: testCwd });
      const port = getServerPort(server);

      // @step And I have a markdown attachment with a Python code block
      const markdownContent = `# Code Example

\`\`\`python
def hello():
    print("Hello, World!")
\`\`\`
`;
      const markdownPath = path.join(testCwd, 'code-example.md');
      await fs.writeFile(markdownPath, markdownContent);

      // @step When I press 'o' on the work unit
      const response = await fetch(
        `http://localhost:${port}/view/code-example.md`
      );
      const html = await response.text();

      // @step Then the browser should show syntax-highlighted Python code
      expect(html).toContain('class="code-block"');
      expect(html).toContain('data-language="python"');

      // @step And a copy button should be visible on hover
      expect(html).toContain('copy-button');

      // @step And a language badge showing 'python' should be displayed
      expect(html).toContain('language-badge');
    });
  });
});

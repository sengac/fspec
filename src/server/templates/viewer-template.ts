/**
 * Viewer Template - HTML template for rendering markdown with mermaid and syntax highlighting
 *
 * Provides a clean HTML page with:
 * - Client-side mermaid rendering
 * - Syntax-highlighted code blocks via Prism
 * - OS theme detection with manual toggle
 * - localStorage persistence
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

import { getViewerStyles } from './viewer-styles.js';
import {
  getMermaidScript,
  getPrismScripts,
  getInteractionScript,
} from './viewer-scripts.js';
import { escapeHtml } from '../utils/html-escape.js';

export interface ViewerTemplateOptions {
  title: string;
  content: string; // Rendered HTML content
}

/**
 * Generates an HTML page for viewing rendered markdown content.
 *
 * @param options - Template options
 * @returns Complete HTML document string
 */
export function getViewerTemplate(options: ViewerTemplateOptions): string {
  const { title, content } = options;

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(title)}</title>

  <!-- Prism CSS for syntax highlighting -->
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism-themes/1.9.0/prism-vsc-dark-plus.min.css" />

${getMermaidScript()}

${getPrismScripts()}

  <style>${getViewerStyles()}</style>
</head>
<body>
  <!-- Theme toggle button -->
  <button id="theme-toggle" class="theme-toggle">
    <span id="theme-icon">🌙</span>
  </button>

  <!-- Font size controls -->
  <div id="font-size-controls" class="font-size-controls">
    <button id="font-size-decrease" class="font-size-button">−</button>
    <span id="font-size-display" class="font-size-display">16px</span>
    <button id="font-size-increase" class="font-size-button">+</button>
  </div>

  <div class="markdown-content">
    ${content}
  </div>

  <!-- Fullscreen Mermaid Modal -->
  <div id="mermaid-modal" class="modal-backdrop" style="display: none;">
    <div class="modal-container">
      <header class="modal-header">
        <h2 class="modal-title">Diagram Fullscreen View</h2>
        <div class="modal-controls">
          <button id="modal-download" class="modal-button" title="Download SVG">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
          </button>
          <button id="modal-close" class="modal-button" title="Close (ESC)">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </header>
      <div class="modal-body">
        <div id="modal-diagram-container" class="diagram-container"></div>
      </div>
      <!-- Zoom controls -->
      <div class="zoom-controls">
        <button id="zoom-in" class="zoom-btn" title="Zoom In">+</button>
        <button id="zoom-out" class="zoom-btn" title="Zoom Out">−</button>
        <button id="zoom-reset" class="zoom-btn" title="Reset Zoom">⟲</button>
        <span id="zoom-level" class="zoom-level">100%</span>
      </div>
      <!-- Mode indicator -->
      <div class="mode-indicator">Zoom Mode (hold Space for Pan)</div>
    </div>
  </div>

  <!-- Panzoom library -->
  <script src="https://cdn.jsdelivr.net/npm/@panzoom/panzoom@4.5.1/dist/panzoom.min.js"></script>

${getInteractionScript()}
</body>
</html>`;
}

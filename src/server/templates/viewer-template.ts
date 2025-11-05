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

${getInteractionScript()}
</body>
</html>`;
}

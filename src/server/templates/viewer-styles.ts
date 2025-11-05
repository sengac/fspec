/**
 * Viewer Styles - CSS styles for markdown viewer
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

/**
 * Generates CSS styles for the markdown viewer.
 *
 * @returns CSS style string
 */
export function getViewerStyles(): string {
  return `
    * {
      box-sizing: border-box;
    }

    :root {
      --bg-color: #1e1e1e;
      --text-color: #d4d4d4;
      --code-bg: #2d2d2d;
      --code-text-color: #d4d4d4;
      --border-color: #404040;
      --control-bg: rgba(255, 255, 255, 0.1);
      --control-border: rgba(255, 255, 255, 0.2);
      --control-hover-bg: rgba(255, 255, 255, 0.2);
      --button-text-color: #ffffff;
      --button-hover-text: #ffffff;
      --base-font-size: 16px;
      --font-scale: 1;
    }

    :root.light-theme {
      --bg-color: #ffffff;
      --text-color: #24292f;
      --code-bg: #f6f8fa;
      --code-text-color: #24292f;
      --border-color: #d0d7de;
      --control-bg: rgba(175, 184, 193, 0.2);
      --control-border: rgba(31, 35, 40, 0.15);
      --control-hover-bg: rgba(175, 184, 193, 0.3);
      --button-text-color: #24292f;
      --button-hover-text: #000000;
    }

    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
        'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif;
      line-height: 1.6;
      max-width: calc(900px * var(--font-scale));
      margin: 0 auto;
      padding: 2rem calc(2rem * var(--font-scale));
      background-color: var(--bg-color);
      color: var(--text-color);
      transition: background-color 0.3s, color 0.3s;
    }

    .markdown-content {
      font-size: var(--base-font-size);
      max-width: 100%;
      width: 100%;
      box-sizing: border-box;
    }

    /* Theme toggle button */
    .theme-toggle {
      position: fixed;
      top: 1rem;
      right: 1rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      padding: 0.5rem;
      border-radius: 0.5rem;
      cursor: pointer;
      font-size: 1.25rem;
      z-index: 1000;
      transition: background-color 0.2s;
    }

    .theme-toggle:hover {
      background-color: var(--control-hover-bg);
    }

    /* Font size controls */
    .font-size-controls {
      position: fixed;
      top: 1rem;
      right: 5rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      padding: 0.5rem;
      border-radius: 0.5rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      z-index: 1000;
    }

    .font-size-button {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      color: var(--text-color);
      padding: 0.25rem 0.5rem;
      border-radius: 0.25rem;
      cursor: pointer;
      font-size: 1rem;
      line-height: 1;
      min-width: 1.5rem;
      transition: background-color 0.2s;
    }

    .font-size-button:hover:not(:disabled) {
      background-color: var(--control-hover-bg);
    }

    .font-size-button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .font-size-display {
      font-size: 0.875rem;
      min-width: 3rem;
      text-align: center;
      color: var(--text-color);
    }

    /* Markdown content styles */
    .markdown-content h1,
    .markdown-content h2,
    .markdown-content h3 {
      margin-top: 1.5em;
      margin-bottom: 0.5em;
    }

    .markdown-content h1 {
      font-size: 2em;
      border-bottom: 1px solid var(--border-color);
      padding-bottom: 0.3em;
    }

    .markdown-content a {
      color: #58a6ff;
      text-decoration: none;
    }

    .markdown-content code {
      font-family: 'Courier New', Courier, monospace;
      background-color: var(--code-bg);
      padding: 0.2em 0.4em;
      border-radius: 3px;
      font-size: 85%;
      font-weight: 600;
      color: var(--code-text-color);
    }

    /* Code block styles */
    pre.code-block {
      position: relative;
      background-color: var(--code-bg);
      border: 1px solid var(--border-color);
      border-radius: 0.5rem;
      padding: 1rem;
      overflow-x: auto;
      margin: 1.5rem 0;
    }

    pre.code-block code {
      background-color: transparent;
      padding: 0;
      font-size: var(--base-font-size);
      font-weight: 600;
      color: var(--code-text-color);
    }

    /* Copy button */
    .copy-button {
      position: absolute;
      top: 0.5rem;
      right: 0.5rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      color: var(--button-text-color);
      padding: 0.25rem 0.5rem;
      border-radius: 0.25rem;
      font-size: 0.75rem;
      font-weight: 500;
      cursor: pointer;
      opacity: 0;
      transition: opacity 0.2s, background-color 0.2s;
    }

    .copy-button:hover {
      background-color: var(--control-hover-bg);
      color: var(--button-hover-text);
    }

    pre.code-block:hover .copy-button {
      opacity: 1;
    }

    /* Language badge */
    .language-badge {
      position: absolute;
      bottom: 0.5rem;
      right: 0.5rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      color: var(--button-text-color);
      padding: 0.125rem 0.5rem;
      border-radius: 0.25rem;
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: uppercase;
    }

    /* Mermaid diagram styles */
    .mermaid {
      margin: 1.5rem 0;
      text-align: center;
      background-color: transparent;
      width: 100%;
      max-width: 100%;
    }

    .mermaid svg {
      max-width: 100%;
      height: auto;
      display: block;
      margin: 0 auto;
    }

    /* Mermaid wrapper with hover buttons */
    .mermaid-wrapper {
      position: relative;
      display: block;
      margin: 1.5rem 0;
      text-align: center;
      width: 100%;
    }

    .mermaid-wrapper > pre {
      display: block;
      width: 100%;
    }

    .mermaid-overlay {
      position: absolute;
      top: 0;
      right: 0;
      display: flex;
      gap: 0.5rem;
      padding: 0.5rem;
      opacity: 0;
      transition: opacity 0.2s ease;
      pointer-events: none;
    }

    .mermaid-wrapper:hover .mermaid-overlay {
      opacity: 1;
      pointer-events: auto;
    }

    .mermaid-fullscreen-btn,
    .mermaid-download-btn {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      color: var(--button-text-color);
      padding: 0.5rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .mermaid-fullscreen-btn:hover,
    .mermaid-download-btn:hover {
      background-color: var(--button-hover-bg);
      transform: scale(1.05);
    }

    .mermaid-fullscreen-btn svg,
    .mermaid-download-btn svg {
      width: 20px;
      height: 20px;
    }

    /* Fullscreen modal */
    .modal-backdrop {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(4px);
      z-index: 9999;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 2rem;
    }

    .modal-container {
      width: 100%;
      height: 100%;
      max-width: 100%;
      max-height: 100%;
      background-color: var(--bg-color);
      border-radius: 0.75rem;
      box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    .modal-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.5rem;
      border-bottom: 1px solid var(--control-border);
      background-color: var(--code-bg);
    }

    .modal-title {
      margin: 0;
      font-size: 1.25rem;
      font-weight: 600;
      color: var(--text-color);
    }

    .modal-controls {
      display: flex;
      gap: 0.5rem;
    }

    .modal-button {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      color: var(--button-text-color);
      padding: 0.5rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .modal-button:hover {
      background-color: var(--button-hover-bg);
      transform: scale(1.05);
    }

    .modal-button svg {
      width: 20px;
      height: 20px;
    }

    .modal-body {
      flex: 1;
      overflow: hidden;
      position: relative;
      background-color: var(--bg-color);
    }

    .diagram-container {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      cursor: grab;
    }

    .diagram-container:active {
      cursor: grabbing;
    }

    .diagram-container.pan-mode {
      cursor: move;
    }

    .diagram-container svg {
      max-width: none;
      max-height: none;
    }

    /* Zoom controls */
    .zoom-controls {
      position: absolute;
      bottom: 1rem;
      right: 1rem;
      display: flex;
      gap: 0.5rem;
      align-items: center;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      padding: 0.5rem;
      border-radius: 0.5rem;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
    }

    .zoom-btn {
      background-color: var(--button-bg);
      border: 1px solid var(--control-border);
      color: var(--button-text-color);
      width: 2rem;
      height: 2rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 1.25rem;
      font-weight: 600;
      transition: all 0.2s ease;
    }

    .zoom-btn:hover {
      background-color: var(--button-hover-bg);
      transform: scale(1.05);
    }

    .zoom-level {
      min-width: 3rem;
      text-align: center;
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--text-color);
    }

    /* Mode indicator */
    .mode-indicator {
      position: absolute;
      bottom: 1rem;
      left: 1rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      color: var(--button-text-color);
      padding: 0.5rem 1rem;
      border-radius: 0.5rem;
      font-size: 0.875rem;
      font-weight: 500;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
      opacity: 0;
      transition: opacity 0.2s ease;
      pointer-events: none;
    }

    .mode-indicator.visible {
      opacity: 1;
    }

    /* Prism syntax highlighting overrides for light theme */
    :root.light-theme .token.comment,
    :root.light-theme .token.prolog,
    :root.light-theme .token.doctype,
    :root.light-theme .token.cdata {
      color: #6a737d;
    }

    :root.light-theme .token.punctuation {
      color: #24292f;
    }

    :root.light-theme .token.property,
    :root.light-theme .token.tag,
    :root.light-theme .token.boolean,
    :root.light-theme .token.number,
    :root.light-theme .token.constant,
    :root.light-theme .token.symbol,
    :root.light-theme .token.deleted {
      color: #005cc5;
    }

    :root.light-theme .token.selector,
    :root.light-theme .token.attr-name,
    :root.light-theme .token.string,
    :root.light-theme .token.char,
    :root.light-theme .token.builtin,
    :root.light-theme .token.inserted {
      color: #032f62;
    }

    :root.light-theme .token.operator,
    :root.light-theme .token.entity,
    :root.light-theme .token.url,
    :root.light-theme .language-css .token.string,
    :root.light-theme .style .token.string {
      color: #d73a49;
    }

    :root.light-theme .token.atrule,
    :root.light-theme .token.attr-value,
    :root.light-theme .token.keyword {
      color: #d73a49;
    }

    :root.light-theme .token.function,
    :root.light-theme .token.class-name {
      color: #6f42c1;
    }

    :root.light-theme .token.regex,
    :root.light-theme .token.important,
    :root.light-theme .token.variable {
      color: #e36209;
    }
  `;
}

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
      --border-color: #404040;
      --base-font-size: 16px;
      --font-scale: 1;
    }

    :root.light-theme {
      --bg-color: #ffffff;
      --text-color: #1e1e1e;
      --code-bg: #f5f5f5;
      --border-color: #d0d0d0;
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
    }

    /* Theme toggle button */
    .theme-toggle {
      position: fixed;
      top: 1rem;
      right: 1rem;
      background-color: rgba(255, 255, 255, 0.1);
      border: 1px solid rgba(255, 255, 255, 0.2);
      padding: 0.5rem;
      border-radius: 0.5rem;
      cursor: pointer;
      font-size: 1.25rem;
      z-index: 1000;
    }

    /* Font size controls */
    .font-size-controls {
      position: fixed;
      top: 1rem;
      right: 5rem;
      background-color: rgba(255, 255, 255, 0.1);
      border: 1px solid rgba(255, 255, 255, 0.2);
      padding: 0.5rem;
      border-radius: 0.5rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      z-index: 1000;
    }

    .font-size-button {
      background-color: rgba(255, 255, 255, 0.1);
      border: 1px solid rgba(255, 255, 255, 0.2);
      color: var(--text-color);
      padding: 0.25rem 0.5rem;
      border-radius: 0.25rem;
      cursor: pointer;
      font-size: 1rem;
      line-height: 1;
      min-width: 1.5rem;
    }

    .font-size-button:hover:not(:disabled) {
      background-color: rgba(255, 255, 255, 0.2);
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
    }

    /* Copy button */
    .copy-button {
      position: absolute;
      top: 0.5rem;
      right: 0.5rem;
      background-color: rgba(255, 255, 255, 0.1);
      border: 1px solid rgba(255, 255, 255, 0.2);
      color: #ffffff;
      padding: 0.25rem 0.5rem;
      border-radius: 0.25rem;
      font-size: 0.75rem;
      cursor: pointer;
      opacity: 0;
      transition: opacity 0.2s;
    }

    pre.code-block:hover .copy-button {
      opacity: 1;
    }

    /* Language badge */
    .language-badge {
      position: absolute;
      bottom: 0.5rem;
      right: 0.5rem;
      background-color: rgba(0, 0, 0, 0.6);
      backdrop-filter: blur(4px);
      color: #ffffff;
      padding: 0.125rem 0.5rem;
      border-radius: 0.25rem;
      font-size: 0.75rem;
      text-transform: uppercase;
    }

    /* Mermaid diagram styles */
    .mermaid {
      margin: 1.5rem 0;
      text-align: center;
      background-color: transparent;
    }
  `;
}

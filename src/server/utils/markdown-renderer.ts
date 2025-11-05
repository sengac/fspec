/**
 * Markdown Renderer - Converts markdown to HTML with mermaid support
 *
 * Uses 'marked' for markdown parsing and preserves mermaid code blocks
 * for client-side rendering.
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

import { marked } from 'marked';
import { gfmHeadingId } from 'marked-gfm-heading-id';
import { escapeHtml } from './html-escape.js';

/**
 * Marked code token interface
 */
interface MarkedCodeToken {
  text: string;
  lang?: string;
  infostring?: string;
}

// Configure marked once at module level to avoid race conditions
// This ensures multiple concurrent requests don't conflict

// Add GitHub-compatible heading IDs for anchor navigation (TUI-022)
// This extension automatically generates id attributes for headings (h1-h6)
// and handles duplicate headings with numbered suffixes (-1, -2, etc.)
marked.use(gfmHeadingId());

marked.use({
  gfm: true, // GitHub Flavored Markdown
  breaks: true, // Convert \n to <br>
  renderer: {
    code(token: MarkedCodeToken): string {
      // Extract code text and language from token object
      const code = token.text || '';
      const lang = token.lang || token.infostring || '';

      // Mermaid diagrams get special treatment
      if (lang === 'mermaid') {
        return `<pre class="mermaid">\n${escapeHtml(code)}\n</pre>\n`;
      }

      // Regular code blocks with data attributes for client-side highlighting
      const language = lang || 'text';
      return `<pre class="code-block" data-language="${language}"><code>${escapeHtml(code)}</code></pre>\n`;
    },
  },
});

/**
 * Renders markdown content to HTML, preserving code blocks for client-side highlighting.
 *
 * Code blocks are converted to <pre class="code-block"> elements with data attributes
 * for language detection. Mermaid blocks get special <pre class="mermaid"> treatment.
 *
 * @param markdown - Markdown content to render
 * @returns Rendered HTML string
 */
export function renderMarkdown(markdown: string): string {
  // Defensive check: ensure markdown is a string
  if (typeof markdown !== 'string') {
    markdown = String(markdown || '');
  }

  // Parse and render markdown (configuration already set at module level)
  const html = marked.parse(markdown) as string;

  return html;
}

/**
 * HTML Escape Utility - Escapes HTML special characters to prevent XSS
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

/**
 * Escapes HTML special characters to prevent XSS attacks.
 *
 * @param text - Text to escape
 * @returns Escaped text safe for HTML insertion
 */
export function escapeHtml(text: string): string {
  // Defensive check: ensure text is a string
  if (typeof text !== 'string') {
    text = String(text || '');
  }

  const map: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#039;',
  };

  return text.replace(/[&<>"']/g, char => map[char] || char);
}

/**
 * Thinking Block Handler
 *
 * BRIDGE-006: Intelligent Content-Aware Chunking for Telegram Display
 *
 * Manages the state and formatting of thinking blocks wrapped in <think>...</think> tags.
 *
 * Single Responsibility: Only handles thinking block state and tag generation.
 * Open/Closed: Can be extended without modification (e.g., different tag formats).
 * Composable: Stateless formatting methods + stateful block tracking.
 */

// ===========================================
// CONSTANTS
// ===========================================

/** Opening tag for thinking blocks (escaped for MarkdownV2) */
const THINK_OPEN_TAG = '\\<think\\>';

/** Closing tag for thinking blocks (escaped for MarkdownV2) */
const THINK_CLOSE_TAG = '\\</think\\>';

// ===========================================
// HANDLER CLASS
// ===========================================

/**
 * Manages the state and formatting of thinking blocks.
 *
 * Usage:
 * ```typescript
 * const handler = new ThinkingBlockHandler();
 *
 * // First thinking chunk - opens block
 * buffer.push(handler.processThinking('First thought'));
 *
 * // Subsequent chunks - content only
 * buffer.push(handler.processThinking('Second thought'));
 *
 * // When transitioning to non-thinking content
 * buffer.push(handler.closeIfOpen());
 * ```
 */
export class ThinkingBlockHandler {
  private _isOpen: boolean = false;

  /**
   * Process a thinking chunk.
   *
   * @param content - The thinking content (already escaped or raw)
   * @returns Opening tag + content for first chunk, just content for subsequent
   */
  processThinking(content: string): string {
    if (!this._isOpen) {
      this._isOpen = true;
      return THINK_OPEN_TAG + content;
    }
    return content;
  }

  /**
   * Close the thinking block if open.
   *
   * @returns Closing tag if block was open, empty string otherwise
   */
  close(): string {
    if (this._isOpen) {
      this._isOpen = false;
      return THINK_CLOSE_TAG + '\n\n';
    }
    return '';
  }

  /**
   * Alias for close() - more explicit about the conditional behavior.
   *
   * @returns Closing tag if block was open, empty string otherwise
   */
  closeIfOpen(): string {
    return this.close();
  }

  /**
   * Check if a thinking block is currently open.
   */
  isOpen(): boolean {
    return this._isOpen;
  }

  /**
   * Reset the handler state.
   * Does NOT return closing tag - use close() first if you need it.
   */
  reset(): void {
    this._isOpen = false;
  }
}

// ===========================================
// FACTORY FUNCTION
// ===========================================

/**
 * Create a new ThinkingBlockHandler instance.
 * Useful for dependency injection and testing.
 */
export function createThinkingBlockHandler(): ThinkingBlockHandler {
  return new ThinkingBlockHandler();
}

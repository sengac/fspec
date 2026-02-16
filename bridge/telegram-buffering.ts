/**
 * Telegram Message Buffering
 *
 * Handles buffering of stream chunks before sending to Telegram.
 * Implements idle-flush strategy to reduce API calls while maintaining responsiveness.
 *
 * BRIDGE-002: Telegram Bridge Endpoint (extracted for maintainability)
 */

import type TelegramBot from 'node-telegram-bot-api';
import { truncateMessage } from './telegram-formatting';

// ============================================================================
// Buffering Configuration
// ============================================================================

/** Flush buffer after this many milliseconds of no new chunks (idle detection) */
export const BUFFER_IDLE_FLUSH_MS = 800;

/** Minimum time between sends to avoid rate limiting */
export const MIN_SEND_INTERVAL_MS = 300;

/** Force flush if buffer exceeds this many chunks */
export const MAX_BUFFER_SIZE = 50;

/** Force flush if buffer approaches Telegram's 4096 character limit */
export const MAX_BUFFER_CHARS = 3500;

// ============================================================================
// Buffer State Interface
// ============================================================================

export interface BufferState {
  messageBuffer: string[];
  bufferCharCount: number;
  bufferTimer: ReturnType<typeof setTimeout> | null;
  lastSendTime: number;
  lastChunkTime: number;
  bot: TelegramBot | null;
  chatId: string | null;
}

// ============================================================================
// Buffer Operations
// ============================================================================

/**
 * Flush the message buffer to Telegram.
 * Combines all buffered chunks into a single message.
 */
export async function flushBuffer(state: BufferState): Promise<void> {
  // Clear the timer
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
    state.bufferTimer = null;
  }

  // Nothing to flush
  if (state.messageBuffer.length === 0) {
    return;
  }

  if (!state.bot || !state.chatId) {
    state.messageBuffer = [];
    state.bufferCharCount = 0;
    return;
  }

  // Combine all buffered messages
  const combined = state.messageBuffer.join('');
  state.messageBuffer = [];
  state.bufferCharCount = 0;

  if (!combined.trim()) {
    return;
  }

  // Truncate if necessary
  const text = truncateMessage(combined);

  // Send to Telegram
  try {
    state.lastSendTime = Date.now();
    await state.bot.sendMessage(state.chatId, text, {
      parse_mode: 'MarkdownV2',
    });
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('[telegram-buffering] Telegram API error:', errorMessage);
    // Drop the message and continue
  }
}

/**
 * Schedule a buffer flush based on idle time.
 * Resets the timer each time new content arrives, so we only flush
 * after a period of no new chunks (idle flush).
 */
export function scheduleIdleFlush(state: BufferState): void {
  // Clear any existing timer - we reset on each new chunk
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
  }

  // Calculate delay - respect minimum send interval
  const timeSinceLastSend = Date.now() - state.lastSendTime;
  const delay = Math.max(
    BUFFER_IDLE_FLUSH_MS,
    MIN_SEND_INTERVAL_MS - timeSinceLastSend
  );

  state.bufferTimer = setTimeout(() => {
    void flushBuffer(state);
  }, delay);
}

/**
 * Add text to the buffer with optional newlines.
 */
export function addToBuffer(
  state: BufferState,
  text: string,
  prependNewline: boolean = false,
  appendNewline: boolean = false
): void {
  if (prependNewline && state.messageBuffer.length > 0) {
    state.messageBuffer.push('\n');
    state.bufferCharCount += 1;
  }

  state.messageBuffer.push(text);
  state.bufferCharCount += text.length;

  if (appendNewline) {
    state.messageBuffer.push('\n');
    state.bufferCharCount += 1;
  }
}

/**
 * Check if buffer should be force-flushed due to size limits.
 */
export function shouldForceFlush(state: BufferState): boolean {
  return (
    state.bufferCharCount >= MAX_BUFFER_CHARS ||
    state.messageBuffer.length >= MAX_BUFFER_SIZE
  );
}

/**
 * Clear the buffer without sending.
 */
export function clearBuffer(state: BufferState): void {
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
    state.bufferTimer = null;
  }
  state.messageBuffer = [];
  state.bufferCharCount = 0;
}

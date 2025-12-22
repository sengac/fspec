/**
 * Unicode-aware string width utilities for terminal rendering.
 *
 * Handles the common issue where string-width library reports widths that
 * don't match actual terminal rendering for certain Unicode characters.
 *
 * The Problem:
 * Many characters have Emoji_Presentation=No in Unicode (they're text by default):
 * - Warning sign, pencil, desktop, cloud, heart, star, etc.
 *
 * When U+FE0F (Variation Selector-16) is added, string-width correctly reports
 * width 2 (emoji presentation). BUT many terminals IGNORE U+FE0F and render
 * as width 1, causing layout misalignment (borders, padding, columns).
 *
 * The Fix:
 * Strip ALL U+FE0F variation selectors from text before measuring.
 * - For text-default chars: removes VS16, string-width reports 1, terminal renders 1
 * - For emoji-default chars: they're already emoji, VS16 is redundant, no effect
 *
 * See: border-debug.test.tsx for detailed reproduction and explanation.
 */

import stringWidth from 'string-width';

/**
 * Normalize emoji sequences for consistent terminal width calculation.
 *
 * Handles two major issues:
 *
 * 1. Variation Selector-16 (U+FE0F):
 *    Text-default emojis with VS16 cause string-width to report width 2,
 *    but terminals often render them as width 1.
 *
 * 2. Zero Width Joiner sequences (U+200D):
 *    ZWJ sequences like 👨‍👩‍👧‍👦 (family) are supposed to render as ONE emoji,
 *    but many terminals don't support them and render each component separately.
 *    string-width reports 2 (one emoji), but terminal shows 4 emojis (width 8).
 *
 * @param text - Text potentially containing emoji sequences
 * @returns Text with problematic sequences normalized for width calculation
 */
export function normalizeEmojiWidth(text: string): string {
  // Remove U+FE0F (Variation Selector-16) - fixes text-default emoji width
  // Remove U+200D (Zero Width Joiner) - expands ZWJ sequences to components
  // This ensures string-width matches what the terminal actually renders
  return text.replace(/[\uFE0F\u200D]/g, '');
}

/**
 * Get the visual width of a string in terminal columns.
 *
 * This is a normalized version of string-width that handles emoji variation
 * selectors correctly for terminal rendering.
 *
 * @param text - Text to measure
 * @returns Visual width in terminal columns
 */
export function getVisualWidth(text: string): number {
  return stringWidth(normalizeEmojiWidth(text));
}

/**
 * Fit text to a specific width, truncating or padding as needed.
 *
 * Uses normalized width calculation to handle Unicode correctly.
 *
 * @param text - Text to fit
 * @param width - Target width in terminal columns
 * @returns Text truncated or padded to exact width
 */
export function fitToWidth(text: string, width: number): string {
  const normalized = normalizeEmojiWidth(text);
  const visualWidth = stringWidth(normalized);

  if (visualWidth > width) {
    // Truncate by iterating through codepoints and measuring width
    let result = '';
    let currentVisualWidth = 0;
    for (const char of normalized) {
      const charWidth = stringWidth(char);
      if (currentVisualWidth + charWidth > width) break;
      result += char;
      currentVisualWidth += charWidth;
    }
    // Pad to exact width
    return result + ' '.repeat(width - currentVisualWidth);
  } else if (visualWidth < width) {
    // Pad with spaces to reach visual width
    return normalized + ' '.repeat(width - visualWidth);
  }
  return normalized;
}

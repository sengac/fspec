/**
 * Unicode whitespace normalization for file paths.
 *
 * macOS uses U+202F (NARROW NO-BREAK SPACE) in screenshot filenames:
 *   "Screenshot 2026-04-13 at 9.13.45\u202fam.png"
 *
 * This module provides two strategies:
 * - normalizeFilePath(): Replace Unicode whitespace → ASCII space (fast, sync)
 * - resolveFilePath(): Try exact → normalized → directory scan (robust, async)
 */

import { access, readdir } from 'fs/promises';
import { dirname, basename, join } from 'path';

/**
 * All Unicode whitespace characters (category Zs) that should normalize
 * to ASCII space (U+0020). Covers:
 * - U+00A0 NO-BREAK SPACE
 * - U+1680 OGHAM SPACE MARK
 * - U+2000-U+200A Various typographic spaces
 * - U+202F NARROW NO-BREAK SPACE (macOS screenshot filenames)
 * - U+205F MEDIUM MATHEMATICAL SPACE
 * - U+3000 IDEOGRAPHIC SPACE
 */
const UNICODE_WHITESPACE_RE = /[\u00A0\u1680\u2000-\u200A\u202F\u205F\u3000]/g;

/**
 * Replace Unicode whitespace variants with ASCII space (U+0020).
 *
 * This is a fast, synchronous, idempotent operation. Apply at input
 * boundaries where user-provided paths enter the system.
 *
 * Path separators (/ and \) are NOT affected.
 *
 * @param filePath - The file path to normalize
 * @returns The path with all Unicode whitespace replaced by ASCII space
 */
export function normalizeFilePath(filePath: string): string {
  return filePath.replace(UNICODE_WHITESPACE_RE, ' ');
}

/**
 * Two-phase file resolution for paths with potential Unicode whitespace.
 *
 * Phase 1 (fast): Try exact path, then normalized path.
 * Phase 2 (robust): Scan parent directory for fuzzy whitespace match.
 *
 * Handles BOTH directions:
 * - User pastes U+202F but file has regular space → Phase 1 normalization
 * - User types regular space but file has U+202F → Phase 2 directory scan
 *
 * @param filePath - The file path to resolve
 * @returns The path that actually exists on disk, or the normalized path as fallback
 */
export async function resolveFilePath(filePath: string): Promise<string> {
  // Phase 1a: Try exact path (fast path, preserves original Unicode)
  try {
    await access(filePath);
    return filePath;
  } catch {
    // Not found as-is
  }

  // Phase 1b: Try with normalized whitespace
  const normalized = normalizeFilePath(filePath);
  if (normalized !== filePath) {
    try {
      await access(normalized);
      return normalized;
    } catch {
      // Normalized also not found
    }
  }

  // Phase 2: Scan directory for entry whose normalized name matches
  try {
    const dir = dirname(filePath);
    const targetBase = normalizeFilePath(basename(filePath));
    const entries = await readdir(dir);

    for (const entry of entries) {
      if (normalizeFilePath(entry) === targetBase) {
        return join(dir, entry);
      }
    }
  } catch {
    // Directory unreadable — fall through
  }

  // Nothing found — return normalized so caller gets a clean error message
  return normalized;
}

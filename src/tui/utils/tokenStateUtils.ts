/**
 * Token State Utilities
 *
 * SOLID: Pure functions for extracting and calculating token/context state
 * DRY: Single source of truth for token state extraction from chunks
 *      and context fill percentage calculation
 */

import type { TokenTracker } from '../types/conversation';
import { logger } from '../../utils/logger';
import {
  sessionGetTokens,
  persistenceSetSessionTokens,
} from '@sengac/codelet-napi';

/**
 * StreamChunk type subset for token state extraction
 * Uses the minimal interface needed to avoid circular dependencies
 */
interface TokenStateChunk {
  type: string;
  tokens?: TokenTracker;
  contextFill?: {
    fillPercentage: number;
    // RPC-101: extra fields carried on the wire by ContextFillInfo
    // (codelet/napi/index.d.ts:2952-2957). Optional here because
    // older fixtures only set fillPercentage; consumers that want
    // real-time recompute on TokenUpdate need threshold.
    effectiveTokens?: number;
    threshold?: number;
    contextWindow?: number;
  };
}

/**
 * Result of extracting token state from chunks
 */
export interface ExtractedTokenState {
  /** Last token usage (input/output tokens, cache tokens) */
  tokenUsage: TokenTracker | null;
  /** Last context fill percentage */
  contextFillPercentage: number | null;
  /** Tokens per second (only valid for running sessions) */
  tokensPerSecond: number | null;
  /**
   * RPC-101: Cached context-fill threshold (in tokens) from the last
   * ContextFillUpdate in the buffer. Callers seed
   * `cachedContextThresholdRef` with this so subsequent live
   * TokenUpdates can recompute the percentage locally without
   * waiting for the next ContextFillUpdate from the backend.
   */
  contextThreshold: number | null;
}

/**
 * Extract the last token usage and context fill percentage from a list of stream chunks.
 *
 * Scans through chunks to find the LAST TokenUpdate and ContextFillUpdate,
 * which represent the most current state of the session.
 *
 * Used when restoring session state from buffered output (resume, switch session).
 *
 * @param chunks - Array of stream chunks to scan
 * @returns Extracted token state (tokenUsage, contextFillPercentage, tokensPerSecond, contextThreshold)
 */
export function extractTokenStateFromChunks(
  chunks: TokenStateChunk[]
): ExtractedTokenState {
  let lastTokenUpdate: TokenStateChunk | null = null;
  let lastContextFillUpdate: TokenStateChunk | null = null;

  for (const chunk of chunks) {
    if (chunk.type === 'TokenUpdate' && chunk.tokens) {
      lastTokenUpdate = chunk;
    } else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
      lastContextFillUpdate = chunk;
    }
  }

  // Extract tokensPerSecond from TokenTracker if present
  // Note: This should only be used for running sessions; callers should
  // check session status before applying tokensPerSecond to display
  const tokensPerSecond =
    lastTokenUpdate?.tokens &&
    'tokensPerSecond' in lastTokenUpdate.tokens &&
    lastTokenUpdate.tokens.tokensPerSecond !== undefined
      ? (lastTokenUpdate.tokens.tokensPerSecond as number)
      : null;

  // RPC-101: surface the last known threshold so callers can prime
  // the realtime-recompute cache. Returns null when threshold is
  // missing or non-positive (older fixtures emit ContextFillUpdate
  // without it).
  const rawThreshold = lastContextFillUpdate?.contextFill?.threshold;
  const contextThreshold =
    typeof rawThreshold === 'number' && rawThreshold > 0 ? rawThreshold : null;

  return {
    tokenUsage: lastTokenUpdate?.tokens ?? null,
    contextFillPercentage:
      lastContextFillUpdate?.contextFill?.fillPercentage ?? null,
    tokensPerSecond,
    contextThreshold,
  };
}

/**
 * Maximum output token reservation for context threshold calculation.
 * Matches the constant in Rust (codelet/cli/src/compaction_threshold.rs).
 */
const MAX_OUTPUT_RESERVATION = 32000;

/**
 * Calculate context fill percentage from token count and model limits.
 *
 * Uses the same formula as Rust:
 *   threshold = contextWindow - min(maxOutput, 32000)
 *   fillPercentage = round((inputTokens / threshold) * 100)
 *
 * This ensures consistency between TypeScript and Rust calculations.
 *
 * @param inputTokens - Current context size in tokens
 * @param contextWindow - Model's total context window size
 * @param maxOutput - Model's maximum output tokens
 * @returns Fill percentage (0-100+, can exceed 100 near compaction threshold)
 */
export function calculateContextFillPercentage(
  inputTokens: number,
  contextWindow: number,
  maxOutput: number
): number {
  // Reserve space for output, capped at MAX_OUTPUT_RESERVATION
  const maxOutputReservation = Math.min(maxOutput, MAX_OUTPUT_RESERVATION);
  const threshold = contextWindow - maxOutputReservation;

  if (threshold <= 0) {
    return 0;
  }

  return Math.round((inputTokens / threshold) * 100);
}

/**
 * Persist token state to disk for session restoration.
 *
 * Best-effort persistence - errors are logged but don't interrupt the caller.
 * This ensures /resume can restore token counts from persisted sessions.
 *
 * @param sessionId - Session ID to persist tokens for (null = no-op)
 */
export function persistTokenState(sessionId: string | null): void {
  if (!sessionId) {
    return;
  }
  try {
    const tokens = sessionGetTokens(sessionId);
    persistenceSetSessionTokens(
      sessionId,
      tokens.inputTokens,
      tokens.outputTokens,
      0, // cacheRead - not tracked in SessionTokens
      0, // cacheCreate - not tracked in SessionTokens
      tokens.inputTokens, // cumulativeInput (using current as fallback)
      tokens.outputTokens // cumulativeOutput
    );
  } catch (e) {
    logger.error(`Failed to persist token state for session ${sessionId}:`, e);
  }
}

// TOOL-010: Dynamic Thinking Level Detection via Keywords
// BRIDGE-006: Unified Thinking Level Detection (DRY)
//
// Single source of truth: Rust (rust/napi/src/thinking_level_detection.rs)
// This TypeScript module now wraps the Rust NAPI functions for UI display purposes.
//
// The detection logic lives in Rust and is applied in agent_loop for ALL input paths:
// - TUI user input
// - Bridge/Telegram input
// - Supervisor input
//
// Priority: disable keywords > high > medium > low > conversational exclusion
//
// Keywords modeled after Claude Code's approach:
// - ultrathink, think harder → High (~32K tokens)
// - megathink, think hard → Medium (~10K tokens)
// - think about, think through → Low (~4K tokens)
// - quickly, briefly, nothink → Disable (Off)

import {
  napiDetectThinkingLevel,
  napiHasDisableKeywords,
  napiComputeEffectiveThinkingLevel,
} from '@sengac/codelet-napi';

// JsThinkingLevel enum - mirrors @sengac/codelet-napi
export enum JsThinkingLevel {
  Off = 0,
  Low = 1,
  Medium = 2,
  High = 3,
}

/**
 * Detect thinking level from prompt keywords.
 *
 * DRY: Calls Rust NAPI function - single source of truth.
 *
 * Priority order:
 * 1. Disable keywords (quickly, briefly, etc.) → Off
 * 2. Conversational patterns (I think, what do you think) → Off
 * 3. High-level keywords (ultrathink, think harder) → High
 * 4. Medium-level keywords (megathink, think hard) → Medium
 * 5. Low-level keywords (think about, think through) → Low
 * 6. No match → Off
 *
 * @param prompt - The user's prompt text
 * @returns The detected thinking level
 */
export function detectThinkingLevel(prompt: string): JsThinkingLevel {
  return napiDetectThinkingLevel(prompt) as JsThinkingLevel;
}

/**
 * Get display label for thinking level.
 *
 * @param level - The thinking level
 * @returns Display string with emoji, or null if Off
 */
export function getThinkingLevelLabel(level: JsThinkingLevel): string | null {
  switch (level) {
    case JsThinkingLevel.High:
      return '🧠 High';
    case JsThinkingLevel.Medium:
      return '🧠 Medium';
    case JsThinkingLevel.Low:
      return '🧠 Low';
    default:
      return null;
  }
}

/**
 * TUI-054: Check if disable keywords were detected in prompt.
 *
 * DRY: Calls Rust NAPI function - single source of truth.
 *
 * This is used to determine if the effective level should be forced to Off
 * regardless of the base level.
 *
 * @param prompt - The user's prompt text
 * @returns true if disable keywords were found
 */
export function hasDisableKeywords(prompt: string): boolean {
  return napiHasDisableKeywords(prompt);
}

/**
 * TUI-054: Compute effective thinking level from base level and detected level.
 *
 * DRY: Calls Rust NAPI function - single source of truth.
 *
 * Rules:
 * 1. If disable keywords detected (forceOff=true), always return Off
 * 2. Otherwise, return max(baseLevel, detectedLevel)
 *
 * This allows text keywords to INCREASE the level (e.g., base=Medium + ultrathink → High)
 * but not DECREASE it (e.g., base=High + think about → High, not Low).
 *
 * Exception: Disable keywords (quickly, briefly) ALWAYS force Off regardless of base.
 *
 * @param baseLevel - The base thinking level set via /thinking dialog
 * @param detectedLevel - The level detected from prompt keywords
 * @param forceOff - If true, disable keywords were detected (force Off)
 * @returns The effective thinking level to use
 */
export function computeEffectiveThinkingLevel(
  baseLevel: JsThinkingLevel,
  detectedLevel: JsThinkingLevel,
  forceOff: boolean = false
): JsThinkingLevel {
  return napiComputeEffectiveThinkingLevel(
    baseLevel,
    detectedLevel,
    forceOff
  ) as JsThinkingLevel;
}

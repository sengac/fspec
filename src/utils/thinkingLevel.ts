// TOOL-010: Dynamic Thinking Level Detection via Keywords
//
// Detects thinking/reasoning level from prompt keywords.
// Priority: disable keywords > high > medium > low > conversational exclusion
//
// Keywords modeled after Claude Code's approach:
// - ultrathink, think harder → High (~32K tokens)
// - megathink, think hard → Medium (~10K tokens)
// - think about, think through → Low (~4K tokens)
// - quickly, briefly, nothink → Disable (Off)

// JsThinkingLevel enum - mirrors @sengac/codelet-napi
export enum JsThinkingLevel {
  Off = 0,
  Low = 1,
  Medium = 2,
  High = 3,
}

// Disable keywords have HIGHEST priority - always force Off
const DISABLE_KEYWORDS = [
  'quickly',
  'brief',
  'briefly',
  'fast',
  'nothink',
  'no thinking',
  "don't think hard",
  "don't overthink",
];

// High-level patterns (explicit thinking commands)
const HIGH_PATTERNS = [
  /\bultrathink\b/i,
  /\bthink\s+harder\b/i,
  /\bthink\s+intensely\b/i,
  /\bthink\s+very\s+hard\b/i,
  /\bthink\s+super\s+hard\b/i,
  /\bthink\s+really\s+hard\b/i,
  /\bthink\s+longer\b/i,
];

// Medium-level patterns
const MEDIUM_PATTERNS = [
  /\bmegathink\b/i,
  /\bthink\s+hard\b/i,
  /\bthink\s+deeply\b/i,
  /\bthink\s+more\b/i,
  /\bthink\s+a\s+lot\b/i,
];

// Low-level patterns (command-like phrases)
const LOW_PATTERNS = [
  /\bthink\s+about\b/i,
  /\bthink\s+through\b/i,
  /\bthink\s+carefully\b/i,
  /^think\b/i, // Starts with "think"
  /[:.]\s*think\b/i, // After colon or period
];

// Conversational patterns - DO NOT match these
const CONVERSATIONAL_PATTERNS = [
  /\bi\s+think\b/i, // "I think..."
  /\bwhat\s+do\s+you\s+think\b/i, // "what do you think"
  /\bdon'?t\s+think\s+so\b/i, // "don't think so"
  /\bwas\s+thinking\b/i, // "was thinking"
  /\bthinking\s+about\b/i, // "thinking about" (gerund)
  /\bi\s+was\s+thinking\b/i, // "I was thinking"
  /\bdo\s+you\s+think\b/i, // "do you think"
];

/**
 * Detect thinking level from prompt keywords.
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
  const lower = prompt.toLowerCase();

  // 1. DISABLE keywords have highest priority
  if (DISABLE_KEYWORDS.some(kw => lower.includes(kw))) {
    return JsThinkingLevel.Off;
  }

  // 2. Skip if conversational usage
  if (CONVERSATIONAL_PATTERNS.some(p => p.test(prompt))) {
    return JsThinkingLevel.Off;
  }

  // 3. Check HIGH level
  if (HIGH_PATTERNS.some(p => p.test(prompt))) {
    return JsThinkingLevel.High;
  }

  // 4. Check MEDIUM level
  if (MEDIUM_PATTERNS.some(p => p.test(prompt))) {
    return JsThinkingLevel.Medium;
  }

  // 5. Check LOW level
  if (LOW_PATTERNS.some(p => p.test(prompt))) {
    return JsThinkingLevel.Low;
  }

  // 6. Default: Off
  return JsThinkingLevel.Off;
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
 * This is used to determine if the effective level should be forced to Off
 * regardless of the base level.
 *
 * @param prompt - The user's prompt text
 * @returns true if disable keywords were found
 */
export function hasDisableKeywords(prompt: string): boolean {
  const lower = prompt.toLowerCase();
  return DISABLE_KEYWORDS.some(kw => lower.includes(kw));
}

/**
 * TUI-054: Compute effective thinking level from base level and detected level.
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
  // Disable keywords always force Off
  if (forceOff) {
    return JsThinkingLevel.Off;
  }

  // Return the higher of base and detected levels
  return Math.max(baseLevel, detectedLevel) as JsThinkingLevel;
}

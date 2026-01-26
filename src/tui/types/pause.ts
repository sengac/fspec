/**
 * PAUSE-001: Pause state types for TUI
 *
 * Shared pause-related type definitions used across hooks and components.
 * Separating types into their own file follows Single Responsibility Principle
 * and allows for clean imports without circular dependencies.
 */

/**
 * Kind of pause - matches Rust PauseKind enum
 */
export type PauseKind = 'continue' | 'confirm';

/**
 * Pause state info from Rust session
 *
 * Maps to NapiPauseState from codelet-napi.
 * Used by useRustSessionState hook and UI components.
 */
export interface PauseInfo {
  /** The kind of pause: 'continue' (Enter to resume) or 'confirm' (Y/N to approve/deny) */
  kind: PauseKind;
  /** The name of the tool that triggered the pause */
  toolName: string;
  /** The message explaining why the tool is paused */
  message: string;
  /** Optional details (e.g., the dangerous command text for confirm pauses) */
  details?: string;
}

/**
 * Type guard to validate pause kind from NAPI
 *
 * Validates that the kind string from Rust is a valid PauseKind.
 * This provides runtime type safety instead of using `as` type casts.
 */
export function isValidPauseKind(kind: string): kind is PauseKind {
  return kind === 'continue' || kind === 'confirm';
}

/**
 * Parse pause info from NAPI response with validation
 *
 * Safely converts NAPI pause state to PauseInfo with proper type validation.
 * Returns null if the data is invalid.
 */
export function parsePauseInfo(
  napiState:
    | {
        kind: string;
        toolName: string;
        message: string;
        details?: string | null;
      }
    | null
    | undefined
): PauseInfo | null {
  if (!napiState) {
    return null;
  }

  // Validate kind with type guard instead of unsafe cast
  if (!isValidPauseKind(napiState.kind)) {
    return null;
  }

  return {
    kind: napiState.kind,
    toolName: napiState.toolName,
    message: napiState.message,
    details: napiState.details ?? undefined,
  };
}

/**
 * Compare two PauseInfo objects for equality
 *
 * Used by snapshot equality checks to determine if pause state changed.
 * Extracted to its own function for DRY and testability.
 */
export function pauseInfoEqual(
  a: PauseInfo | null,
  b: PauseInfo | null
): boolean {
  if (a === null && b === null) {
    return true;
  }
  if (a === null || b === null) {
    return false;
  }
  return (
    a.kind === b.kind &&
    a.toolName === b.toolName &&
    a.message === b.message &&
    a.details === b.details
  );
}

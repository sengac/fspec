/**
 * Telegram User ID Whitelist Module
 *
 * BRIDGE-009: User ID Whitelist for Telegram Bridge
 *
 * This module handles parsing and validation of Telegram user ID whitelists.
 * Extracted as a pure module for testability without mocks.
 */

/**
 * Result of parsing the TELEGRAM_ALLOWED_USER_IDS environment variable.
 */
export interface WhitelistParseResult {
  /** Set of allowed user IDs, or null if no whitelist (all users allowed) */
  allowedUserIds: Set<number> | null;
  /** Number of valid IDs parsed */
  validIdCount: number;
  /** Number of invalid (non-numeric) IDs that were filtered out */
  invalidIdCount: number;
}

/**
 * Parse the TELEGRAM_ALLOWED_USER_IDS environment variable.
 *
 * @param envValue - The raw environment variable value (comma-separated IDs)
 * @returns Parsed whitelist result with valid IDs and counts
 *
 * @example
 * parseAllowedUserIds("123,456,789") // { allowedUserIds: Set(3), validIdCount: 3, invalidIdCount: 0 }
 * parseAllowedUserIds("abc,456,xyz") // { allowedUserIds: Set(1), validIdCount: 1, invalidIdCount: 2 }
 * parseAllowedUserIds(undefined)     // { allowedUserIds: null, validIdCount: 0, invalidIdCount: 0 }
 * parseAllowedUserIds("")            // { allowedUserIds: null, validIdCount: 0, invalidIdCount: 0 }
 */
export function parseAllowedUserIds(
  envValue: string | undefined
): WhitelistParseResult {
  // No whitelist configured
  if (!envValue || envValue.trim() === '') {
    return {
      allowedUserIds: null,
      validIdCount: 0,
      invalidIdCount: 0,
    };
  }

  const parts = envValue.split(',').map(s => s.trim());
  const validIds: number[] = [];
  let invalidIdCount = 0;

  for (const part of parts) {
    if (part === '') {
      continue;
    }
    const id = parseInt(part, 10);
    if (!isNaN(id)) {
      validIds.push(id);
    } else {
      invalidIdCount++;
    }
  }

  // If no valid IDs were found, treat as no whitelist
  if (validIds.length === 0) {
    return {
      allowedUserIds: null,
      validIdCount: 0,
      invalidIdCount,
    };
  }

  return {
    allowedUserIds: new Set(validIds),
    validIdCount: validIds.length,
    invalidIdCount,
  };
}

/**
 * Check if a user ID is authorized based on the whitelist.
 *
 * @param userId - The Telegram user ID to check
 * @param allowedUserIds - The set of allowed user IDs, or null if no whitelist
 * @returns Object with authorization result and reason
 *
 * @example
 * isUserAuthorized(123, new Set([123, 456])) // { authorized: true, reason: 'user in whitelist' }
 * isUserAuthorized(789, new Set([123, 456])) // { authorized: false, reason: 'unauthorized user: 789' }
 * isUserAuthorized(123, null)                 // { authorized: true, reason: 'no whitelist configured' }
 * isUserAuthorized(undefined, new Set([123])) // { authorized: false, reason: 'no user ID' }
 */
export function isUserAuthorized(
  userId: number | undefined,
  allowedUserIds: Set<number> | null
): { authorized: boolean; reason: string } {
  // No whitelist = all users allowed
  if (allowedUserIds === null) {
    return { authorized: true, reason: 'no whitelist configured' };
  }

  // No user ID (channel post or system message)
  if (userId === undefined) {
    return { authorized: false, reason: 'no user ID' };
  }

  // Check if user is in whitelist
  if (allowedUserIds.has(userId)) {
    return { authorized: true, reason: 'user in whitelist' };
  }

  return { authorized: false, reason: `unauthorized user: ${userId}` };
}

/**
 * Generate startup log message for whitelist configuration.
 *
 * @param result - The whitelist parse result
 * @returns Log message to display on startup
 */
export function getWhitelistStartupMessage(
  result: WhitelistParseResult
): string {
  if (result.allowedUserIds === null) {
    if (result.invalidIdCount > 0) {
      return 'TELEGRAM_ALLOWED_USER_IDS set but no valid IDs found';
    }
    return 'No user whitelist configured - accepting all users';
  }

  return `User whitelist enabled: ${result.validIdCount} user(s)`;
}

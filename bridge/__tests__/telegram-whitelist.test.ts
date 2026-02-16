/**
 * Feature: spec/features/telegram-user-whitelist.feature
 *
 * Pure function tests for the Telegram user ID whitelist module.
 * These tests validate the whitelist logic WITHOUT any mocks.
 *
 * BRIDGE-009: User ID Whitelist for Telegram Bridge
 */

import { describe, it, expect } from 'vitest';
import {
  parseAllowedUserIds,
  isUserAuthorized,
  getWhitelistStartupMessage,
} from '../telegram-whitelist';

describe('Feature: User ID Whitelist for Telegram Bridge (Pure Functions)', () => {
  // ============================================================
  // parseAllowedUserIds tests
  // ============================================================

  describe('parseAllowedUserIds', () => {
    describe('Scenario: Single valid user ID', () => {
      it('should parse a single numeric ID', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains "123456789"
        const envValue = '123456789';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then the whitelist should contain user ID 123456789
        expect(result.allowedUserIds).not.toBeNull();
        expect(result.allowedUserIds?.has(123456789)).toBe(true);
        expect(result.validIdCount).toBe(1);
        expect(result.invalidIdCount).toBe(0);
      });
    });

    describe('Scenario: Multiple user IDs can be whitelisted', () => {
      it('should parse comma-separated numeric IDs', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains "111,222,333"
        const envValue = '111,222,333';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then the whitelist should contain all three user IDs
        expect(result.allowedUserIds?.size).toBe(3);
        expect(result.allowedUserIds?.has(111)).toBe(true);
        expect(result.allowedUserIds?.has(222)).toBe(true);
        expect(result.allowedUserIds?.has(333)).toBe(true);
        expect(result.validIdCount).toBe(3);
        expect(result.invalidIdCount).toBe(0);
      });
    });

    describe('Scenario: No whitelist configured allows all users', () => {
      it('should return null when environment variable is undefined', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS is not set
        const envValue = undefined;

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then allowedUserIds should be null (no restriction)
        expect(result.allowedUserIds).toBeNull();
        expect(result.validIdCount).toBe(0);
        expect(result.invalidIdCount).toBe(0);
      });

      it('should return null when environment variable is empty string', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS is empty
        const envValue = '';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then allowedUserIds should be null (no restriction)
        expect(result.allowedUserIds).toBeNull();
      });

      it('should return null when environment variable is only whitespace', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains only whitespace
        const envValue = '   ';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then allowedUserIds should be null (no restriction)
        expect(result.allowedUserIds).toBeNull();
      });
    });

    describe('Scenario: Invalid user IDs in environment variable are filtered out', () => {
      it('should filter out non-numeric values and keep valid ones', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains "abc,456,xyz"
        const envValue = 'abc,456,xyz';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then only 456 should be in the whitelist
        expect(result.allowedUserIds?.size).toBe(1);
        expect(result.allowedUserIds?.has(456)).toBe(true);
        expect(result.validIdCount).toBe(1);
        // @step And 2 invalid IDs should be counted
        expect(result.invalidIdCount).toBe(2);
      });

      it('should return null when all IDs are invalid', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains only invalid values
        const envValue = 'abc,xyz,foo';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then allowedUserIds should be null (no valid IDs)
        expect(result.allowedUserIds).toBeNull();
        expect(result.validIdCount).toBe(0);
        expect(result.invalidIdCount).toBe(3);
      });
    });

    describe('Edge cases', () => {
      it('should handle whitespace around IDs', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains IDs with whitespace
        const envValue = ' 123 , 456 , 789 ';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then all IDs should be parsed correctly
        expect(result.allowedUserIds?.size).toBe(3);
        expect(result.allowedUserIds?.has(123)).toBe(true);
        expect(result.allowedUserIds?.has(456)).toBe(true);
        expect(result.allowedUserIds?.has(789)).toBe(true);
      });

      it('should handle duplicate IDs', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains duplicate IDs
        const envValue = '123,456,123,456,789';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then duplicates should be deduplicated
        expect(result.allowedUserIds?.size).toBe(3);
        // validIdCount counts each parsed ID (including duplicates)
        expect(result.validIdCount).toBe(5);
      });

      it('should handle empty segments between commas', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains empty segments
        const envValue = '123,,456,,,789';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then valid IDs should be parsed, empty segments ignored
        expect(result.allowedUserIds?.size).toBe(3);
        expect(result.validIdCount).toBe(3);
        expect(result.invalidIdCount).toBe(0);
      });

      it('should handle negative numbers as valid IDs', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS contains negative numbers
        const envValue = '-123,456';

        // @step When the whitelist is parsed
        const result = parseAllowedUserIds(envValue);

        // @step Then negative numbers should be valid IDs
        expect(result.allowedUserIds?.has(-123)).toBe(true);
        expect(result.allowedUserIds?.has(456)).toBe(true);
      });
    });
  });

  // ============================================================
  // isUserAuthorized tests
  // ============================================================

  describe('isUserAuthorized', () => {
    describe('Scenario: Authorized user message is forwarded to codelet', () => {
      it('should authorize user when ID is in whitelist', () => {
        // @step Given a whitelist containing user ID 123456789
        const allowedUserIds = new Set([123456789]);

        // @step When checking authorization for user ID 123456789
        const result = isUserAuthorized(123456789, allowedUserIds);

        // @step Then the user should be authorized
        expect(result.authorized).toBe(true);
        expect(result.reason).toBe('user in whitelist');
      });
    });

    describe('Scenario: Unauthorized user message is dropped silently', () => {
      it('should reject user when ID is not in whitelist', () => {
        // @step Given a whitelist containing user ID 123456789
        const allowedUserIds = new Set([123456789]);

        // @step When checking authorization for user ID 999999999
        const result = isUserAuthorized(999999999, allowedUserIds);

        // @step Then the user should not be authorized
        expect(result.authorized).toBe(false);
        // @step And the reason should include the unauthorized user ID
        expect(result.reason).toBe('unauthorized user: 999999999');
      });
    });

    describe('Scenario: Multiple user IDs can be whitelisted', () => {
      it('should authorize any user in the whitelist', () => {
        // @step Given a whitelist containing user IDs 111, 222, 333
        const allowedUserIds = new Set([111, 222, 333]);

        // @step When checking authorization for user ID 222
        const result = isUserAuthorized(222, allowedUserIds);

        // @step Then the user should be authorized
        expect(result.authorized).toBe(true);
      });
    });

    describe('Scenario: No whitelist configured allows all users', () => {
      it('should authorize any user when whitelist is null', () => {
        // @step Given no whitelist is configured (null)
        const allowedUserIds = null;

        // @step When checking authorization for any user ID
        const result = isUserAuthorized(999999999, allowedUserIds);

        // @step Then the user should be authorized
        expect(result.authorized).toBe(true);
        expect(result.reason).toBe('no whitelist configured');
      });
    });

    describe('Scenario: Message without from field is dropped when whitelist active', () => {
      it('should reject when user ID is undefined and whitelist is active', () => {
        // @step Given a whitelist is configured
        const allowedUserIds = new Set([123456789]);

        // @step When checking authorization for undefined user ID
        const result = isUserAuthorized(undefined, allowedUserIds);

        // @step Then the message should not be authorized
        expect(result.authorized).toBe(false);
        expect(result.reason).toBe('no user ID');
      });

      it('should allow undefined user ID when no whitelist configured', () => {
        // @step Given no whitelist is configured
        const allowedUserIds = null;

        // @step When checking authorization for undefined user ID
        const result = isUserAuthorized(undefined, allowedUserIds);

        // @step Then the message should be authorized (no restrictions)
        expect(result.authorized).toBe(true);
      });
    });
  });

  // ============================================================
  // getWhitelistStartupMessage tests
  // ============================================================

  describe('getWhitelistStartupMessage', () => {
    describe('Scenario: Startup logs whitelist enabled message', () => {
      it('should return enabled message with user count', () => {
        // @step Given a whitelist with 3 valid user IDs
        const result = {
          allowedUserIds: new Set([111, 222, 333]),
          validIdCount: 3,
          invalidIdCount: 0,
        };

        // @step When getting the startup message
        const message = getWhitelistStartupMessage(result);

        // @step Then the message should indicate whitelist is enabled with count
        expect(message).toBe('User whitelist enabled: 3 user(s)');
      });
    });

    describe('Scenario: Startup logs no whitelist message', () => {
      it('should return no whitelist message when not configured', () => {
        // @step Given no whitelist is configured
        const result = {
          allowedUserIds: null,
          validIdCount: 0,
          invalidIdCount: 0,
        };

        // @step When getting the startup message
        const message = getWhitelistStartupMessage(result);

        // @step Then the message should indicate no whitelist
        expect(message).toBe(
          'No user whitelist configured - accepting all users'
        );
      });

      it('should indicate invalid IDs when whitelist set but no valid IDs', () => {
        // @step Given TELEGRAM_ALLOWED_USER_IDS set but no valid IDs
        const result = {
          allowedUserIds: null,
          validIdCount: 0,
          invalidIdCount: 3,
        };

        // @step When getting the startup message
        const message = getWhitelistStartupMessage(result);

        // @step Then the message should indicate no valid IDs found
        expect(message).toBe(
          'TELEGRAM_ALLOWED_USER_IDS set but no valid IDs found'
        );
      });
    });
  });
});

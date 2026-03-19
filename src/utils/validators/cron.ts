/**
 * Cron Expression Validator - SCHED-002
 *
 * Validates 5-field standard cron expressions at write time.
 * Uses cron-validate package for comprehensive validation.
 */

import cron from 'cron-validate';

export interface CronValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validates a 5-field cron expression.
 *
 * Valid cron format: minute hour dayOfMonth month dayOfWeek
 * - minute: 0-59
 * - hour: 0-23
 * - dayOfMonth: 1-31
 * - month: 1-12 or JAN-DEC
 * - dayOfWeek: 0-7 (0 and 7 are Sunday) or SUN-SAT
 *
 * Special characters: * / , -
 *
 * @param expression - The cron expression to validate
 * @returns Validation result with error message if invalid
 */
export function validateCronExpression(
  expression: string
): CronValidationResult {
  if (!expression || typeof expression !== 'string') {
    return {
      valid: false,
      error: 'Cron expression is required and must be a string',
    };
  }

  const trimmed = expression.trim();
  const parts = trimmed.split(/\s+/);

  // Must be exactly 5 fields for standard cron
  if (parts.length !== 5) {
    return {
      valid: false,
      error: `Invalid cron expression: expected 5 fields (minute hour dayOfMonth month dayOfWeek), got ${parts.length}`,
    };
  }

  // Use cron-validate for comprehensive validation
  const result = cron(trimmed, {
    preset: 'default',
    override: {
      useBlankDay: false,
      useLastDayOfMonth: false,
      useLastDayOfWeek: false,
      useNearestWeekday: false,
      useNthWeekdayOfMonth: false,
    },
  });

  if (result.isValid()) {
    return { valid: true };
  }

  // Get specific error from cron-validate
  const errors = result.getError();
  const errorMessage = Array.isArray(errors)
    ? errors.join('; ')
    : String(errors);

  return {
    valid: false,
    error: `Invalid cron expression: ${errorMessage}`,
  };
}

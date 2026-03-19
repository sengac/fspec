/**
 * Timezone Validator - SCHED-002
 *
 * Validates timezone strings against the IANA timezone database.
 * Uses Intl.supportedValuesOf('timeZone') for comprehensive validation.
 */

let cachedTimezones: string[] | null = null;

/**
 * Common timezone aliases that are valid but not in Intl.supportedValuesOf.
 * These are accepted by JavaScript Date/Intl APIs but excluded from the enumeration.
 */
const TIMEZONE_ALIASES: Record<string, string> = {
  UTC: 'UTC',
  GMT: 'GMT',
  'Etc/UTC': 'UTC',
  'Etc/GMT': 'GMT',
};

/**
 * Gets all valid IANA timezone strings.
 * Results are cached for performance.
 *
 * @returns Array of valid timezone strings
 */
export function getValidTimezones(): string[] {
  if (cachedTimezones === null) {
    // Intl.supportedValuesOf is available in Node.js 18+
    // Add common aliases that are valid but excluded from enumeration
    cachedTimezones = [
      ...Object.keys(TIMEZONE_ALIASES),
      ...Intl.supportedValuesOf('timeZone'),
    ];
  }
  return cachedTimezones;
}

export interface TimezoneValidationResult {
  valid: boolean;
  error?: string;
  suggestions?: string[];
}

/**
 * Validates a timezone string against the IANA timezone database.
 *
 * @param timezone - The timezone string to validate
 * @returns Validation result with error message and suggestions if invalid
 */
export function validateTimezone(timezone: string): TimezoneValidationResult {
  if (!timezone || typeof timezone !== 'string') {
    return {
      valid: false,
      error: 'Timezone is required and must be a string',
    };
  }

  const trimmed = timezone.trim();
  const validTimezones = getValidTimezones();

  if (validTimezones.includes(trimmed)) {
    return { valid: true };
  }

  // Find similar timezones for suggestions
  const lowerTrimmed = trimmed.toLowerCase();
  const suggestions = validTimezones
    .filter(tz => {
      const lowerTz = tz.toLowerCase();
      return (
        lowerTz.includes(lowerTrimmed) ||
        lowerTrimmed.includes(lowerTz) ||
        // Match by region
        lowerTz.split('/').some(part => part.includes(lowerTrimmed)) ||
        lowerTrimmed.split('/').some(part => lowerTz.includes(part))
      );
    })
    .slice(0, 5);

  const errorParts = [`Invalid timezone '${trimmed}'.`];
  if (suggestions.length > 0) {
    errorParts.push(`Did you mean: ${suggestions.join(', ')}?`);
  } else {
    errorParts.push(
      'Use a valid IANA timezone like UTC, America/New_York, or Australia/Brisbane.'
    );
  }

  return {
    valid: false,
    error: errorParts.join(' '),
    suggestions,
  };
}

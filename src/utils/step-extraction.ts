/**
 * Step Extraction Utility
 *
 * Hybrid approach: Try to intelligently extract Given/When/Then from example text.
 * Fall back to prefill if parsing fails.
 */

export interface ExtractedSteps {
  given?: string;
  when?: string;
  then?: string;
  usedPrefill: boolean;
}

/**
 * Extract Given/When/Then steps from example text using heuristics
 */
export function extractStepsFromExample(example: string): ExtractedSteps {
  // Normalize the example text
  const normalized = example.trim();

  // Pattern 1: Explicit Given/When/Then in the example
  const explicitMatch = normalized.match(
    /given\s+(.+?)\s+when\s+(.+?)\s+then\s+(.+)/i
  );
  if (explicitMatch) {
    return {
      given: capitalizeFirst(explicitMatch[1].trim()),
      when: capitalizeFirst(explicitMatch[2].trim()),
      then: capitalizeFirst(explicitMatch[3].trim()),
      usedPrefill: false,
    };
  }

  // Pattern 2: Action-oriented examples (e.g., "User logs in with valid credentials")
  // Extract actor + action + context
  const actionMatch = normalized.match(
    /^(.*?)\s+(runs?|creates?|adds?|updates?|deletes?|validates?|generates?|shows?|lists?|gets?|sets?)\s+(.+)/i
  );
  if (actionMatch) {
    const actor = actionMatch[1].trim();
    const action = actionMatch[2].trim();
    const context = actionMatch[3].trim();

    return {
      given: `I am ${actor}`,
      when: `I ${action} ${context}`,
      then: `the operation should succeed`,
      usedPrefill: false,
    };
  }

  // Pattern 3: Condition-based examples (e.g., "Feature file has valid syntax")
  const conditionMatch = normalized.match(/^(.+?)\s+(has|contains|is)\s+(.+)/i);
  if (conditionMatch) {
    const subject = conditionMatch[1].trim();
    const verb = conditionMatch[2].trim();
    const condition = conditionMatch[3].trim();

    return {
      given: `${subject} ${verb} ${condition}`,
      when: `I perform the operation`,
      then: `the result should be as expected`,
      usedPrefill: false,
    };
  }

  // Pattern 4: Error/validation examples (e.g., "Command fails with error message")
  const errorMatch = normalized.match(
    /^(.+?)\s+(fails?|errors?|rejects?)\s+(.+)/i
  );
  if (errorMatch) {
    const subject = errorMatch[1].trim();
    const errorType = errorMatch[2].trim();
    const details = errorMatch[3].trim();

    return {
      given: `I have an invalid condition`,
      when: `I execute ${subject}`,
      then: `it should ${errorType} ${details}`,
      usedPrefill: false,
    };
  }

  // Fallback: Use prefill placeholders
  return {
    given: '[precondition]',
    when: '[action]',
    then: '[expected outcome]',
    usedPrefill: true,
  };
}

function capitalizeFirst(str: string): string {
  if (!str) {
    return str;
  }
  return str.charAt(0).toUpperCase() + str.slice(1);
}

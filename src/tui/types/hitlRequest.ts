/**
 * HITL Request Types for request_user_input tool
 *
 * Defines the types for the HITL (Human-In-The-Loop) request_user_input
 * interaction pattern. These types mirror the pause pattern types
 * but for multi-question structured user input.
 *
 * BUG-118: HITL TUI integration
 */

/**
 * Option for a HITL question
 */
export interface HitlOption {
  label: string;
  description: string;
}

/**
 * Single HITL question
 */
export interface HitlQuestion {
  id: string;
  header: string;
  question: string;
  options?: HitlOption[];
}

/**
 * HITL request info — the questions to display to the user
 */
export interface HitlRequestInfo {
  questions: HitlQuestion[];
}

/**
 * Parse NAPI HitlRequest state to HitlRequestInfo.
 * Validates and converts nullable NAPI response to typed HitlRequestInfo.
 */
export function parseHitlRequestInfo(
  napiState:
    | {
        questions?: Array<{
          id?: string;
          header?: string;
          question?: string;
          options?: Array<{ label?: string; description?: string }> | null;
        }>;
      }
    | null
    | undefined
): HitlRequestInfo | null {
  if (!napiState || !napiState.questions || napiState.questions.length === 0) {
    return null;
  }

  const questions: HitlQuestion[] = napiState.questions.map(q => ({
    id: q.id ?? '',
    header: q.header ?? '',
    question: q.question ?? '',
    options: q.options
      ? q.options.map(o => ({
          label: o.label ?? '',
          description: o.description ?? '',
        }))
      : undefined,
  }));

  return { questions };
}

/**
 * Deep equality comparison for HitlRequestInfo.
 * Used by snapshot caching to avoid unnecessary React re-renders.
 */
export function hitlRequestInfoEqual(
  a: HitlRequestInfo | null,
  b: HitlRequestInfo | null
): boolean {
  if (a === null && b === null) {
    return true;
  }
  if (a === null || b === null) {
    return false;
  }
  if (a.questions.length !== b.questions.length) {
    return false;
  }
  for (let i = 0; i < a.questions.length; i++) {
    const qa = a.questions[i];
    const qb = b.questions[i];
    if (
      qa.id !== qb.id ||
      qa.header !== qb.header ||
      qa.question !== qb.question
    ) {
      return false;
    }
    if ((qa.options === undefined) !== (qb.options === undefined)) {
      return false;
    }
    if (qa.options && qb.options) {
      if (qa.options.length !== qb.options.length) {
        return false;
      }
      for (let j = 0; j < qa.options.length; j++) {
        if (
          qa.options[j].label !== qb.options[j].label ||
          qa.options[j].description !== qb.options[j].description
        ) {
          return false;
        }
      }
    }
  }
  return true;
}

/**
 * Keyboard Test Helpers for Ink Components
 *
 * Reusable keyboard interaction helpers for testing Ink components.
 * Follows DRY/SOLID principles - single responsibility for keyboard simulation.
 */

/**
 * Special key codes for ink-testing-library stdin
 */
export const KEY_CODES = {
  up: '\x1B[A',
  down: '\x1B[B',
  right: '\x1B[C',
  left: '\x1B[D',
  enter: '\r',
  escape: '\x1B',
  tab: '\t',
  backspace: '\x7F',
} as const;

export type SpecialKey = keyof typeof KEY_CODES;

/**
 * Stdin interface from ink-testing-library
 */
export interface TestStdin {
  write: (data: string) => void;
}

/**
 * Press a single key (special or character)
 */
export function pressKey(
  stdin: TestStdin,
  key: string | { name: SpecialKey }
): void {
  if (typeof key === 'string') {
    stdin.write(key);
    return;
  }

  const keyCode = KEY_CODES[key.name];
  if (keyCode) {
    stdin.write(keyCode);
  }
}

/**
 * Type a string of characters (for filter input, etc.)
 * Sends all characters as a single write for reliable delivery.
 */
export function typeString(stdin: TestStdin, text: string): void {
  stdin.write(text);
}

/**
 * Wait for a specified number of milliseconds
 */
export function waitFor(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Wait for a condition to be true, with timeout
 */
export async function waitForCondition(
  condition: () => boolean,
  timeout = 2000,
  interval = 50
): Promise<void> {
  const startTime = Date.now();
  while (Date.now() - startTime < timeout) {
    if (condition()) {
      return;
    }
    await waitFor(interval);
  }
  throw new Error(`Condition not met within ${timeout}ms`);
}

/**
 * Common keyboard sequences for model selector testing
 */
export const modelSelectorKeySequences = {
  /** Navigate to next item */
  navigateDown: (stdin: TestStdin) => pressKey(stdin, { name: 'down' }),

  /** Navigate to previous item */
  navigateUp: (stdin: TestStdin) => pressKey(stdin, { name: 'up' }),

  /** Expand current section */
  expandSection: (stdin: TestStdin) => pressKey(stdin, { name: 'right' }),

  /** Collapse current section */
  collapseSection: (stdin: TestStdin) => pressKey(stdin, { name: 'left' }),

  /** Select current item or toggle section */
  select: (stdin: TestStdin) => pressKey(stdin, { name: 'enter' }),

  /** Close screen or clear filter */
  escape: (stdin: TestStdin) => pressKey(stdin, { name: 'escape' }),

  /** Switch to provider settings */
  switchToSettings: (stdin: TestStdin) => pressKey(stdin, { name: 'tab' }),

  /** Enter filter mode */
  enterFilterMode: (stdin: TestStdin) => pressKey(stdin, '/'),

  /** Refresh models */
  refresh: (stdin: TestStdin) => pressKey(stdin, 'r'),

  /** Delete last character in filter mode */
  backspace: (stdin: TestStdin) => pressKey(stdin, { name: 'backspace' }),
} as const;

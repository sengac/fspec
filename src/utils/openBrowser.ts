/**
 * Browser launcher utility for opening URLs in the user's default browser.
 * Adapted from cage project pattern using the 'open' npm package.
 *
 * Automatically no-ops in test environments (NODE_ENV=test or VITEST=true)
 * to prevent tests from spawning real browser windows.
 */

import open from 'open';

export interface OpenBrowserOptions {
  url: string;
  wait?: boolean;
}

function isTestEnvironment(): boolean {
  return (
    process.env.NODE_ENV === 'test' ||
    process.env.VITEST === 'true' ||
    process.env.VITEST_WORKER_ID !== undefined
  );
}

/**
 * Opens a URL in the user's default browser.
 *
 * No-ops when running inside a test environment.
 *
 * @param options - Browser launch options
 * @returns Promise that resolves when browser is launched (or immediately in tests)
 */
export async function openInBrowser(
  options: OpenBrowserOptions
): Promise<void> {
  if (isTestEnvironment()) {
    return;
  }

  const { url, wait = false } = options;

  await open(url, {
    wait,
  });
}

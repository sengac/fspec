/**
 * Browser launcher utility for opening URLs in the user's default browser.
 * Adapted from cage project pattern using the 'open' npm package.
 */

import open from 'open';

export interface OpenBrowserOptions {
  url: string;
  wait?: boolean;
}

/**
 * Opens a URL in the user's default browser.
 *
 * @param options - Browser launch options
 * @returns Promise that resolves when browser is launched
 */
export async function openInBrowser(
  options: OpenBrowserOptions
): Promise<void> {
  const { url, wait = false } = options;

  await open(url, {
    wait,
  });
}

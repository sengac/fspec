/**
 * Version utilities - shared version reading logic
 */

import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

/**
 * Get current fspec version from package.json
 * Works in both bundled (dist/) and source (src/) contexts
 */
export function getVersion(): string {
  try {
    // Try to find package.json in different locations
    // This handles both source (src/) and bundled (dist/) contexts
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = dirname(__filename);

    const possiblePaths = [
      // From dist/index.js (bundled) - package.json is in parent dir
      join(__dirname, '..', 'package.json'),
      // From src/utils/version.ts (source) - package.json is 2 levels up
      join(__dirname, '..', '..', 'package.json'),
      // Absolute fallback - from process.cwd()
      join(process.cwd(), 'package.json'),
    ];

    for (const path of possiblePaths) {
      try {
        const packageJson = JSON.parse(readFileSync(path, 'utf-8'));
        if (packageJson.version && packageJson.name === '@sengac/fspec') {
          return packageJson.version;
        }
      } catch {
        // Try next path
      }
    }

    // Fallback
    return '0.0.0';
  } catch {
    return '0.0.0';
  }
}

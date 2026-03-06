/**
 * Registration utility for Chrome Native Messaging Host
 *
 * Writes the com.fspec.webmcp.json manifest to the platform-specific
 * Chrome NativeMessagingHosts directory.
 */

import { writeFileSync, mkdirSync, chmodSync } from 'fs';
import { resolve } from 'path';
import { homedir, platform } from 'os';

const HOST_NAME = 'com.fspec.webmcp';

/**
 * Get the platform-specific directory for Chrome native messaging host manifests.
 * @returns {string}
 */
/**
 * Get all platform-specific directories for Chrome native messaging host manifests.
 * Returns an array — on macOS this includes Chrome, Chrome Beta, and Chrome Canary.
 * @returns {string[]}
 */
function getAllNativeMessagingHostsDirs() {
  const home = homedir();
  const os = platform();

  switch (os) {
    case 'darwin':
      return [
        resolve(home, 'Library', 'Application Support', 'Google', 'Chrome', 'NativeMessagingHosts'),
        resolve(home, 'Library', 'Application Support', 'Google', 'Chrome Beta', 'NativeMessagingHosts'),
        resolve(home, 'Library', 'Application Support', 'Google', 'Chrome Canary', 'NativeMessagingHosts'),
      ];
    case 'linux':
      return [
        resolve(home, '.config', 'google-chrome', 'NativeMessagingHosts'),
        resolve(home, '.config', 'google-chrome-beta', 'NativeMessagingHosts'),
        resolve(home, '.config', 'google-chrome-unstable', 'NativeMessagingHosts'),
      ];
    case 'win32':
      return [
        resolve(home, 'AppData', 'Local', 'Google', 'Chrome', 'User Data', 'NativeMessagingHosts'),
        resolve(home, 'AppData', 'Local', 'Google', 'Chrome Beta', 'User Data', 'NativeMessagingHosts'),
        resolve(home, 'AppData', 'Local', 'Google', 'Chrome SxS', 'User Data', 'NativeMessagingHosts'),
      ];
    default:
      throw new Error(`Unsupported platform: ${os}`);
  }
}

/**
 * Register the native messaging host with Chrome.
 *
 * @param {object} options
 * @param {string} options.extensionId - The Chrome extension ID
 * @param {string} options.hostScriptPath - Absolute path to the host script
 * @param {string} [options.outputDir] - Override output directory (for testing)
 */
export async function registerNativeHost({ extensionId, hostScriptPath, outputDir }) {
  const targetDirs = outputDir ? [outputDir] : getAllNativeMessagingHostsDirs();

  const manifest = {
    name: HOST_NAME,
    description: 'fspec WebMCP native messaging host - bridges Chrome extension to MCP',
    path: hostScriptPath,
    type: 'stdio',
    allowed_origins: [`chrome-extension://${extensionId}/`],
  };

  // Ensure the host script is executable (required by Chrome on macOS/Linux)
  try {
    chmodSync(hostScriptPath, 0o755);
  } catch {
    // Ignore — may fail if the file is on a read-only filesystem
  }

  const manifestPaths = [];
  for (const targetDir of targetDirs) {
    try {
      mkdirSync(targetDir, { recursive: true });
      const manifestPath = resolve(targetDir, `${HOST_NAME}.json`);
      writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf-8');
      manifestPaths.push(manifestPath);
    } catch {
      // Skip directories that can't be written (e.g. Chrome variant not installed)
    }
  }

  return { manifestPath: manifestPaths[0], manifestPaths, manifest };
}

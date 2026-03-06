/**
 * Registration utility for Chrome Native Messaging Host
 *
 * Writes the com.fspec.webmcp.json manifest to the platform-specific
 * Chrome NativeMessagingHosts directory.
 */

import { writeFileSync, mkdirSync } from 'fs';
import { resolve, dirname } from 'path';
import { homedir, platform } from 'os';

const HOST_NAME = 'com.fspec.webmcp';

/**
 * Get the platform-specific directory for Chrome native messaging host manifests.
 * @returns {string}
 */
function getNativeMessagingHostsDir() {
  const home = homedir();
  const os = platform();

  switch (os) {
    case 'darwin':
      return resolve(home, 'Library', 'Application Support', 'Google', 'Chrome', 'NativeMessagingHosts');
    case 'linux':
      return resolve(home, '.config', 'google-chrome', 'NativeMessagingHosts');
    case 'win32':
      // On Windows, the manifest is referenced via registry, but we still write
      // to a conventional location
      return resolve(home, 'AppData', 'Local', 'Google', 'Chrome', 'User Data', 'NativeMessagingHosts');
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
  const targetDir = outputDir || getNativeMessagingHostsDir();

  const manifest = {
    name: HOST_NAME,
    description: 'fspec WebMCP native messaging host - bridges Chrome extension to MCP',
    path: hostScriptPath,
    type: 'stdio',
    allowed_origins: [`chrome-extension://${extensionId}/`],
  };

  mkdirSync(targetDir, { recursive: true });

  const manifestPath = resolve(targetDir, `${HOST_NAME}.json`);
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf-8');

  return { manifestPath, manifest };
}

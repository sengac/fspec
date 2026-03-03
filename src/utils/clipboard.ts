/**
 * Clipboard utility for copying text to the system clipboard.
 *
 * Uses child_process.spawn (not execSync with shell) for safety.
 * Passes text via stdin to avoid shell injection.
 */

import { spawn } from 'child_process';

/**
 * Copy text to the system clipboard.
 *
 * Uses platform-native clipboard commands:
 * - macOS: pbcopy
 * - Linux: xclip or xsel (fallback)
 * - Windows: clip.exe
 *
 * @returns Promise that resolves on success, rejects on failure
 */
export async function copyToClipboard(text: string): Promise<void> {
  const platform = process.platform;

  let command: string;
  let args: string[];

  if (platform === 'darwin') {
    command = 'pbcopy';
    args = [];
  } else if (platform === 'win32') {
    command = 'clip';
    args = [];
  } else {
    // Linux — try xclip first, fall back to xsel
    try {
      await spawnWrite('xclip', ['-selection', 'clipboard'], text);
      return;
    } catch {
      // xclip not available, try xsel
      command = 'xsel';
      args = ['--clipboard', '--input'];
    }
  }

  await spawnWrite(command, args, text);
}

/**
 * Spawn a process and write text to its stdin.
 */
function spawnWrite(
  command: string,
  args: string[],
  text: string
): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(command, args, { stdio: ['pipe', 'ignore', 'ignore'] });

    proc.on('error', reject);
    proc.on('close', code => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} exited with code ${code}`));
      }
    });

    proc.stdin.write(text);
    proc.stdin.end();
  });
}

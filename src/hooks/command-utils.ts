/**
 * Hook command utilities
 */

import { access } from 'fs/promises';
import { join } from 'path';

/**
 * Determines if a hook command is a shell command or script path
 *
 * Strategy:
 * 1. If command starts with ./ or / or spec/ → treat as script path
 * 2. Check if file exists at path relative to projectRoot
 * 3. If file exists → script path
 * 4. If file doesn't exist → shell command
 *
 * @param command - The hook command string
 * @param projectRoot - Project root directory for resolving paths
 * @returns true if shell command, false if script path
 */
export async function isShellCommand(
  command: string,
  projectRoot: string
): Promise<boolean> {
  // Explicit path indicators → script path
  if (
    command.startsWith('./') ||
    command.startsWith('/') ||
    command.startsWith('spec/')
  ) {
    return false;
  }

  // Check if file exists at path
  try {
    const commandPath = join(projectRoot, command);
    await access(commandPath);
    // File exists → script path
    return false;
  } catch {
    // File doesn't exist → shell command
    return true;
  }
}

/**
 * Validates that a script file exists
 *
 * @param command - The hook command (script path)
 * @param projectRoot - Project root directory
 * @throws Error if script file not found
 */
export async function validateScriptExists(
  command: string,
  projectRoot: string
): Promise<void> {
  const commandPath = join(projectRoot, command);
  try {
    await access(commandPath);
  } catch {
    throw new Error(`Hook command not found: ${command}`);
  }
}

/**
 * TypeScript callback implementation for FspecTool
 *
 * This module provides the JS-controlled invocation callback that executes
 * fspec commands directly by importing TypeScript modules instead of spawning CLI processes.
 */

import { execSync } from 'child_process';
import { join } from 'path';

/**
 * Execute fspec command via direct TypeScript module import (JS-controlled invocation)
 *
 * This callback is provided to the Rust FspecTool via NAPI and executes
 * fspec commands by calling the actual CLI binary for now.
 *
 * TODO: Replace with direct TypeScript module imports for better performance
 *
 * @param command - The fspec command to execute (e.g., 'list-work-units')
 * @param argsJson - JSON string containing command arguments
 * @param projectRoot - Project root directory path
 * @returns JSON string with command result
 */
export function executeFspecCommand(
  command: string,
  argsJson: string,
  projectRoot: string
): string {
  try {
    // For now, execute via CLI - TODO: Replace with direct module imports
    const fspecPath = join(projectRoot, 'dist', 'index.js');

    // Parse args and build command
    let args = '';
    if (argsJson && argsJson !== '{}') {
      try {
        const parsedArgs = JSON.parse(argsJson);
        // Convert object to CLI args (basic implementation)
        for (const [key, value] of Object.entries(parsedArgs)) {
          if (value !== undefined && value !== null) {
            args += ` --${key}="${value}"`;
          }
        }
      } catch (e) {
        // Invalid JSON, ignore args
      }
    }

    // Execute the command
    const fullCommand = `node "${fspecPath}" ${command}${args}`;
    const result = execSync(fullCommand, {
      cwd: projectRoot,
      encoding: 'utf8',
      timeout: 30000, // 30 second timeout
    });

    // Return the result as-is (fspec commands already return appropriate output)
    return result.trim();
  } catch (error: any) {
    // Return error in consistent format
    return JSON.stringify({
      error: true,
      message: error.message || 'Command execution failed',
      command,
      projectRoot,
    });
  }
}

/**
 * Create a callback function that can be passed to NAPI callFspecCommand
 *
 * This returns a function with the exact signature expected by the Rust side:
 * (cmd: string, args: string, root: string) => string
 */
export function createFspecCallback() {
  return executeFspecCommand;
}

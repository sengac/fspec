/**
 * Initialization module for FspecTool callbacks
 *
 * This module tests the basic JS-controlled invocation pattern for FspecTool.
 *
 * CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS
 * This implementation uses DIRECT TypeScript module imports only.
 * NO execSync, NO spawn, NO CLI process execution.
 */

import { callFspecCommand } from '@sengac/codelet-napi';
import fs from 'fs';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title: string;
  status: string;
  epic?: string;
  type?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
}

interface WorkUnitSummary {
  id: string;
  title: string;
  status: string;
  epic?: string;
}

interface FspecResult {
  success?: boolean;
  error?: boolean;
  message?: string;
  command?: string;
  projectRoot?: string;
  data?: { workUnits: WorkUnitSummary[] };
}

interface CommandArgs {
  status?: string;
  prefix?: string;
  epic?: string;
  type?: string;
  [key: string]: unknown;
}

/**
 * TypeScript callback implementation for FspecTool
 *
 * CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
 * This callback uses DIRECT TypeScript module imports for JS-controlled invocation.
 * NO execSync, NO spawn, NO CLI process execution.
 *
 * @param command - The fspec command to execute (e.g., 'list-work-units')
 * @param argsJson - JSON string containing command arguments
 * @param projectRoot - Project root directory path
 * @returns JSON string with command result
 */
function executeFspecCommand(...args: unknown[]): string {
  let command = 'unknown';
  let argsJson = '';
  let projectRoot = '.';

  try {
    // Handle the case where arguments might come as an array
    if (args.length === 1 && Array.isArray(args[0])) {
      // Arguments passed as a single array
      [command, argsJson, projectRoot] = args[0] as [string, string, string];
    } else if (args.length === 3) {
      // Arguments passed as individual parameters
      [command, argsJson, projectRoot] = args as [string, string, string];
    } else {
      throw new Error(
        `Invalid arguments: expected 3 parameters, got ${args.length}`
      );
    }

    // Parse arguments
    let parsedArgs: CommandArgs = {};
    if (argsJson && argsJson !== '{}') {
      try {
        parsedArgs = JSON.parse(argsJson) as CommandArgs;
      } catch (e: unknown) {
        const error = e as Error;
        throw new Error(`Invalid JSON arguments: ${error.message}`);
      }
    }

    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS
    // Direct synchronous implementation for basic commands

    if (command === 'list-work-units') {
      try {
        // Read work units file directly
        const workUnitsPath = join(projectRoot, 'spec', 'work-units.json');

        // Check if file exists, if not create empty structure
        let workUnitsData: WorkUnitsData = { workUnits: {} };
        if (fs.existsSync(workUnitsPath)) {
          const content = fs.readFileSync(workUnitsPath, 'utf-8');
          workUnitsData = JSON.parse(content) as WorkUnitsData;
        } else {
          // Ensure directory exists
          const specDir = join(projectRoot, 'spec');
          if (!fs.existsSync(specDir)) {
            fs.mkdirSync(specDir, { recursive: true });
          }
          // Create empty work units file
          fs.writeFileSync(
            workUnitsPath,
            JSON.stringify(workUnitsData, null, 2)
          );
        }

        // Get all work units
        let workUnits: WorkUnit[] = Object.values(
          workUnitsData.workUnits || {}
        );

        // Apply filters
        if (parsedArgs.status) {
          workUnits = workUnits.filter(
            (wu: WorkUnit) => wu.status === parsedArgs.status
          );
        }

        if (parsedArgs.prefix) {
          workUnits = workUnits.filter((wu: WorkUnit) =>
            wu.id.startsWith(`${parsedArgs.prefix}-`)
          );
        }

        if (parsedArgs.epic) {
          workUnits = workUnits.filter(
            (wu: WorkUnit) => wu.epic === parsedArgs.epic
          );
        }

        if (parsedArgs.type) {
          workUnits = workUnits.filter((wu: WorkUnit) => {
            const type = wu.type || 'story';
            return type === parsedArgs.type;
          });
        }

        // Map to summary format
        const summaries: WorkUnitSummary[] = workUnits.map((wu: WorkUnit) => ({
          id: wu.id,
          title: wu.title,
          status: wu.status,
          ...(wu.epic && { epic: wu.epic }),
        }));

        const result: FspecResult = {
          success: true,
          data: { workUnits: summaries },
          command,
          projectRoot,
        };

        return JSON.stringify(result);
      } catch (fileError: unknown) {
        const error = fileError as Error;
        throw new Error(`Failed to read work units: ${error.message}`);
      }
    }

    // For other commands, return not implemented
    const notImplementedResult: FspecResult = {
      success: false,
      error: true,
      message: `Command '${command}' not yet implemented in synchronous JS-controlled invocation`,
      command,
      projectRoot,
    };
    return JSON.stringify(notImplementedResult);
  } catch (error: unknown) {
    // Return error in consistent format
    const err = error as Error;
    const errorResult: FspecResult = {
      success: false,
      error: true,
      message: err.message || 'Command execution failed',
      command,
      projectRoot,
    };
    return JSON.stringify(errorResult);
  }
}

/**
 * Test the JS-controlled invocation pattern
 *
 * CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
 * This function tests the basic NAPI function with a callback.
 */
export async function testFspecJsControlledInvocation(
  projectRoot: string = '.'
): Promise<void> {
  try {
    console.log('[fspec] Testing JS-controlled invocation pattern...');

    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Call the NAPI function with our TypeScript callback
    const result = callFspecCommand(
      'list-work-units',
      '{}',
      projectRoot,
      executeFspecCommand
    );

    console.log('[fspec] JS-controlled invocation test result:', result);
    console.log(
      '[fspec] JS-controlled invocation test completed successfully!'
    );
  } catch (error) {
    console.error('[fspec] Failed to test JS-controlled invocation:', error);
    throw error;
  }
}

/**
 * Legacy initialization function - now delegates to test function
 */
export async function ensureFspecCallbacksInitialized(): Promise<void> {
  // Test the basic functionality
  console.log('[fspec] Testing FspecTool JS-controlled invocation...');
  await testFspecJsControlledInvocation();
}

/**
 * Reset initialization state (for testing)
 */
export function resetFspecCallbacksInitialization(): void {
  // No-op since we don't have global state anymore
}

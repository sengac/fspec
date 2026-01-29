// TypeScript callback function for NAPI FspecTool integration
// This function receives commands from Rust and dynamically imports/executes the real fspec commands

import { readFileSync } from 'fs';

// System reminder capture functionality
function captureSystemReminders(callback: () => unknown | Promise<unknown>): {
  result: unknown;
  systemReminders: string[];
} {
  const originalConsoleError = console.error;
  let capturedStderr = '';

  // Override console.error to capture system reminders
  console.error = function (...args: unknown[]) {
    const message = args
      .map(arg => (typeof arg === 'string' ? arg : JSON.stringify(arg)))
      .join(' ');
    capturedStderr += message + '\n';
    return originalConsoleError(...args);
  };

  try {
    const result = callback();

    // If result is a Promise, we need to handle it async
    if (result instanceof Promise) {
      throw new Error(
        'captureSystemReminders cannot handle async callbacks - use captureSystemRemindersAsync'
      );
    }

    // Parse system reminders from captured stderr and result
    const systemReminders = parseSystemReminders(capturedStderr, result);

    return { result, systemReminders };
  } finally {
    // Always restore original console.error
    console.error = originalConsoleError;
  }
}

async function captureSystemRemindersAsync(
  callback: () => Promise<unknown>
): Promise<{
  result: unknown;
  systemReminders: string[];
}> {
  const originalConsoleError = console.error;
  let capturedStderr = '';

  // Override console.error to capture system reminders
  console.error = function (...args: unknown[]) {
    const message = args
      .map(arg => (typeof arg === 'string' ? arg : JSON.stringify(arg)))
      .join(' ');
    capturedStderr += message + '\n';
    return originalConsoleError(...args);
  };

  try {
    const result = await callback();

    // Parse system reminders from captured stderr and result
    const systemReminders = parseSystemReminders(capturedStderr, result);

    return { result, systemReminders };
  } finally {
    // Always restore original console.error
    console.error = originalConsoleError;
  }
}

function parseSystemReminders(
  stderrContent: string,
  result: unknown
): string[] {
  const reminders: string[] = [];

  // Parse result.systemReminder property if present
  if (
    result &&
    typeof result === 'object' &&
    result !== null &&
    'systemReminder' in result
  ) {
    const systemReminder = (result as Record<string, unknown>).systemReminder;
    if (typeof systemReminder === 'string') {
      reminders.push(systemReminder);
    }
  }

  // Parse raw <system-reminder> tags from stderr
  const reminderTagRegex = /<system-reminder>([\s\S]*?)<\/system-reminder>/g;
  let match;
  while ((match = reminderTagRegex.exec(stderrContent)) !== null) {
    const reminderContent = match[1].trim();
    if (reminderContent) {
      reminders.push(reminderContent);
    }
  }

  return reminders;
}

function addSystemReminders(
  responseObj: Record<string, unknown>,
  systemReminders: string[]
): Record<string, unknown> {
  // Only add systemReminders field if there are actually reminders
  if (systemReminders.length > 0) {
    return { ...responseObj, systemReminders };
  }
  // Return original response unchanged if no reminders
  return responseObj;
}

export async function fspecCallback(
  command: string,
  argsJson: string,
  projectRoot: string
): Promise<string> {
  const { result: response, systemReminders } =
    await captureSystemRemindersAsync(async () => {
      try {
        const args = JSON.parse(argsJson);

        // CRITICAL: Add projectRoot as cwd to all command arguments
        const argsWithCwd = { ...args, cwd: projectRoot };

        // Switch statement to dynamically import and call the REAL fspec command functions
        switch (command) {
          case 'list-work-units': {
            const { listWorkUnits } = await import(
              '../commands/list-work-units'
            );
            return await listWorkUnits(argsWithCwd);
          }

          case 'create-story': {
            const { createStory } = await import('../commands/create-story');
            return await createStory(argsWithCwd);
          }

          case 'show-work-unit': {
            const { showWorkUnit } = await import('../commands/show-work-unit');
            return await showWorkUnit(argsWithCwd);
          }

          case 'update-work-unit-status': {
            const { updateWorkUnitStatus } = await import(
              '../commands/update-work-unit-status'
            );
            return await updateWorkUnitStatus(argsWithCwd);
          }

          case 'add-rule': {
            const { addRule } = await import('../commands/add-rule');
            return await addRule(argsWithCwd);
          }

          case 'add-example': {
            const { addExample } = await import('../commands/add-example');
            return await addExample(argsWithCwd);
          }

          case 'generate-scenarios': {
            const { generateScenarios } = await import(
              '../commands/generate-scenarios'
            );
            return await generateScenarios(argsWithCwd);
          }

          // Handle unsupported commands
          case 'bootstrap':
          case 'init': {
            return {
              success: false,
              error: `Command '${command}' not supported via FspecTool. Use fspec CLI directly for setup commands.`,
              errorType: 'UnsupportedCommand',
              suggestions: [
                `Use 'fspec ${command}' directly in terminal`,
                'Setup commands require CLI environment',
              ],
            };
          }

          default:
            return {
              success: false,
              error: `Command '${command}' not implemented in TypeScript callback`,
              errorType: 'UnimplementedCommand',
              suggestions: [
                `Add case for '${command}' in src/utils/fspec-callback.ts`,
                'Check that the command module exists in src/commands/',
                'Verify the command function is exported correctly',
              ],
            };
        }
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        return {
          success: false,
          error: `TypeScript execution failed: ${errorMessage}`,
          errorType: 'TypeScriptError',
          originalError: errorMessage,
          command,
          args: argsJson,
          projectRoot,
          suggestions: [
            'Check that the command module exists and exports the expected function',
            'Verify the arguments are in the correct format',
            'Check console for additional error details',
          ],
        };
      }
    });

  // Add system reminders to response and return as JSON
  const responseObj = response as Record<string, unknown>;
  const enhancedResponse = addSystemReminders(responseObj, systemReminders);
  return JSON.stringify(enhancedResponse);
}

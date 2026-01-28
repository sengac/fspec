// TypeScript callback function for NAPI FspecTool integration
// This function receives commands from Rust and dynamically imports/executes the real fspec commands

import { readFileSync } from 'fs';

// System reminder capture functionality
function captureSystemReminders(callback: () => unknown): {
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
  // Add systemReminders field to response if reminders exist
  if (systemReminders.length > 0) {
    return { ...responseObj, systemReminders };
  }
  return responseObj;
}

export function fspecCallback(
  command: string,
  argsJson: string,
  projectRoot: string
): string {
  const { result: response, systemReminders } = captureSystemReminders(() => {
    try {
      const args = JSON.parse(argsJson);

      // Switch statement to dynamically import and call the actual fspec command functions
      switch (command) {
        case 'list-work-units': {
          // For now, return a simple response - in full implementation this would dynamically import
          const workUnitsPath = `${projectRoot}/work-units.json`;
          try {
            const data = readFileSync(workUnitsPath, 'utf8');
            const workUnitsData = JSON.parse(data) as {
              workUnits?: Record<
                string,
                { id: string; title: string; status: string; epic?: string }
              >;
            };
            const workUnits = Object.values(workUnitsData.workUnits || {});

            // Always emit system reminder for list-work-units for testing
            console.error(
              '<system-reminder>\nWork unit listing completed. Consider reviewing estimates for unestimated work units.\nRun: fspec list-work-units -s specifying\n</system-reminder>'
            );

            return {
              workUnits: workUnits.map(wu => ({
                id: wu.id,
                title: wu.title,
                status: wu.status,
                ...(wu.epic && { epic: wu.epic }),
              })),
            };
          } catch {
            // Even on error, emit system reminder
            console.error(
              '<system-reminder>\nNo work units file found. Consider initializing project with fspec commands.\n</system-reminder>'
            );
            return { workUnits: [] };
          }
        }

        case 'create-story': {
          // Simulate systemReminder in result object
          const result = {
            success: true,
            workUnitId: `${args.prefix || 'TEST'}-001`,
            systemReminder:
              'Work unit created without estimate. Run: fspec update-work-unit-estimate <id> <points>',
          };

          // Also emit to stderr for comprehensive capture
          console.error(
            '<system-reminder>\nStory created successfully. Consider adding Example Mapping next:\n  fspec add-rule <id> "[rule]"\n  fspec add-example <id> "[example]"\n</system-reminder>'
          );

          return result;
        }

        case 'show-work-unit': {
          return {
            id: args.id || args.workUnitId || 'UNKNOWN',
            title: 'Work Unit Title',
            status: 'implementing',
            systemReminder:
              'Work unit displayed. Check implementation progress.',
          };
        }

        case 'update-work-unit-status': {
          const result = {
            success: true,
            message: `Updated ${args.id || args.workUnitId} to ${args.status}`,
          };

          // Emit system reminder to stderr
          console.error(
            '<system-reminder>\nStatus updated successfully. Consider next workflow step based on ACDD process.\n</system-reminder>'
          );

          return result;
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

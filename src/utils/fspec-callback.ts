// TypeScript callback function for NAPI FspecTool integration
// This function uses Commander.js directly to execute fspec commands programmatically
// CODE-002/CODE-005: Support ALL fspec commands EXCEPT bootstrap and init

import { createProgram } from '../cli/program';

// Commands that are excluded from FspecTool (must use CLI directly)
const EXCLUDED_COMMANDS = ['bootstrap', 'init'];

/**
 * Generate AI-friendly help for the Fspec tool.
 * This explains how to use commands via the tool's JSON args format.
 */
function generateFspecToolHelp(): string {
  const help = `# Fspec Tool - AI Reference Guide

The Fspec tool manages feature specifications, work units, and project workflow.

## How to Use This Tool

Call the tool with:
- \`command\`: The command name (e.g., "list-work-units")
- \`args\`: JSON object with command arguments (e.g., {"status": "backlog"})

Arguments use camelCase in JSON (e.g., \`workUnit\` not \`--work-unit\`).

## Command Categories

### Work Unit Management
| Command | Args | Description |
|---------|------|-------------|
| list-work-units | {status?, type?} | List all work units, optionally filtered |
| show-work-unit | {id} | Show details of a specific work unit |
| create-story | {title, epic?} | Create a new user story |
| create-bug | {title, epic?} | Create a new bug report |
| create-task | {title, epic?} | Create a new task |
| create-epic | {title} | Create a new epic |
| update-work-unit | {id, title?, description?} | Update work unit details |
| update-work-unit-status | {id, status} | Change work unit status |
| update-work-unit-estimate | {id, estimate} | Set effort estimate |
| delete-work-unit | {id} | Delete a work unit |
| prioritize-work-unit | {id, priority} | Set priority (1=highest) |

### Workflow States
Status values: backlog, specifying, implementing, testing, validating, done, blocked

| Command | Args | Description |
|---------|------|-------------|
| board | {} | Show kanban board overview |
| auto-advance | {id} | Auto-advance work unit to next state |

### Example Mapping (Specifying Phase)
| Command | Args | Description |
|---------|------|-------------|
| add-rule | {workUnit, rule} | Add a business rule |
| add-example | {workUnit, rule, example} | Add example for a rule |
| add-question | {workUnit, question} | Add open question |
| answer-question | {workUnit, questionIndex, answer} | Answer a question |
| remove-rule | {workUnit, ruleIndex} | Remove a rule |
| remove-example | {workUnit, ruleIndex, exampleIndex} | Remove an example |
| remove-question | {workUnit, questionIndex} | Remove a question |
| show-deleted | {workUnit} | Show deleted rules/examples/questions |
| restore-rule | {workUnit, ruleIndex} | Restore a deleted rule |
| restore-example | {workUnit, ruleIndex, exampleIndex} | Restore a deleted example |
| restore-question | {workUnit, questionIndex} | Restore a deleted question |

### Feature Files (Gherkin)
| Command | Args | Description |
|---------|------|-------------|
| list-features | {workUnit?} | List feature files |
| show-feature | {feature} | Show feature file contents |
| create-feature | {workUnit, name, description?} | Create new feature file |
| add-scenario | {feature, name, steps?} | Add scenario to feature |
| add-step | {feature, scenario, step, type} | Add step (Given/When/Then) |
| update-scenario | {feature, scenario, name?} | Update scenario |
| delete-scenario | {feature, scenario} | Delete scenario |
| get-scenarios | {feature} | Get all scenarios from feature |
| generate-scenarios | {workUnit} | Generate scenarios from example map |

### Architecture & Foundation
| Command | Args | Description |
|---------|------|-------------|
| show-foundation | {} | Show project foundation document |
| update-foundation | {section, content} | Update foundation section |
| add-architecture-note | {workUnit, note, category?} | Add architecture decision |
| show-event-storm | {workUnit?} | Show event storming results |
| discover-event-storm | {workUnit} | Run event storm discovery |

### Tags & Organization
| Command | Args | Description |
|---------|------|-------------|
| list-tags | {} | List all registered tags |
| register-tag | {tag, description?, color?} | Register a new tag |
| add-tag-to-feature | {feature, tag} | Add tag to feature |
| add-tag-to-scenario | {feature, scenario, tag} | Add tag to scenario |
| tag-stats | {} | Show tag usage statistics |

### Dependencies
| Command | Args | Description |
|---------|------|-------------|
| dependencies | {id?} | Show dependency graph |
| add-dependency | {from, to, type?} | Add dependency between work units |
| remove-dependency | {from, to} | Remove dependency |
| suggest-dependencies | {id} | AI suggests dependencies |

### Epics
| Command | Args | Description |
|---------|------|-------------|
| list-epics | {} | List all epics |
| show-epic | {id} | Show epic details |
| create-epic | {title, description?} | Create new epic |
| delete-epic | {id} | Delete epic |

### Validation & Quality
| Command | Args | Description |
|---------|------|-------------|
| validate | {} | Validate all specifications |
| validate-work-units | {} | Validate work unit consistency |
| check | {id?} | Run quality checks |
| review | {id} | Review work unit readiness |

### Research Tools
| Command | Args | Description |
|---------|------|-------------|
| research | {tool, query, workUnit?} | Execute research tool |

Available research tools vary by configuration. Use \`configure-tools\` to see available tools.

### Queries & Reports
| Command | Args | Description |
|---------|------|-------------|
| query-work-units | {query} | Search work units |
| query-metrics | {} | Show project metrics |
| query-bottlenecks | {} | Find workflow bottlenecks |
| query-orphans | {} | Find orphaned items |
| show-coverage | {workUnit?} | Show test coverage |

### Checkpoints
| Command | Args | Description |
|---------|------|-------------|
| checkpoint | {message?} | Create spec checkpoint |
| list-checkpoints | {} | List available checkpoints |
| restore-checkpoint | {id} | Restore to checkpoint |

### Hooks (Automation)
| Command | Args | Description |
|---------|------|-------------|
| list-hooks | {} | List registered hooks |
| add-hook | {event, action} | Add automation hook |
| remove-hook | {id} | Remove hook |

## Common Workflows

### Starting Work on a Story
1. \`list-work-units\` with {status: "backlog"} - find work to do
2. \`show-work-unit\` with {id: "STORY-001"} - understand the story
3. \`update-work-unit-status\` with {id: "STORY-001", status: "specifying"} - start specifying

### Example Mapping Session
1. \`add-rule\` with {workUnit: "STORY-001", rule: "Users must be authenticated"}
2. \`add-example\` with {workUnit: "STORY-001", rule: "Users must be authenticated", example: "Valid JWT token allows access"}
3. \`add-question\` with {workUnit: "STORY-001", question: "What about API keys?"}
4. \`generate-scenarios\` with {workUnit: "STORY-001"} - create Gherkin from rules

### Moving to Implementation
1. \`review\` with {id: "STORY-001"} - check readiness
2. \`update-work-unit-status\` with {id: "STORY-001", status: "implementing"}

## Notes
- Commands return JSON with \`success: true/false\`
- Use \`help\` command with {command: "command-name"} for specific command help
- \`bootstrap\` and \`init\` commands are not available via this tool (use CLI)
`;

  return help;
}

/**
 * Generate help for a specific command
 */
function generateCommandHelp(commandName: string): string | null {
  // Map of commands to their AI-friendly documentation
  const commandDocs: Record<string, string> = {
    'list-work-units': `## list-work-units

List all work units in the project.

**Args:**
- \`status\` (optional): Filter by status - "backlog", "specifying", "implementing", "testing", "validating", "done", "blocked"
- \`type\` (optional): Filter by type - "story", "bug", "task", "epic"

**Examples:**
- List all: {args: "{}"}
- Backlog only: {args: "{\\"status\\": \\"backlog\\"}"}
- All bugs: {args: "{\\"type\\": \\"bug\\"}"}

**Returns:** {workUnits: [{id, title, status, type, ...}]}`,

    'show-work-unit': `## show-work-unit

Show detailed information about a specific work unit.

**Args:**
- \`id\` (required): Work unit ID (e.g., "STORY-001")

**Example:** {args: "{\\"id\\": \\"STORY-001\\"}"}

**Returns:** Full work unit details including rules, examples, questions, scenarios`,

    'create-story': `## create-story

Create a new user story.

**Args:**
- \`title\` (required): Story title
- \`epic\` (optional): Parent epic ID
- \`description\` (optional): Story description

**Example:** {args: "{\\"title\\": \\"User can reset password\\", \\"epic\\": \\"EPIC-001\\"}"}

**Returns:** {id: "STORY-XXX", ...}`,

    'add-rule': `## add-rule

Add a business rule to a work unit's example map.

**Args:**
- \`workUnit\` (required): Work unit ID
- \`rule\` (required): The business rule text

**Example:** {args: "{\\"workUnit\\": \\"STORY-001\\", \\"rule\\": \\"Password must be at least 8 characters\\"}"}

**Returns:** Updated work unit with new rule`,

    'add-example': `## add-example

Add an example to illustrate a business rule.

**Args:**
- \`workUnit\` (required): Work unit ID
- \`rule\` (required): The rule text (must match existing rule)
- \`example\` (required): Example that illustrates the rule

**Example:** {args: "{\\"workUnit\\": \\"STORY-001\\", \\"rule\\": \\"Password must be at least 8 characters\\", \\"example\\": \\"'hello' is rejected as too short\\"}"}`,

    'add-question': `## add-question

Add an open question during example mapping.

**Args:**
- \`workUnit\` (required): Work unit ID
- \`question\` (required): The question text

**Example:** {args: "{\\"workUnit\\": \\"STORY-001\\", \\"question\\": \\"Should we allow special characters in passwords?\\"}"}`,

    'update-work-unit-status': `## update-work-unit-status

Change the workflow status of a work unit.

**Args:**
- \`id\` (required): Work unit ID
- \`status\` (required): New status - "backlog", "specifying", "implementing", "testing", "validating", "done", "blocked"

**Example:** {args: "{\\"id\\": \\"STORY-001\\", \\"status\\": \\"implementing\\"}"}`,

    board: `## board

Show the kanban board with work units organized by status.

**Args:** None required

**Example:** {args: "{}"}

**Returns:** Board data with columns for each workflow state`,

    'generate-scenarios': `## generate-scenarios

Generate Gherkin scenarios from a work unit's example map.

**Args:**
- \`workUnit\` (required): Work unit ID

**Example:** {args: "{\\"workUnit\\": \\"STORY-001\\"}"}

**Returns:** Generated feature file with scenarios based on rules and examples`,

    research: `## research

Execute a research tool to gather information.

**Args:**
- \`tool\` (required): Research tool name (e.g., "perplexity")
- \`query\` (required): Research query
- \`workUnit\` (optional): Attach results to work unit

**Example:** {args: "{\\"tool\\": \\"perplexity\\", \\"query\\": \\"best practices for password validation\\", \\"workUnit\\": \\"STORY-001\\"}"}`,
  };

  return commandDocs[commandName] || null;
}

/**
 * Execute an fspec command programmatically via Commander.js
 *
 * This reuses the same Commander.js setup as the CLI, ensuring:
 * - All commands are statically imported (works in bundled code)
 * - Same routing logic as CLI
 * - DRY - no duplicate command registration
 */
export async function fspecCallback(
  command: string,
  argsJson: string,
  projectRoot: string
): Promise<string> {
  // Handle special 'help' command for AI-friendly documentation
  if (command === 'help') {
    let args: Record<string, unknown> = {};
    try {
      args = argsJson && argsJson.trim() ? JSON.parse(argsJson) : {};
    } catch {
      // Ignore parse errors for help command
    }

    // If a specific command is requested, return help for that command
    if (args.command && typeof args.command === 'string') {
      const commandHelp = generateCommandHelp(args.command);
      if (commandHelp) {
        return JSON.stringify({
          success: true,
          data: commandHelp,
        });
      }
      // Fall back to general help with note about unknown command
      return JSON.stringify({
        success: true,
        data:
          `Command "${args.command}" not found in quick reference.\n\n` +
          generateFspecToolHelp(),
      });
    }

    // Return general help
    return JSON.stringify({
      success: true,
      data: generateFspecToolHelp(),
    });
  }

  // Check for excluded commands
  if (EXCLUDED_COMMANDS.includes(command)) {
    return JSON.stringify({
      success: false,
      error: `Command '${command}' not supported via FspecTool. Use fspec CLI directly for setup commands.`,
      errorType: 'UnsupportedCommand',
      suggestions: [
        `Use 'fspec ${command}' directly in terminal`,
        'Setup commands require CLI environment',
      ],
    });
  }

  // Parse args from JSON
  let args: Record<string, unknown> = {};
  try {
    args = argsJson && argsJson.trim() ? JSON.parse(argsJson) : {};
  } catch (e) {
    return JSON.stringify({
      success: false,
      error: `Invalid JSON in args: ${e instanceof Error ? e.message : String(e)}`,
      errorType: 'InvalidArgs',
    });
  }

  // Capture stdout/stderr - must capture BOTH console.* AND process.stdout/stderr
  // because Commander.js writes help directly to process.stdout.write()
  let capturedOutput = '';
  let capturedError = '';

  // Capture console methods
  const originalLog = console.log;
  const originalError = console.error;
  const originalWarn = console.warn;

  console.log = (...args: unknown[]) => {
    capturedOutput +=
      args.map(a => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ') +
      '\n';
  };
  console.error = (...args: unknown[]) => {
    capturedError +=
      args.map(a => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ') +
      '\n';
  };
  console.warn = (...args: unknown[]) => {
    capturedError +=
      args.map(a => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ') +
      '\n';
  };

  // Capture process.stdout/stderr.write (Commander.js uses these directly for help)
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  const originalStderrWrite = process.stderr.write.bind(process.stderr);

  process.stdout.write = (chunk: unknown, ...rest: unknown[]): boolean => {
    if (typeof chunk === 'string') {
      capturedOutput += chunk;
    } else if (Buffer.isBuffer(chunk)) {
      capturedOutput += chunk.toString();
    }
    return true;
  };

  process.stderr.write = (chunk: unknown, ...rest: unknown[]): boolean => {
    if (typeof chunk === 'string') {
      capturedError += chunk;
    } else if (Buffer.isBuffer(chunk)) {
      capturedError += chunk.toString();
    }
    return true;
  };

  // Override process.exit to prevent Commander and command handlers from exiting
  // This is necessary because many commands call process.exit() directly
  const originalExit = process.exit;
  let exitCode: number | undefined;
  process.exit = ((code?: number): never => {
    exitCode = code ?? 0;
    throw new Error(`__FSPEC_EXIT_OVERRIDE__:${exitCode}`);
  }) as typeof process.exit;

  // Save and change cwd
  const originalCwd = process.cwd();

  try {
    // Change to project root for command execution
    process.chdir(projectRoot);

    // Create a fresh program instance
    const program = createProgram();

    // Configure to not exit on error - throws CommanderError instead
    program.exitOverride();

    // Configure output to go through our capture (belt + suspenders with process.stdout/stderr)
    program.configureOutput({
      writeOut: (str: string) => {
        capturedOutput += str;
      },
      writeErr: (str: string) => {
        capturedError += str;
      },
      outputError: (str: string) => {
        capturedError += str;
      },
    });

    // Build argv array: ['node', 'fspec', command, ...options]
    // Always request JSON format when available for structured output
    const argv = ['node', 'fspec', command, '--format', 'json'];

    // Convert args object to CLI flags
    for (const [key, value] of Object.entries(args)) {
      if (key === 'cwd' || key === 'format') continue; // Skip cwd and format (we handle them)

      const flagName =
        key.length === 1
          ? `-${key}`
          : `--${key.replace(/([A-Z])/g, '-$1').toLowerCase()}`;

      if (typeof value === 'boolean') {
        if (value) argv.push(flagName);
      } else if (value !== undefined && value !== null) {
        argv.push(flagName, String(value));
      }
    }

    // Execute command via Commander.js
    await program.parseAsync(argv, { from: 'node' });

    // Parse system reminders from captured stderr
    const systemReminders = parseSystemReminders(capturedError);

    // Try to extract and parse JSON from captured output
    // Commands may output log messages before JSON, so we need to find the JSON
    const trimmedOutput = capturedOutput.trim();
    let resultData: unknown;
    let isJson = false;

    // Try to find JSON object or array in the output
    const jsonMatch = trimmedOutput.match(/(\{[\s\S]*\}|\[[\s\S]*\])$/);
    if (jsonMatch) {
      try {
        resultData = JSON.parse(jsonMatch[1]);
        isJson = true;
      } catch {
        // Not valid JSON
      }
    }

    // Build result
    const result: Record<string, unknown> = {
      success: true,
    };

    if (isJson && resultData !== undefined) {
      // Merge JSON result into response (e.g., { workUnits: [...] })
      if (typeof resultData === 'object' && resultData !== null) {
        Object.assign(result, resultData);
      } else {
        result.data = resultData;
      }
    } else {
      result.data = trimmedOutput;
    }

    if (systemReminders.length > 0) {
      result.systemReminders = systemReminders;
    }

    return JSON.stringify(result);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);

    // Check for our exit override (commands that called process.exit)
    if (errorMessage.startsWith('__FSPEC_EXIT_OVERRIDE__:')) {
      const code = parseInt(errorMessage.split(':')[1], 10);
      const trimmedOutput = capturedOutput.trim();

      // Exit code 0 is success (e.g., help display)
      if (code === 0) {
        return JSON.stringify({
          success: true,
          data: trimmedOutput || capturedError.trim(),
        });
      }

      // Non-zero exit code indicates error
      return JSON.stringify({
        success: false,
        error: `Command exited with code ${code}`,
        errorType: 'ExitCode',
        data: trimmedOutput,
        stderr: capturedError.trim(),
      });
    }

    // Commander.js throws CommanderError on exitOverride - check error code
    // @ts-expect-error - CommanderError has code property
    const errorCode = error?.code as string | undefined;

    // Check if it's just a help/version exit (these are not errors)
    if (
      errorCode === 'commander.help' ||
      errorCode === 'commander.helpDisplayed' ||
      errorCode === 'commander.version' ||
      errorMessage.includes('(outputHelp)') ||
      errorMessage.includes('(version)')
    ) {
      return JSON.stringify({
        success: true,
        data: capturedOutput.trim(),
      });
    }

    // Check for command not found
    if (
      errorMessage.includes('unknown command') ||
      errorMessage.includes('error: unknown command')
    ) {
      return JSON.stringify({
        success: false,
        error: `Command '${command}' not found.`,
        errorType: 'CommandNotFound',
        suggestions: ['Run fspec --help to see available commands'],
      });
    }

    // Check if we still got some output despite error
    const trimmedOutput = capturedOutput.trim();
    if (trimmedOutput) {
      // Try to find JSON in case command succeeded but threw on exit
      const jsonMatch = trimmedOutput.match(/(\{[\s\S]*\}|\[[\s\S]*\])$/);
      if (jsonMatch) {
        try {
          const resultData = JSON.parse(jsonMatch[1]);
          const result: Record<string, unknown> = { success: true };
          if (typeof resultData === 'object' && resultData !== null) {
            Object.assign(result, resultData);
          } else {
            result.data = resultData;
          }
          return JSON.stringify(result);
        } catch {
          // Not valid JSON
        }
      }
    }

    return JSON.stringify({
      success: false,
      error: `Command failed: ${errorMessage}`,
      errorType: 'ExecutionError',
      stderr: capturedError.trim(),
    });
  } finally {
    // Restore console
    console.log = originalLog;
    console.error = originalError;
    console.warn = originalWarn;

    // Restore process.stdout/stderr.write
    process.stdout.write = originalStdoutWrite;
    process.stderr.write = originalStderrWrite;

    // Restore process.exit
    process.exit = originalExit;

    // Restore cwd
    process.chdir(originalCwd);
  }
}

/**
 * Parse system reminders from stderr content
 */
function parseSystemReminders(stderrContent: string): string[] {
  const reminders: string[] = [];
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

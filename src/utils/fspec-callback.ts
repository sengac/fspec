// TypeScript callback function for NAPI FspecTool integration
// This function uses Commander.js directly to execute fspec commands programmatically
// CODE-002/CODE-005: Support ALL fspec commands EXCEPT bootstrap and init

import { createProgram } from '../cli/program';
import {
  createCaptureContext,
  setOutputContext,
  resetOutputContext,
  setFspecPositionalArgs,
  clearFspecPositionalArgs,
  stripAnsi,
} from './output';

// Commands that are excluded from FspecTool (must use CLI directly)
const EXCLUDED_COMMANDS = ['bootstrap', 'init'];

/**
 * Generate AI-friendly help for the Fspec tool.
 * This explains how to use commands via the tool's parameter format.
 */
function generateFspecToolHelp(): string {
  const help = `# Fspec Tool Reference

## Tool Parameters

The Fspec tool accepts three parameters:

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| command | string | Yes | The fspec command name (e.g., "list-work-units") |
| args | string | No | JSON string with command arguments (default: "{}") |
| project_root | string | No | Project root directory (default: ".") |

## How Arguments Work

The \`args\` parameter is a **JSON object**. 

**Positional arguments** use the special \`_\` key (array):
- \`{"_": ["AUTH-001"]}\` → \`fspec command AUTH-001\`
- \`{"_": ["AUTH-001", "implementing"]}\` → \`fspec command AUTH-001 implementing\`

**Named options** use camelCase keys (become kebab-case flags):
- \`{"status": "backlog"}\` → \`--status backlog\`
- \`{"skipValidation": true}\` → \`--skip-validation\`

## Example Tool Calls

**List all work units:**
\`\`\`
command: "list-work-units"
args: {}
\`\`\`

**List work units filtered by status:**
\`\`\`
command: "list-work-units"
args: {"status": "backlog"}
\`\`\`

**Show a specific work unit:**
\`\`\`
command: "show-work-unit"
args: {"_": ["STORY-001"]}
\`\`\`

**Update work unit status:**
\`\`\`
command: "update-work-unit-status"
args: {"_": ["STORY-001", "implementing"]}
\`\`\`

**Add a rule to a work unit:**
\`\`\`
command: "add-rule"
args: {"_": ["STORY-001", "Users must be authenticated"]}
\`\`\`

**Execute research tool:**
\`\`\`
command: "research"
args: {"tool": "perplexity", "query": "best practices for password hashing"}
\`\`\`

---

## Command Reference

### Work Unit Management

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| list-work-units | - | status, type | List all work units |
| show-work-unit | [workUnitId] | - | Show work unit details |
| create-story | [prefix, title] | epic, description | Create user story |
| create-bug | [prefix, title] | epic, description | Create bug report |
| create-task | [prefix, title] | epic, description | Create task |
| update-work-unit | [workUnitId] | title, description | Update work unit |
| update-work-unit-status | [workUnitId, status] | - | Change workflow status |
| update-work-unit-estimate | [workUnitId, estimate] | - | Set effort estimate |
| delete-work-unit | [workUnitId] | - | Delete work unit |
| prioritize-work-unit | [workUnitId, priority] | - | Set priority (1=highest) |

**Status values:** backlog, specifying, implementing, testing, validating, done, blocked

### Example Mapping (Specifying Phase)

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| add-rule | [workUnitId, rule] | - | Add business rule |
| add-example | [workUnitId, example] | rule | Add example for rule |
| add-question | [workUnitId, question] | - | Add open question |
| answer-question | [workUnitId, index, answer] | - | Answer question |
| remove-rule | [workUnitId, index] | - | Remove rule |
| remove-example | [workUnitId, index] | - | Remove example |
| remove-question | [workUnitId, index] | - | Remove question |
| restore-rule | [workUnitId, index] | - | Restore deleted rule |
| restore-example | [workUnitId, index] | - | Restore deleted example |
| restore-question | [workUnitId, index] | - | Restore deleted question |
| show-deleted | [workUnitId] | - | Show deleted items |
| compact-work-unit | [workUnitId] | - | Permanently remove deleted items |

### Feature Files (Gherkin)

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| list-features | - | workUnit | List feature files |
| show-feature | [feature] | - | Show feature contents |
| create-feature | [name] | workUnit, description | Create feature file |
| add-scenario | [scenario] | feature, steps | Add scenario |
| add-step | [feature, scenario, type, text] | - | Add step (type: Given/When/Then) |
| update-scenario | [feature, oldName, newName] | - | Update scenario |
| delete-scenario | [scenario] | feature | Delete scenario |
| get-scenarios | [feature] | - | Get all scenarios |
| generate-scenarios | [workUnitId] | - | Generate from example map |

### Workflow & Board

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| board | - | - | Show kanban board |
| auto-advance | [workUnitId] | - | Auto-advance to next state |
| review | [workUnitId] | - | Review work unit readiness |

### Research Tools

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| research | - | tool, workUnit, query | Execute research tool |

**Available research tools depend on project configuration.**

To list available tools:
\`\`\`
command: "research"
args: {}
\`\`\`

To use a tool (e.g., perplexity):
\`\`\`
command: "research"
args: {"tool": "perplexity", "query": "your research question"}
\`\`\`

### Architecture & Foundation

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| show-foundation | - | - | Show foundation document |
| update-foundation | [section, content] | - | Update foundation section |
| add-architecture-note | [workUnitId, note] | category | Add architecture decision |
| show-event-storm | [workUnitId] | - | Show event storm results |
| discover-event-storm | [workUnitId] | - | Run event storm discovery |

### Tags & Organization

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| list-tags | - | - | List registered tags |
| register-tag | [tag, category, description] | color | Register new tag |
| add-tag-to-feature | [feature, tags...] | - | Add tag to feature |
| add-tag-to-scenario | [feature, tags...] | scenario | Add tag to scenario |
| tag-stats | - | - | Show tag statistics |

### Dependencies

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| dependencies | - | id | Show dependency graph |
| add-dependency | [workUnitId] | dependsOn, type | Add dependency |
| remove-dependency | [workUnitId] | dependsOn | Remove dependency |
| suggest-dependencies | [workUnitId] | - | AI suggests dependencies |

### Epics

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| list-epics | - | - | List all epics |
| show-epic | [epicId] | - | Show epic details |
| create-epic | [title] | description | Create epic |
| delete-epic | [epicId] | - | Delete epic |

### Validation & Quality

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| validate | - | - | Validate all specs |
| validate-work-units | - | - | Validate work units |
| check | - | id | Run quality checks |

### Queries & Reports

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| query-work-units | [query] | - | Search work units |
| query-metrics | - | - | Show project metrics |
| query-bottlenecks | - | - | Find bottlenecks |
| query-orphans | - | - | Find orphaned items |
| show-coverage | - | workUnit | Show test coverage |

### Checkpoints

| Command | Positional Args (\`_\`) | Optional Args | Description |
|---------|------------------------|---------------|-------------|
| checkpoint | [workUnitId] | message | Create checkpoint |
| list-checkpoints | [workUnitId] | - | List checkpoints |
| restore-checkpoint | [workUnitId, checkpointName] | - | Restore checkpoint |

---

## Notes

- **Excluded commands:** \`bootstrap\` and \`init\` must be run via CLI
- **Positional args:** Use \`_\` key with array: \`{"_": ["arg1", "arg2"]}\`
- **Named options:** Use camelCase keys: \`{"status": "backlog"}\`
- **Get command help:** Use \`command: "help", args: {"command": "command-name"}\`
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

### Tool Call
\`\`\`
command: "list-work-units"
args: {}                           // No filter
args: {"status": "backlog"}        // Filter by status
args: {"type": "story"}            // Filter by type
\`\`\`

### Args (all optional - these are named options, not positional)
| Arg | Type | Values |
|-----|------|--------|
| status | string | backlog, specifying, implementing, testing, validating, done, blocked |
| type | string | story, bug, task, epic |

### Returns
\`\`\`json
{
  "success": true,
  "workUnits": [
    {"id": "STORY-001", "title": "...", "status": "backlog", "type": "story", ...}
  ]
}
\`\`\``,

    'show-work-unit': `## show-work-unit

Show detailed information about a specific work unit including rules, examples, and questions.

### Tool Call
\`\`\`
command: "show-work-unit"
args: {"_": ["STORY-001"]}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId] |

### Returns
Full work unit details including:
- Basic info (title, status, type, description)
- Example map (rules, examples, questions)
- Linked features and scenarios
- Dependencies`,

    'create-story': `## create-story

Create a new user story work unit.

### Tool Call
\`\`\`
command: "create-story"
args: {"_": ["AUTH", "User can reset password"]}
args: {"_": ["AUTH", "User can reset password"], "epic": "EPIC-001"}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [prefix, title] |
| epic | string | No | Parent epic ID |
| description | string | No | Story description |

### Returns
\`\`\`json
{
  "success": true,
  "id": "AUTH-001",
  "title": "User can reset password",
  "status": "backlog"
}
\`\`\``,

    'add-rule': `## add-rule

Add a business rule to a work unit's example map during the specifying phase.

### Tool Call
\`\`\`
command: "add-rule"
args: {"_": ["STORY-001", "Password must be at least 8 characters"]}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId, rule] |

### Returns
Updated work unit with the new rule added.`,

    'add-example': `## add-example

Add an example that illustrates a business rule.

### Tool Call
\`\`\`
command: "add-example"
args: {"_": ["STORY-001", "'hello' (5 chars) is rejected"], "rule": "Password must be at least 8 characters"}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId, example] |
| rule | string | Yes | The rule text (must match existing rule exactly) |

### Notes
- The rule text must match an existing rule exactly
- Examples help clarify edge cases and expected behavior`,

    'add-question': `## add-question

Add an open question during example mapping to capture uncertainties.

### Tool Call
\`\`\`
command: "add-question"
args: {"_": ["STORY-001", "Should we allow special characters in passwords?"]}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId, question] |

### Notes
Questions capture things that need clarification before implementation.`,

    'update-work-unit-status': `## update-work-unit-status

Change the workflow status of a work unit.

### Tool Call
\`\`\`
command: "update-work-unit-status"
args: {"_": ["STORY-001", "implementing"]}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId, status] |

### Status Values
- **backlog** - Not yet started
- **specifying** - Example mapping in progress
- **implementing** - Code being written
- **testing** - Tests being written/run
- **validating** - Final validation
- **done** - Complete
- **blocked** - Blocked by dependency/issue`,

    board: `## board

Show the kanban board with work units organized by workflow status.

### Tool Call
\`\`\`
command: "board"
args: {}
\`\`\`

### Args
None required.

### Returns
Board data showing work units in each column (backlog, specifying, implementing, etc.)`,

    'generate-scenarios': `## generate-scenarios

Generate Gherkin scenarios from a work unit's example map (rules and examples).

### Tool Call
\`\`\`
command: "generate-scenarios"
args: {"_": ["STORY-001"]}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| _ | array | Yes | Positional args: [workUnitId] |

### Notes
- Requires the work unit to have rules and examples defined
- Creates a feature file with scenarios based on the example map`,

    research: `## research

Execute a research tool to gather information. Research tools are configured per-project.

### List Available Tools
\`\`\`
command: "research"
args: {}
\`\`\`

### Execute a Research Tool
\`\`\`
command: "research"
args: {"tool": "perplexity", "query": "best practices for password hashing"}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| tool | string | Yes* | Research tool name (* No if just listing tools) |
| query | string | Depends | Query for the tool (required for most tools) |
| workUnit | string | No | Attach results to this work unit |

### Common Research Tools
- **perplexity** - AI-powered web search
- **tavily** - Search API
- **github** - GitHub code search

### Notes
- Available tools depend on project configuration in spec/fspec-config.json
- Each tool may have additional specific arguments`,

    help: `## help

Get help for the Fspec tool or a specific command.

### General Help
\`\`\`
command: "help"
args: {}
\`\`\`

### Command-Specific Help
\`\`\`
command: "help"
args: {"command": "add-rule"}
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| command | string | No | Get help for specific command |`,
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

  // Set up output capture context
  // Commands now use output.log/error/warn which route through this context
  const {
    context: captureContext,
    stdout: capturedStdout,
    stderr: capturedStderr,
  } = createCaptureContext();

  // We still need a string for Commander.js help output (uses configureOutput)
  let commanderOutput = '';
  let commanderError = '';

  // NOTE: We intentionally do NOT override process.stdout.write / process.stderr.write.
  // The TUI's Ink renderer writes to process.stdout concurrently during async command
  // execution (ThinkingIndicator spinner, screen redraws), and a global override would
  // capture those TUI frames into the tool result — contaminating it with spinner text,
  // conversation content, and UI chrome. Instead, all command output is captured via:
  //   - Layer 1: output.log/error/warn → createCaptureContext arrays (all commands use this)
  //   - Layer 2: Commander.js configureOutput → commanderOutput/commanderError strings
  // Layer 2 is propagated to all subcommands below to ensure complete capture.

  // Override process.exit to prevent Commander and command handlers from exiting
  // This is necessary because many commands call process.exit() directly
  const originalExit = process.exit;
  process.exit = ((code?: number): never => {
    throw new Error(`__FSPEC_EXIT_OVERRIDE__:${code ?? 0}`);
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

    // Configure output to capture Commander.js help output and errors.
    // IMPORTANT: configureOutput must be propagated to ALL subcommands because
    // Commander's copyInheritedSettings (called during addCommand) copies the parent's
    // _outputConfiguration by reference BEFORE we call configureOutput. When configureOutput
    // creates a new object via spread, the shared reference is broken and subcommands
    // retain the old config that writes to process.stdout/stderr directly.
    const commanderOutputConfig = {
      writeOut: (str: string) => {
        commanderOutput += stripAnsi(str);
      },
      writeErr: (str: string) => {
        commanderError += stripAnsi(str);
      },
      outputError: (str: string) => {
        commanderError += stripAnsi(str);
      },
    };
    program.configureOutput(commanderOutputConfig);
    for (const cmd of program.commands) {
      cmd.configureOutput(commanderOutputConfig);
    }

    // Build argv array: ['node', 'fspec', command, ...options]
    // Always request JSON format when available for structured output
    const argv = ['node', 'fspec', command];

    // Handle positional arguments via special '_' key (array of positional args in order)
    // This follows the convention used by minimist/yargs
    const positionalArgs = args._ as unknown[] | undefined;
    if (Array.isArray(positionalArgs)) {
      for (const arg of positionalArgs) {
        if (arg !== undefined && arg !== null) {
          argv.push(String(arg));
        }
      }
    }

    // RES-022: Set positional args for commands that need them (like research)
    // This allows commands to access the args without using process.argv
    const positionalArgsStrings = Array.isArray(positionalArgs)
      ? positionalArgs
          .filter(a => a !== undefined && a !== null)
          .map(a => String(a))
      : [];
    setFspecPositionalArgs(positionalArgsStrings);

    // Dynamically check if the command supports --format option
    const cmd = program.commands.find(c => c.name() === command);
    const hasFormatOption = cmd?.options.some(
      opt => opt.long === '--format' || opt.short === '-f'
    );
    if (hasFormatOption) {
      argv.push('--format', 'json');
    }

    // Convert remaining args object to CLI flags
    for (const [key, value] of Object.entries(args)) {
      if (key === '_' || key === 'cwd' || key === 'format') continue; // Skip special keys

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

    // Set the output context to capture command output
    setOutputContext(captureContext);

    // Execute command via Commander.js
    await program.parseAsync(argv, { from: 'node' });

    // Reset output context immediately after execution
    resetOutputContext();

    // Combine captured output from both sources:
    // - capturedStdout/capturedStderr: output from commands using output.log/error/warn
    // - commanderOutput/commanderError: output from Commander.js help/errors
    const capturedOutput =
      capturedStdout.join('\n') +
      (commanderOutput ? '\n' + commanderOutput : '');
    const capturedError =
      capturedStderr.join('\n') + (commanderError ? '\n' + commanderError : '');

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
    // Reset output context before processing error
    resetOutputContext();

    // Combine captured output from all sources
    const capturedOutput =
      capturedStdout.join('\n') +
      (commanderOutput ? '\n' + commanderOutput : '');
    const capturedError =
      capturedStderr.join('\n') + (commanderError ? '\n' + commanderError : '');

    const errorMessage = error instanceof Error ? error.message : String(error);

    // Check for our exit override (commands that called process.exit)
    if (errorMessage.startsWith('__FSPEC_EXIT_OVERRIDE__:')) {
      const code = parseInt(errorMessage.split(':')[1], 10);
      const trimmedOutput = capturedOutput.trim();

      // EXIT CASCADE FIX: Check if the ORIGINAL exit was successful (code 0)
      // This happens when a command succeeds, calls process.exit(0), but the exception
      // is caught by the command's own try/catch block which then calls process.exit(1).
      // We detect this by checking if stderr contains __FSPEC_EXIT_OVERRIDE__:0
      const originalExitWasSuccess = capturedError.includes(
        '__FSPEC_EXIT_OVERRIDE__:0'
      );

      // Helper function to clean up __FSPEC_EXIT_OVERRIDE__ artifacts from output
      // Note: ANSI codes are already stripped at capture time, so only plain text patterns needed
      const cleanExitOverrideArtifacts = (text: string): string => {
        return (
          text
            // Handle "Error: __FSPEC_EXIT_OVERRIDE__:N" patterns
            .replace(/Error:\s*__FSPEC_EXIT_OVERRIDE__:\d+\n?/g, '')
            // Handle "✗ Error: __FSPEC_EXIT_OVERRIDE__:N" patterns (from command catch blocks)
            .replace(
              /✗\s*(Error:|Failed:)?\s*__FSPEC_EXIT_OVERRIDE__:\d+\n?/gi,
              ''
            )
            // Handle any remaining bare "__FSPEC_EXIT_OVERRIDE__:N" patterns
            .replace(/__FSPEC_EXIT_OVERRIDE__:\d+\n?/g, '')
            .trim()
        );
      };

      // Exit code 0 is success (e.g., help display)
      // OR if original exit was 0 but got cascaded to 1 by command's catch block
      if (code === 0 || originalExitWasSuccess) {
        const cleanOutput = cleanExitOverrideArtifacts(trimmedOutput);
        const cleanError = cleanExitOverrideArtifacts(capturedError);

        // Parse system reminders from captured stderr (before cleaning)
        const systemReminders = parseSystemReminders(capturedError);

        const result: Record<string, unknown> = {
          success: true,
          data: cleanOutput || cleanError,
        };

        if (systemReminders.length > 0) {
          result.systemReminders = systemReminders;
        }

        return JSON.stringify(result);
      }

      // Non-zero exit code indicates error (and it wasn't a cascade from success)
      const cleanStdout = cleanExitOverrideArtifacts(trimmedOutput);
      const cleanStderr = cleanExitOverrideArtifacts(capturedError);

      const errorDetail = cleanStderr || cleanStdout || `Exit code ${code}`;
      return JSON.stringify({
        success: false,
        error: errorDetail,
        errorType: 'CommandError',
        exitCode: code,
        stdout: cleanStdout,
        stderr: cleanStderr,
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
    // Ensure output context is reset (in case of unexpected errors)
    resetOutputContext();

    // RES-022: Clear fspec positional args
    clearFspecPositionalArgs();

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

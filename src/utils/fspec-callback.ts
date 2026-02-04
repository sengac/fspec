// TypeScript callback function for NAPI FspecTool integration
// This function uses Commander.js directly to execute fspec commands programmatically
// CODE-002/CODE-005: Support ALL fspec commands EXCEPT bootstrap and init

import { createProgram } from '../cli/program';

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

The \`args\` parameter is a **JSON string**. Keys in the JSON become command-line flags:
- camelCase keys become kebab-case flags: \`workUnit\` → \`--work-unit\`
- Boolean \`true\` adds the flag: \`{"all": true}\` → \`--all\`
- String/number values become flag values: \`{"status": "backlog"}\` → \`--status backlog\`

## Example Tool Calls

**List all work units:**
\`\`\`
command: "list-work-units"
args: "{}"
\`\`\`

**List work units filtered by status:**
\`\`\`
command: "list-work-units"
args: "{\\"status\\": \\"backlog\\"}"
\`\`\`

**Show a specific work unit:**
\`\`\`
command: "show-work-unit"
args: "{\\"id\\": \\"STORY-001\\"}"
\`\`\`

**Add a rule to a work unit:**
\`\`\`
command: "add-rule"
args: "{\\"workUnit\\": \\"STORY-001\\", \\"rule\\": \\"Users must be authenticated\\"}"
\`\`\`

**Execute research tool:**
\`\`\`
command: "research"
args: "{\\"tool\\": \\"perplexity\\", \\"query\\": \\"best practices for password hashing\\"}"
\`\`\`

---

## Command Reference

### Work Unit Management

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| list-work-units | - | status, type | List all work units |
| show-work-unit | id | - | Show work unit details |
| create-story | title | epic, description | Create user story |
| create-bug | title | epic, description | Create bug report |
| create-task | title | epic, description | Create task |
| update-work-unit | id | title, description | Update work unit |
| update-work-unit-status | id, status | - | Change workflow status |
| update-work-unit-estimate | id, estimate | - | Set effort estimate |
| delete-work-unit | id | - | Delete work unit |
| prioritize-work-unit | id, priority | - | Set priority (1=highest) |

**Status values:** backlog, specifying, implementing, testing, validating, done, blocked

### Example Mapping (Specifying Phase)

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| add-rule | workUnit, rule | - | Add business rule |
| add-example | workUnit, rule, example | - | Add example for rule |
| add-question | workUnit, question | - | Add open question |
| answer-question | workUnit, questionIndex, answer | - | Answer question |
| remove-rule | workUnit, ruleIndex | - | Remove rule |
| remove-example | workUnit, ruleIndex, exampleIndex | - | Remove example |
| remove-question | workUnit, questionIndex | - | Remove question |
| restore-rule | workUnit, ruleIndex | - | Restore deleted rule |
| restore-example | workUnit, ruleIndex, exampleIndex | - | Restore deleted example |
| restore-question | workUnit, questionIndex | - | Restore deleted question |
| show-deleted | workUnit | - | Show deleted items |
| compact-work-unit | workUnit | - | Permanently remove deleted items |

### Feature Files (Gherkin)

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| list-features | - | workUnit | List feature files |
| show-feature | feature | - | Show feature contents |
| create-feature | workUnit, name | description | Create feature file |
| add-scenario | feature, name | steps | Add scenario |
| add-step | feature, scenario, step, type | - | Add step (type: Given/When/Then) |
| update-scenario | feature, scenario | name | Update scenario |
| delete-scenario | feature, scenario | - | Delete scenario |
| get-scenarios | feature | - | Get all scenarios |
| generate-scenarios | workUnit | - | Generate from example map |

### Workflow & Board

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| board | - | - | Show kanban board |
| auto-advance | id | - | Auto-advance to next state |
| review | id | - | Review work unit readiness |

### Research Tools

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| research | tool | workUnit, (tool-specific args) | Execute research tool |

**Available research tools depend on project configuration.**

To list available tools:
\`\`\`
command: "research"
args: "{}"
\`\`\`

To use a tool (e.g., perplexity):
\`\`\`
command: "research"
args: "{\\"tool\\": \\"perplexity\\", \\"query\\": \\"your research question\\"}"
\`\`\`

### Architecture & Foundation

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| show-foundation | - | - | Show foundation document |
| update-foundation | section, content | - | Update foundation section |
| add-architecture-note | workUnit, note | category | Add architecture decision |
| show-event-storm | - | workUnit | Show event storm results |
| discover-event-storm | workUnit | - | Run event storm discovery |

### Tags & Organization

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| list-tags | - | - | List registered tags |
| register-tag | tag | description, color | Register new tag |
| add-tag-to-feature | feature, tag | - | Add tag to feature |
| add-tag-to-scenario | feature, scenario, tag | - | Add tag to scenario |
| tag-stats | - | - | Show tag statistics |

### Dependencies

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| dependencies | - | id | Show dependency graph |
| add-dependency | from, to | type | Add dependency |
| remove-dependency | from, to | - | Remove dependency |
| suggest-dependencies | id | - | AI suggests dependencies |

### Epics

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| list-epics | - | - | List all epics |
| show-epic | id | - | Show epic details |
| create-epic | title | description | Create epic |
| delete-epic | id | - | Delete epic |

### Validation & Quality

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| validate | - | - | Validate all specs |
| validate-work-units | - | - | Validate work units |
| check | - | id | Run quality checks |

### Queries & Reports

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| query-work-units | query | - | Search work units |
| query-metrics | - | - | Show project metrics |
| query-bottlenecks | - | - | Find bottlenecks |
| query-orphans | - | - | Find orphaned items |
| show-coverage | - | workUnit | Show test coverage |

### Checkpoints

| Command | Required Args | Optional Args | Description |
|---------|---------------|---------------|-------------|
| checkpoint | - | message | Create checkpoint |
| list-checkpoints | - | - | List checkpoints |
| restore-checkpoint | id | - | Restore checkpoint |

---

## Notes

- **Excluded commands:** \`bootstrap\` and \`init\` must be run via CLI
- **JSON escaping:** In the args string, escape quotes: \`"{\\"key\\": \\"value\\"}"\`
- **Get command help:** Use \`command: "help", args: "{\\"command\\": \\"command-name\\"}"\`
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
args: "{}"                           // No filter
args: "{\\"status\\": \\"backlog\\"}"     // Filter by status
args: "{\\"type\\": \\"story\\"}"         // Filter by type
\`\`\`

### Args (all optional)
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
args: "{\\"id\\": \\"STORY-001\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| id | string | Yes | Work unit ID (e.g., "STORY-001") |

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
args: "{\\"title\\": \\"User can reset password\\"}"
args: "{\\"title\\": \\"User can reset password\\", \\"epic\\": \\"EPIC-001\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| title | string | Yes | Story title |
| epic | string | No | Parent epic ID |
| description | string | No | Story description |

### Returns
\`\`\`json
{
  "success": true,
  "id": "STORY-XXX",
  "title": "User can reset password",
  "status": "backlog"
}
\`\`\``,

    'add-rule': `## add-rule

Add a business rule to a work unit's example map during the specifying phase.

### Tool Call
\`\`\`
command: "add-rule"
args: "{\\"workUnit\\": \\"STORY-001\\", \\"rule\\": \\"Password must be at least 8 characters\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| workUnit | string | Yes | Work unit ID |
| rule | string | Yes | Business rule text |

### Returns
Updated work unit with the new rule added.`,

    'add-example': `## add-example

Add an example that illustrates a business rule.

### Tool Call
\`\`\`
command: "add-example"
args: "{\\"workUnit\\": \\"STORY-001\\", \\"rule\\": \\"Password must be at least 8 characters\\", \\"example\\": \\"'hello' (5 chars) is rejected\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| workUnit | string | Yes | Work unit ID |
| rule | string | Yes | The rule text (must match existing rule exactly) |
| example | string | Yes | Example that illustrates the rule |

### Notes
- The rule text must match an existing rule exactly
- Examples help clarify edge cases and expected behavior`,

    'add-question': `## add-question

Add an open question during example mapping to capture uncertainties.

### Tool Call
\`\`\`
command: "add-question"
args: "{\\"workUnit\\": \\"STORY-001\\", \\"question\\": \\"Should we allow special characters in passwords?\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| workUnit | string | Yes | Work unit ID |
| question | string | Yes | Question text |

### Notes
Questions capture things that need clarification before implementation.`,

    'update-work-unit-status': `## update-work-unit-status

Change the workflow status of a work unit.

### Tool Call
\`\`\`
command: "update-work-unit-status"
args: "{\\"id\\": \\"STORY-001\\", \\"status\\": \\"implementing\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| id | string | Yes | Work unit ID |
| status | string | Yes | New status |

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
args: "{}"
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
args: "{\\"workUnit\\": \\"STORY-001\\"}"
\`\`\`

### Args
| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| workUnit | string | Yes | Work unit ID |

### Notes
- Requires the work unit to have rules and examples defined
- Creates a feature file with scenarios based on the example map`,

    research: `## research

Execute a research tool to gather information. Research tools are configured per-project.

### List Available Tools
\`\`\`
command: "research"
args: "{}"
\`\`\`

### Execute a Research Tool
\`\`\`
command: "research"
args: "{\\"tool\\": \\"perplexity\\", \\"query\\": \\"best practices for password hashing\\"}"
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
args: "{}"
\`\`\`

### Command-Specific Help
\`\`\`
command: "help"
args: "{\\"command\\": \\"add-rule\\"}"
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

  process.stdout.write = (chunk: unknown, ..._rest: unknown[]): boolean => {
    if (typeof chunk === 'string') {
      capturedOutput += chunk;
    } else if (Buffer.isBuffer(chunk)) {
      capturedOutput += chunk.toString();
    }
    return true;
  };

  process.stderr.write = (chunk: unknown, ..._rest: unknown[]): boolean => {
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

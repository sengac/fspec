# BUG-024: Dependencies Command Error Details

## Command Executed
```bash
./dist/index.js dependencies RES-001
```

## Full Error Stack Trace
```
file:///home/rquast/projects/fspec/dist/index.js:8359
    throw new Error(`Invalid action: ${e}`);
          ^

Error: Invalid action: RES-001
    at Command.Wc (file:///home/rquast/projects/fspec/dist/index.js:8359:11)
    at Command.listener [as _actionHandler] (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:568:17)
    at /home/rquast/projects/fspec/node_modules/commander/lib/command.js:1604:14
    at Command._chainOrCall (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1488:12)
    at Command._parseCommand (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1603:27)
    at /home/rquast/projects/fspec/node_modules/commander/lib/command.js:1367:27
    at Command._chainOrCall (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1488:12)
    at Command._dispatchSubcommand (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1363:25)
    at Command._parseCommand (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1559:19)
    at Command.parse (/home/rquast/projects/fspec/node_modules/commander/lib/command.js:1093:10)

Node.js v22.20.0
```

## Expected Behavior
The command should accept a work unit ID as an argument and display the dependency tree for that work unit, showing:
- dependsOn relationships
- blocks relationships
- blockedBy relationships
- relatesTo relationships

## Actual Behavior
Command throws "Invalid action: <id>" error, treating the work unit ID as an invalid action instead of a valid argument.

## Context
- Work unit ID: RES-001
- Command location: dist/index.js:8359
- Error suggests Commander.js argument parsing issue
- The command appears to be registered but not accepting the expected argument

## Reproduction Steps
1. Create a work unit with dependencies (e.g., RES-001 depends on MCP-002)
2. Run: `fspec dependencies RES-001`
3. Observe error instead of dependency tree output

## Related Commands
The help system shows this command should work:
```bash
fspec dependencies <id>
```

But the implementation doesn't match the expected signature.

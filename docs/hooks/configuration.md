# Hook Configuration

This document explains the hook configuration format for fspec.

## Configuration File

Hooks are configured in `spec/fspec-hooks.json` at the root of your project.

## Complete Example

```json
{
  "global": {
    "timeout": 120,
    "shell": "/bin/bash"
  },
  "hooks": {
    "pre-update-work-unit-status": [
      {
        "name": "validate-feature-file",
        "command": "spec/hooks/validate-feature.sh",
        "blocking": true,
        "timeout": 30
      }
    ],
    "post-update-work-unit-status": [
      {
        "name": "run-tests",
        "command": "spec/hooks/run-tests.sh",
        "blocking": false,
        "condition": {
          "tags": ["@security"],
          "prefix": ["AUTH", "SEC"]
        }
      }
    ],
    "post-implementing": [
      {
        "name": "lint",
        "command": "spec/hooks/lint.sh",
        "blocking": true
      }
    ]
  }
}
```

## Configuration Fields

### Global Defaults

The `global` object defines default values for all hooks:

- **`timeout`** (number, optional): Default timeout in seconds for all hooks. Default: 60
- **`shell`** (string, optional): Shell to use for executing hooks. Default: system default

### Hooks Object

The `hooks` object maps event names to arrays of hook definitions. Event names follow the pattern:

- `pre-<command-name>` - Executed before command logic
- `post-<command-name>` - Executed after command logic

### Hook Definition

Each hook has the following properties:

- **`name`** (string, required): Unique name for the hook
- **`command`** (string, required): Path to the hook script, relative to project root
- **`blocking`** (boolean, optional): If true, hook failure prevents command execution (pre-hooks) or sets exit code to 1 (post-hooks). Default: false
- **`timeout`** (number, optional): Timeout in seconds. Overrides global timeout
- **`condition`** (object, optional): Conditions for when the hook should run
  - **`tags`** (array, optional): Hook runs if work unit has ANY of these tags (OR logic)
  - **`prefix`** (array, optional): Hook runs if work unit ID starts with ANY of these prefixes (OR logic)
  - **`epic`** (string, optional): Hook runs if work unit belongs to this epic
  - **`estimateMin`** (number, optional): Hook runs if work unit estimate >= this value
  - **`estimateMax`** (number, optional): Hook runs if work unit estimate <= this value

Multiple condition fields use AND logic (all must match).

## Hook Context

Hooks receive a JSON context object via stdin:

```json
{
  "workUnitId": "AUTH-001",
  "event": "pre-update-work-unit-status",
  "timestamp": "2025-01-15T10:30:00.000Z"
}
```

## Exit Codes

- **0**: Success
- **Non-zero**: Failure

For blocking hooks, non-zero exit codes prevent command execution (pre-hooks) or set command exit code to 1 (post-hooks).

## Output

- **stdout**: Normal output, displayed to user
- **stderr**: Error output
  - Blocking hook stderr is wrapped in `<system-reminder>` tags for AI agents
  - Non-blocking hook stderr is displayed as-is

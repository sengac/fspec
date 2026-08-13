# Command & File Blocklist

fspec includes a blocklist system that can block, allow, or prompt for approval on specific commands and file access patterns. This provides fine-grained control over what agents can do—without requiring a full sandbox.

## Configuration Files

| Location | Purpose |
|----------|---------|
| `~/.fspec/blocklist.json` | System-wide rules (apply to all projects) |
| `.fspec/blocklist.json` | Project-specific rules (override system rules) |

## Config Structure

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "git-checkout-block",
      "pattern": "^git\\s+checkout\\b",
      "action": "block",
      "reason": "git checkout is deprecated",
      "guidance": "Use git switch instead"
    },
    {
      "id": "ssh-config-prompt",
      "pattern": "\\.ssh",
      "action": "prompt",
      "reason": "SSH directory may contain sensitive keys"
    },
    {
      "id": "allow-node-modules-rm",
      "pattern": "^rm\\s+-rf\\s+\\./node_modules\\b",
      "action": "allow",
      "reason": ""
    }
  ]
}
```

## Actions

| Action | Behavior |
|--------|----------|
| **block** | Immediately reject. The AI receives the `reason` and `guidance` as an error message. |
| **prompt** | Pause and ask the user. Shows a triple-choice dialog: **Allow Once**, **Allow Session**, or **Deny**. |
| **allow** | Explicitly permit. Used to override a more general blocking rule. |

## How Rules Are Evaluated

1. **Project rules first** — `.fspec/blocklist.json` rules are checked before system rules
2. **First match wins** — Evaluation stops at the first matching pattern
3. **Allow overrides block** — A project `allow` rule can override a system `block` rule

This means you can have a system-wide rule blocking `rm -rf` but allow it specifically for `./node_modules` in a project config.

## Pattern Syntax

Patterns use **regex**. Common patterns:

```json
"^git\\s+checkout\\b"     // Command starts with "git checkout"
"\\.env"                   // Path contains ".env"
"^rm\\s+-rf\\b"           // Command starts with "rm -rf"
"~/.ssh"                   // Path contains "~/.ssh"
```

## What Gets Checked

- **Bash tool** — Command string is checked before execution
- **Read/Write/Edit tools** — File path is checked before access

## Session Allowances

When a user selects **Allow Session** on a prompt, that pattern is remembered for the current session. The agent can access matching resources without re-prompting until the TUI is restarted.

## Example: Protecting Sensitive Files

System config (`~/.fspec/blocklist.json`):

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "ssh-prompt",
      "pattern": "\\.ssh",
      "action": "prompt",
      "reason": "SSH keys are sensitive credentials"
    },
    {
      "id": "env-prompt",
      "pattern": "\\.env",
      "action": "prompt",
      "reason": "Environment files may contain secrets"
    },
    {
      "id": "aws-prompt",
      "pattern": "\\.aws",
      "action": "prompt",
      "reason": "AWS credentials directory"
    }
  ]
}
```

## Example: Enforcing Tool Usage

Block agents from using shell commands when proper tools exist:

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "cat-block",
      "pattern": "^cat\\s+",
      "action": "block",
      "reason": "Use the Read tool for file reading, not Bash",
      "guidance": "The Read tool provides proper encoding and line numbers"
    },
    {
      "id": "echo-redirect-block",
      "pattern": "echo.*>",
      "action": "block",
      "reason": "Use the Write tool for file writing, not Bash",
      "guidance": "The Write tool handles encoding and creates parent directories"
    }
  ]
}
```

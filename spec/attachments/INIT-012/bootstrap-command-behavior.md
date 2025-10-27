# Bootstrap Command Behavior - INIT-012

## Problem Statement

Currently, the slash command template (`.claude/commands/fspec.md` or `.cursor/commands/fspec.md`) contains multiple command lines that the AI agent must execute sequentially:

```markdown
# Current Template Structure (BEFORE)

<command-message>fspec is running…</command-message>
<command-name>/fspec</command-name>

IMMEDIATELY - run these commands and store them into your context and do not continue if any of them fail:

1. fspec --sync-version 0.6.0
2. fspec --help
3. fspec help specs
4. fspec help work
5. fspec help discovery
6. fspec help metrics
7. fspec help setup
8. fspec help hooks

YOU MUST RUN THOSE COMMANDS AND WAIT FOR THEM TO FINISH BEFORE CONTINUING ANY FURTHER.

[... rest of the documentation ...]
```

**Issues with current approach:**
- AI agent must execute 8 separate Bash commands
- Each command requires separate tool invocation
- Slower loading experience
- More token usage for command execution overhead
- Template file is large and static

## Proposed Solution

Refactor the template to contain **ONLY 2 commands**, and move all help command execution into a new `fspec bootstrap` command that runs everything internally:

```markdown
# New Template Structure (AFTER)

<command-message>fspec is running…</command-message>
<command-name>/fspec</command-name>

IMMEDIATELY - run these commands and store them into your context and do not continue if any of them fail:

1. fspec --sync-version 0.6.0
2. fspec bootstrap

YOU MUST RUN THOSE COMMANDS AND WAIT FOR THEM TO FINISH BEFORE CONTINUING ANY FURTHER.

[... rest of the documentation ...]
```

## How `fspec bootstrap` Works

The `fspec bootstrap` command will:

1. **Internally execute** all the help commands that were previously in the template:
   - `fspec --help`
   - `fspec help specs`
   - `fspec help work`
   - `fspec help discovery`
   - `fspec help metrics`
   - `fspec help setup`
   - `fspec help hooks`

2. **Collect the output** from each command

3. **Apply string replacement** using existing template generation functions:
   - Replace `<test-command>` with configured test command from `spec/fspec-config.json`
   - Replace `<quality-check-commands>` with configured quality check commands

4. **Output all documentation** in one combined response (exactly as it appears today after running all commands)

## Key Differences

### Before (Current Behavior)
```
AI Agent runs:
  → Bash: fspec --help
  → (waits for output)
  → Bash: fspec help specs
  → (waits for output)
  → Bash: fspec help work
  → (waits for output)
  ... (6 more command invocations)
```

### After (New Behavior)
```
AI Agent runs:
  → Bash: fspec bootstrap

Inside fspec bootstrap (TypeScript implementation):
  → Execute: fspec --help (internally, capture output)
  → Execute: fspec help specs (internally, capture output)
  → Execute: fspec help work (internally, capture output)
  ... (continue for all help commands)
  → Combine all outputs
  → Apply string replacements (<test-command>, <quality-check-commands>)
  → Print combined output to stdout
```

## Implementation Notes

1. **Reuse existing code**: The `fspec bootstrap` command should use the **same template generation functions** that currently generate the slash command files during `fspec init`

2. **Identical output**: The output from `fspec bootstrap` must be **byte-for-byte identical** to what currently appears in the template after the `fspec --sync-version` line

3. **String replacement**: Must replace placeholders with actual configured commands from `spec/fspec-config.json`:
   - `<test-command>` → e.g., "npm test"
   - `<quality-check-commands>` → e.g., "npm run format && npm run build && npm run lint"

4. **Template stays lightweight**: The slash command template file becomes much smaller (only 2 commands instead of 8)

5. **Dynamic content**: Documentation is always fresh because it's generated on-demand by the bootstrap command, not baked into a static file

## Benefits

✅ **Faster loading**: Single command invocation instead of 8 separate Bash calls
✅ **Lighter template**: Slash command file is smaller and easier to maintain
✅ **Always fresh**: Documentation comes from live command execution, not static text
✅ **Lower token usage**: Less overhead from multiple tool invocations
✅ **Easier updates**: Changes to help text don't require regenerating slash command files

## Migration Path

When users upgrade fspec and run `fspec init`:
- New template format is written automatically
- Old templates (with 8 commands) are replaced with new format (2 commands)
- No user action required

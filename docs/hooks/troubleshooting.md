# Hook Troubleshooting

This document explains common errors and how to fix them.

## Hook command not found

**Error**: `Hook command not found: spec/hooks/my-hook.sh`

### Cause

This error occurs when fspec cannot find the hook script at the specified path.

### Solutions

#### 1. Check file path is relative to project root

Hook command paths must be relative to the project root directory, not the location of `fspec-hooks.json`.

**Incorrect**:
```json
{
  "hooks": {
    "pre-implementing": [
      {
        "name": "lint",
        "command": "lint.sh"
      }
    ]
  }
}
```

**Correct**:
```json
{
  "hooks": {
    "pre-implementing": [
      {
        "name": "lint",
        "command": "spec/hooks/lint.sh"
      }
    ]
  }
}
```

#### 2. Verify file has execute permissions

Hook scripts must be executable.

```bash
# Check permissions
ls -l spec/hooks/lint.sh

# Add execute permission if needed
chmod +x spec/hooks/lint.sh
```

#### 3. Verify file exists

```bash
# Check if file exists
ls spec/hooks/lint.sh

# If missing, create the file
touch spec/hooks/lint.sh
chmod +x spec/hooks/lint.sh
```

### Testing hook script manually

Test your hook script outside of fspec to verify it works:

```bash
# Test with sample context
echo '{"workUnitId":"TEST-001","event":"pre-implementing","timestamp":"2025-01-15T10:00:00.000Z"}' | spec/hooks/lint.sh

# Check exit code
echo $?
```

## Hook timeout

**Error**: Hook times out and is killed

### Cause

Hook script runs longer than the configured timeout (default: 60 seconds).

### Solutions

#### 1. Increase timeout

```json
{
  "hooks": {
    "post-implementing": [
      {
        "name": "long-running-tests",
        "command": "spec/hooks/test.sh",
        "timeout": 300
      }
    ]
  }
}
```

#### 2. Optimize hook script

Make the hook script faster by:
- Running only necessary checks
- Using parallel execution
- Caching results
- Skipping redundant work

## Hook stderr not displayed

**Cause**: Non-blocking hooks display stderr as-is, but output may be hidden depending on your terminal.

### Solution

Ensure your hook writes errors to stderr:

```bash
#!/bin/bash
echo "Error message" >&2
exit 1
```

## Blocking hook not preventing command execution

**Cause**: The `blocking` field is set to `false` or omitted.

### Solution

Set `blocking: true` in your hook configuration:

```json
{
  "hooks": {
    "pre-implementing": [
      {
        "name": "validate",
        "command": "spec/hooks/validate.sh",
        "blocking": true
      }
    ]
  }
}
```

## Hook condition not matching

**Cause**: Work unit doesn't match the hook's condition.

### Solutions

#### 1. Check work unit tags

```bash
# Show work unit details
fspec show-work-unit AUTH-001

# Check if work unit has required tags
```

#### 2. Verify condition syntax

Condition fields use AND logic (all must match):

```json
{
  "condition": {
    "tags": ["@security"],
    "prefix": ["AUTH"]
  }
}
```

Within each field, use OR logic (any can match):

```json
{
  "condition": {
    "tags": ["@security", "@critical"]
  }
}
```

Hook runs if work unit has `@security` OR `@critical` tag.

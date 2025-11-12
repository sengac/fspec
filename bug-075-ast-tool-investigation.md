# BUG-075: AST Research Tool --file Flag Issue

## Problem Description

The `fspec research --tool=ast` command fails with error `--file is required` even when the `--file` flag is correctly provided.

## Steps to Reproduce

### Attempt 1: Relative Path
```bash
fspec research --tool=ast --operation=list-functions --file=src/git/diff.ts
```

**Error:**
```
Error: --file is required
```

### Attempt 2: Absolute Path
```bash
fspec research --tool=ast --operation=list-functions --file=/Users/rquast/projects/fspec/src/git/diff.ts
```

**Error:**
```
Error: --file is required
```

## Expected Behavior

According to `fspec research --tool=ast --help`:

```
OPTIONS
  --file <path>
    Specific file to analyze (required)
```

The command should accept the `--file` flag and analyze the specified file.

## Example from Help Documentation

```bash
$ fspec research --tool=ast --operation=list-functions --file=src/auth.ts
```

This example syntax matches what I attempted, but it still fails.

## Context

- Working on TUI-030: Handle binary files and large files in diff display
- Needed to analyze `src/git/diff.ts` to understand where to add binary file detection
- Used correct syntax based on help documentation
- Both relative and absolute paths fail with same error

## Investigation Needed

1. Check how the AST tool parses command-line arguments
2. Verify if there's a flag parsing issue (e.g., `--file` vs `--file=`)
3. Check if the tool script is being called correctly
4. Review AST tool implementation for argument handling

## Files to Investigate

- AST tool implementation (likely in `src/research-tools/ast/` or similar)
- Command-line argument parser
- Research command dispatcher

## Related Work Unit

This blocked progress on TUI-030 (Handle binary files and large files in diff display) during Example Mapping phase.

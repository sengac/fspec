# Bug Report: AST Research Tool Fails with Relative File Paths

## Issue Summary

The AST research tool fails when provided with relative file paths, throwing an `ENOENT: no such file or directory` error.

## Steps to Reproduce

1. Run the following command from the project root:
   ```bash
   fspec research --tool=ast --operation=find-pattern --file=src/cli.ts --pattern="command\("
   ```

2. Observe the error:
   ```
   Tool: ast
   Error: ENOENT: no such file or directory, open 'src/cli.ts'
   ```

## Expected Behavior

The AST research tool should resolve relative file paths from the current working directory and successfully analyze the file.

## Actual Behavior

The tool attempts to open 'src/cli.ts' as-is without resolving it relative to the current working directory, resulting in a file not found error.

## Workaround

Use absolute paths instead of relative paths:
```bash
fspec research --tool=ast --operation=find-pattern --file=/Users/rquast/projects/fspec/src/index.ts --pattern="\.command\("
```

## Environment

- Working directory: `/Users/rquast/projects/fspec`
- File being accessed: `src/cli.ts` (relative path)
- Command: `fspec research --tool=ast`

## Root Cause (Suspected)

The AST research tool likely doesn't resolve the `--file` parameter relative to `process.cwd()` before attempting to open the file.

## Suggested Fix

In the AST research tool implementation, resolve relative file paths using:
```typescript
import { resolve } from 'path';

const absolutePath = resolve(process.cwd(), options.file);
```

## Related Work

- Work Unit: BUG-083
- Discovery session needed to understand AST tool implementation

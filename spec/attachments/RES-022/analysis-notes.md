# RES-022 Bug Analysis: Research Tools Fail via Fspec Tool

## Summary

All 5 research tools (ast, perplexity, jira, confluence, stakeholder) fail when invoked via the Fspec tool because the research command handler reads arguments from `process.argv` instead of using the args passed by Commander.js.

## Scope Analysis: Is This Bug Unique to Research Command?

**Yes - the research command is the ONLY command with this issue.**

I searched the entire codebase for `process.argv` usage:

| File | Usage | Issue? |
|------|-------|--------|
| `src/commands/research.ts:304` | Reads args in command handler | **YES - BUG** |
| `src/index.ts:69-85` | Entry point, before Commander.js | No - correct |
| `src/utils/help-interceptor.ts:38` | Pre-Commander help interception | No - correct |
| Test files | Test setup only | No - not production |

**Why only research?** The research command is unique because:
1. It uses `allowUnknownOption()` to forward arbitrary args to tools
2. It needs to collect all non-tool-specific args and pass them to sub-tools
3. It incorrectly reads from `process.argv` instead of using Commander's `varArgs`

All other commands properly receive their arguments through Commander.js options/arguments, which work correctly via the Fspec tool.

## Root Cause

**Location:** `src/commands/research.ts` at line 304

```typescript
// Get all arguments after 'research --tool=<name>'
const allArgs = process.argv.slice(2);
```

### Why This Fails

1. **CLI works:** When running `fspec research --tool=ast --pattern="..."` directly, `process.argv` contains the command arguments ✓

2. **Fspec tool fails:** When invoked via the Fspec tool (through `fspecCallback` in `src/utils/fspec-callback.ts`):
   - Arguments are passed via Commander.js's `parseAsync(argv, { from: 'node' })`
   - The synthetic `argv` array is built correctly (line 653-691 in fspec-callback.ts)
   - But `process.argv` still contains whatever the original agent process was started with
   - The research command ignores the `varArgs` parameter from Commander.js and reads `process.argv` directly

### Code Flow

1. `fspecCallback()` builds argv: `['node', 'fspec', 'research', '--tool', 'ast', '--pattern', '...', '--lang', 'typescript']`
2. `program.parseAsync(argv, { from: 'node' })` is called
3. Commander.js routes to research command action with `varArgs` containing forwarded args
4. **BUG:** Research command ignores `varArgs` and reads `process.argv.slice(2)` which has wrong data

## Affected Tools

All 5 research tools are affected because they all rely on the research command handler to forward arguments:

| Tool | Status | Affected |
|------|--------|----------|
| ast | ✓ Ready | Yes |
| perplexity | ✗ Not configured | Yes |
| jira | ✗ Not configured | Yes |
| confluence | ✗ Not configured | Yes |
| stakeholder | ✗ Not configured | Yes |

## Individual Tools Are Correct

The individual tool implementations correctly receive and parse `args: string[]`:

- **ast.ts** (line 16): `async execute(args: string[]): Promise<string>`
- **perplexity.ts** (line 175): `async execute(args: string[]): Promise<string>`
- **jira.ts** (line 308): `async execute(args: string[]): Promise<string>`
- **confluence.ts** (line 220): `async execute(args: string[]): Promise<string>`
- **stakeholder.ts** (line 94): `async execute(args: string[]): Promise<string>`

The problem is the research command handler doesn't pass the correct args to these tools.

## Proposed Fix

In `src/commands/research.ts`, the action handler should use the `varArgs` parameter passed by Commander.js instead of reading `process.argv`:

**Current (buggy):**
```typescript
.action(async (varArgs: string[], options: any) => {
  // ...
  // Get all arguments after 'research --tool=<name>'
  const allArgs = process.argv.slice(2);  // BUG: ignores varArgs
```

**Fixed:**
```typescript
.action(async (varArgs: string[], options: any) => {
  // ...
  // Use varArgs from Commander.js (works for both CLI and Fspec tool)
  // varArgs contains all unknown options that Commander forwarded
  const forwardedArgs: string[] = [...varArgs];
```

However, we need to be careful because `varArgs` may need processing to include the `--tool` and other known options. Need to verify what Commander.js passes in `varArgs` vs `options`.

## Test Case

```typescript
// Via Fspec tool (currently fails)
command: "research"
args: {"_": ["--tool=ast", "--pattern=async function $NAME($$$ARGS)", "--lang=typescript", "--path=src/"]}

// Expected: AST search results
// Actual: Error: --pattern is required
```

## Files to Modify

1. `src/commands/research.ts` - Fix argument forwarding logic
2. `src/commands/__tests__/research*.test.ts` - Add test for Fspec tool invocation

## Related Code References

- `src/utils/fspec-callback.ts` (lines 653-697) - argv building logic
- `src/research-tools/registry.ts` - Tool loading
- `src/research-tools/types.ts` - ResearchTool interface

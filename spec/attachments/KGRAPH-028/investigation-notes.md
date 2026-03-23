# KGRAPH-028: Phantom Stub File Nodes — Investigation Notes

## Date: 2026-03-24

## Summary

After the barrel import fix (previous session) was confirmed working and the AST graph was reindexed, 3 phantom stub File nodes remain. These are **not** stale index artifacts — they are freshly created on every reindex because the source code contains imports to files that don't exist on disk.

## Phantom Stubs Found

| Phantom Stub Path | Slug | Properties |
|---|---|---|
| `src/types/work-units.ts` | `src-types-work-units-ts` | `isTest: null, language: null, lineCount: null` |
| `src/types/work-unit.ts` | `src-types-work-unit-ts` | `isTest: null, language: null, lineCount: null` |
| `src/tui/hooks/lazyLineIndex.ts` | `src-tui-hooks-lazyLineIndex-ts` | `isTest: null, language: null, lineCount: null` |

All three confirmed non-existent on disk:

```
$ ls -la src/types/work-units.ts src/types/work-unit.ts src/tui/hooks/lazyLineIndex.ts
ls: cannot access 'src/types/work-units.ts': No such file or directory
ls: cannot access 'src/types/work-unit.ts': No such file or directory
ls: cannot access 'src/tui/hooks/lazyLineIndex.ts': No such file or directory
```

## Root Cause

The AST extractor encounters import statements referencing these files, resolves the path, finds no matching file in the known-files set, and creates a stub File node as the Import edge target. This is **correct extractor behavior** — the bug is in the source code having stale imports.

## Affected Source Files

### `src/types/work-units.ts` (4 importers)

```
src/commands/link-coverage.ts:6:       import type { WorkUnitType } from '../types/work-units';
src/commands/link-coverage/step-validator.ts:8: import { WorkUnitType } from '../../types/work-units';
src/commands/link-coverage/utils.ts:3:          import { WorkUnitType } from '../../types/work-units';
src/hooks/integration.ts:16:                    import type { WorkUnit } from '../types/work-units';
```

### `src/types/work-unit.ts` (2 importers)

```
src/commands/query-bottlenecks.ts:1:            import type { WorkUnitsData, WorkUnit } from '../types/work-unit';
src/commands/discover-foundation.ts:19:         import type { WorkUnitsData } from '../types/work-unit';
```

### `src/tui/hooks/lazyLineIndex.ts` (1 importer)

```
src/tui/hooks/useLazyConversationLines.ts:29:   import { LazyLineIndex, createLazyLineIndex } from './lazyLineIndex';
```

## Why the Project Still Builds

These are likely resolved by TypeScript/Vite at build time through:
1. **Type-only imports** (`import type`) get erased entirely by the TypeScript compiler
2. **Path aliasing or re-exports** — the types may be re-exported from `src/types/index.ts` (the barrel), so Vite resolves them differently
3. The actual types (`WorkUnitType`, `WorkUnit`, `WorkUnitsData`) are defined in `src/types/index.ts` which has 221 lines and contains these exact interfaces

## Fix Approach

1. For `work-units.ts` and `work-unit.ts` imports: update to import from `../types` (the barrel) or wherever the types actually live now
2. For `lazyLineIndex.ts`: find where `LazyLineIndex` and `createLazyLineIndex` were moved to (likely `src/tui/utils/lazyLineIndex.ts` which exists with 482 lines) and update the import path
3. Verify build + tests still pass after each change

## Distinct from Barrel Import Bug

The barrel import bug (fixed in the previous session) was about `resolve_import_path()` not trying `foo/index.ts` when `foo.ts` didn't exist and the import path had no extension. That created phantoms for **every** barrel import (`../types`, `../commands/schedule`, etc.).

This issue is different: the files genuinely don't exist anywhere. The imports are pointing to old paths from before a file rename/move.

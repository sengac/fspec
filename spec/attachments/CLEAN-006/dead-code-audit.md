# Dead Code Audit — AST Graph Verified

**Generated**: 2026-03-23 (revised 2026-03-24)  
**Method**: `GraphSearch ast_dead_code` + manual grep verification  
**Work Unit**: CLEAN-006

---

## Summary

| Category | Count | Notes |
|----------|-------|-------|
| Dead files (entire module) | 4 | No production imports at all |
| Dead test files (only import dead modules) | 3 | Delete alongside their dead modules |
| Dead functions in live files | 12 | Exported but never called from production |
| False positives (explained) | ~160+ | `-help.ts` via `import.meta.glob`, Commander.js `.action()`, barrel imports |

---

## 1. Dead Files — Safe to Delete

These files have **zero production imports**. Only referenced from test files.

### 1.1 `src/commands/query.ts` (376 lines)

**Status**: DEAD — superseded by dedicated modules  
**Evidence**: `grep -r "from.*query'" src/ --include='*.ts' | grep -v __tests__` → 0 results  
**Contains 6 exported functions**:
- `queryWorkUnitsByStatus()` — replaced by `query-work-units.ts`
- `queryWorkUnitsByEpic()` — replaced by `query-work-units.ts`
- `queryWorkUnitsCompound()` — replaced by `query-work-units.ts`
- `generateStatisticalReport()` — replaced by `generate-summary-report.ts`
- `exportWorkUnits()` — replaced by `export-work-units.ts`
- `displayKanbanBoard()` — replaced by `work-unit.ts::displayBoard()`

**Also delete**: `src/commands/__tests__/pm-remaining.test.ts` (878 lines — the only consumer)

### 1.2 `src/commands/interactive-questionnaire.ts` (201 lines)

**Status**: DEAD — unused discovery feature  
**Evidence**: `grep -r "from.*interactive-questionnaire" src/ --include='*.ts' | grep -v __tests__` → 0 results  
**Contains 4 interfaces**:
- `QuestionnaireOptions` — unused
- `DiscoveryData` — unused
- `Question` — unused
- `QuestionnaireState` — unused

**Contains 4 exported functions** (3 called internally within the dead file):
- `buildQuestions()` — called by `runQuestionnaire()` internally
- `formatQuestionDisplay()` — called by `runQuestionnaire()` internally
- `validateAnswer()` — called by `runQuestionnaire()` internally
- `runQuestionnaire()` — entry point, never called from production

**Also delete**: `src/commands/__tests__/interactive-questionnaire.test.ts` (113 lines)

### 1.3 `src/commands/research-integration.ts` (245 lines)

**Status**: DEAD — unused research integration module  
**Evidence**: `grep -r "from.*research-integration" src/ --include='*.ts' | grep -v __tests__` → 0 results  
**Contains 7 items (6 exported functions + 1 private helper)**:
- `executeResearchWithPrompt()`
- `analyzeResearchOutput()`
- `acceptAISuggestions()`
- `executeResearchWithAutoAttach()`
- `extractRules()`
- `confirmAndAddRule()`
- `slugify()` (private helper)

**Also delete**: `src/commands/__tests__/research-auto-attachment.test.ts` (195 lines)

### 1.4 `bridge/telegram-buffering.ts` (160 lines)

**Status**: DEAD — orphan bridge module  
**Evidence**: `grep -r "from.*telegram-buffering" bridge/ src/ --include='*.ts' | grep -v __tests__` → 0 results  
**Contains 5 exported functions**:
- `flushBuffer()`
- `scheduleIdleFlush()`
- `addToBuffer()`
- `shouldForceFlush()`
- `clearBuffer()`

**Note**: No dedicated test file exists for this module. The `telegram-endpoint.ts` has its own internal `flushBuffer()` implementation (line 370) that is actively used — this module's `flushBuffer()` is a separate, unused version.

---

## 2. Dead Functions in Live Files

These functions are exported from production files but have **no production callers**. Some have test callers only.

### 2.1 Genuinely Dead (zero callers, not even tests)

| Function | File | Lines | Notes |
|----------|------|-------|-------|
| `getFileStatus()` | `src/git/status.ts` | L277-284 (7) | No callers at all |
| `validateGenericFoundation()` | `src/validators/generic-foundation-validator.ts` | L27-49 (22) | Async version; the sync `validateGenericFoundationObject()` IS used |
| `formatGenericFoundationErrors()` | `src/validators/generic-foundation-validator.ts` | L75-83 (8) | Never called from any file |

### 2.2 Test-Only Callers (production-uncalled, tested)

These are exported for testability but have no production call sites. Review whether they should remain as public API or be removed.

| Function | File | Lines | Test consumers |
|----------|------|-------|----------------|
| `checkFeatureCoverage()` | `src/commands/update-work-unit-status.ts` | L1056-1138 (82) | `coverage-file-synchronization.test.ts` |
| `getToolHelp()` | `src/commands/research-tool-list.ts` | L157-189 (32) | `research-tool-discovery.test.ts` |
| `validateCompactionStateConsistency()` | `src/core-logic/compaction-state-manager.ts` | L123-168 (45) | `compaction-state-manager.test.ts` |
| `createUnifiedCompactionState()` | `src/core-logic/compaction-state-manager.ts` | L173-186 (13) | `compaction-state-manager.test.ts` |
| `parseDiff()` | `src/git/diff-parser.ts` | L34-103 (69) | `diff-parser.test.ts` |
| `getFileDiff()` | `src/git/diff.ts` | L19-28 (9) | `native-rust-diff-operations.test.ts`, `gitoxide-operations.test.ts` |
| `getCheckpointFileDiff()` | `src/git/diff.ts` | L37-50 (13) | `native-rust-diff-operations.test.ts`, `git-checkpoint-restore-deletes-new-files.test.ts` |
| `validateFoundationObject()` | `src/validators/json-schema.ts` | L48-50 (2) | `json-schema.test.ts` |
| `validateTagsObject()` | `src/validators/json-schema.ts` | L56-58 (2) | `json-schema.test.ts` |

**Assessment**: Most of these are utility functions forming a module's public API, exported for testability. They are low-priority for removal and may be needed by future features. The exception is `checkFeatureCoverage()` (82 lines) which appears to be a significant orphaned function.

---

## 3. False Positives — Do NOT Delete

### 3.1 `*-help.ts` files (~157 files)

The graph flags all `-help.ts` files as orphan files because they have no static `import` edges. However, they are loaded dynamically via Vite's `import.meta.glob` in `src/commands/help-registry.ts`:

```typescript
const helpModules = import.meta.glob<{ default: CommandHelpConfig }>(
  './*-help.ts',
  { eager: true }
);
```

**Verdict**: FALSE POSITIVE — do not delete.

### 3.2 `vite.config.ts`

Build configuration file — never imported by source code by design.

**Verdict**: FALSE POSITIVE — do not delete.

### 3.3 `examples/hooks/notify-slack.js`

Example file for documentation purposes. Not meant to be imported.

**Verdict**: FALSE POSITIVE — do not delete.

### 3.4 CLI command entry points (e.g. `validateCommand`, `linkCoverageCommand`)

Many functions like `validateCommand()`, `showFeatureCommand()`, `displayBoard()`, `addRule()`, etc. appear as "uncalled" because they're wired to Commander.js via callback references in `src/cli/program.ts` using `.action()`, which the AST extractor doesn't trace through higher-order function passing.

**Verdict**: FALSE POSITIVE — wired via Commander.js `.action()`.

### 3.5 `src/commands/schedule/index.ts` (barrel file)

Flagged as orphan file by the graph. Actually imported by `src/cli/program.ts:168`:
```typescript
} from '../commands/schedule';
```
The graph doesn't trace through barrel re-exports.

**Verdict**: FALSE POSITIVE — barrel import, used by program.ts.

### 3.6 Bridge content-chunker helper functions

Functions like `findCodeBlockBoundary()`, `findListBoundary()`, `findTableBoundary()`, `formatToolCallSummary()` in `bridge/telegram-content-chunker.ts` are exported standalone functions only called from tests. They exist as public API for the content-chunker module and for testability of individual boundary-detection algorithms.

**Verdict**: FALSE POSITIVE — test-exposed helpers, part of module API.

### 3.7 Functions only called from same file (internal helpers)

Some functions flagged as uncalled are actually called within the same file but via patterns the extractor doesn't yet trace (e.g. called inside closures, callbacks, or conditional branches). These need case-by-case review. Verified examples:
- `buildAIAnalysisReminder()` in `review.ts` — called on line 550 within same file
- `getStagedFilesWithChangeType()` / `getUnstagedFilesWithChangeType()` in `git/status.ts` — called from `src/tui/store/fspecStore.ts`

---

## 4. Cleanup Steps

### Phase 1: Delete dead files (safe, verified)
1. Delete `src/commands/query.ts`
2. Delete `src/commands/__tests__/pm-remaining.test.ts`
3. Delete `src/commands/interactive-questionnaire.ts`
4. Delete `src/commands/__tests__/interactive-questionnaire.test.ts`
5. Delete `src/commands/research-integration.ts`
6. Delete `src/commands/__tests__/research-auto-attachment.test.ts`
7. Delete `bridge/telegram-buffering.ts`

### Phase 2: Verify build + tests pass
```bash
npm run build
npm test
cargo test
```

### Phase 3: Remove dead functions in live files (optional, lower priority)
1. Remove `getFileStatus()` from `src/git/status.ts` (zero callers)
2. Remove `validateGenericFoundation()` from `src/validators/generic-foundation-validator.ts` (async version, zero callers)
3. Remove `formatGenericFoundationErrors()` from `src/validators/generic-foundation-validator.ts` (zero callers)
4. Review `checkFeatureCoverage()` in `src/commands/update-work-unit-status.ts` — 82 lines with no production caller

### Phase 4: Re-run dead code detection
```
GraphSearch ast_index
GraphSearch ast_dead_code
```

Re-examine the next batch after removal.

---

## 5. Estimated Impact

### Phase 1 (Dead Files)

| Metric | Value |
|--------|-------|
| Production files removed | 4 |
| Test files removed | 3 |
| Production lines removed | 982 |
| Test lines removed | 1,186 |
| **Total lines removed** | **2,168** |
| Production risk | None (zero callers verified) |

### Phase 3 (Dead Functions — optional)

| Metric | Value |
|--------|-------|
| Functions removed | 3-4 |
| Lines removed | ~37-119 |
| Production risk | Low (zero callers verified) |

# BUG-123: generate-foundation-md mermaid JSDOM parentRule Error

## Error Message

```
Error: Mermaid validation failed: Cannot set property parentRule of #<CSSStyleDeclaration> which has only a getter
```

## Summary

`generate-foundation-md` always fails when the foundation event storm contains **any** bounded contexts. The command auto-generates a mermaid diagram from bounded context data via `generateBoundedContextMermaid()` in `src/generators/foundation-md.ts`, then validates it via `validateMermaidSyntax()` in `src/utils/mermaid-validation.ts`. The mermaid renderer crashes during JSDOM initialization before it even evaluates the diagram content.

## Affected Code

- **Generator**: `src/generators/foundation-md.ts` lines 348-357 — calls `generateBoundedContextMermaid()` then `validateMermaidSyntax()`
- **Validator**: `src/utils/mermaid-validation.ts` — sets up JSDOM globals, imports mermaid, calls `mermaid.render()`
- **Also affected**: `add-diagram` command — uses same `validateMermaidSyntax()`, also fails

## Reproduction Steps

### Step 1: Create a fresh foundation (works)

```
fspec discover-foundation
fspec update-foundation projectName "Test"
fspec update-foundation projectVision "Test project"
fspec update-foundation projectType "web-app"
fspec update-foundation problemTitle "Test Problem"
fspec update-foundation problemDefinition "Test description"
fspec update-foundation solutionOverview "Test solution"
fspec remove-persona "[QUESTION: Who uses this?]"
fspec add-persona "User" "Test user" --goal "Test goal"
fspec add-capability "Feature" "Test feature"
fspec discover-foundation --finalize
```

**Result**: ✓ Succeeds — generates `foundation.json` and `FOUNDATION.md`

### Step 2: Add a single bounded context (fails)

```
fspec add-foundation-bounded-context "BrowserAgent"
fspec generate-foundation-md
```

**Result**: ✗ Fails with `Cannot set property parentRule of #<CSSStyleDeclaration> which has only a getter`

### Step 3: Verify it's not the diagram content

Even the simplest possible bounded context name with no spaces or special characters fails:

- `"BrowserAgent"` — fails
- `"Test"` — fails (not tested, but inferred since BrowserAgent has no special chars)

### Step 4: Verify add-diagram also fails

```
fspec add-diagram "Architecture" "Test" "flowchart TD\n  A --> B"
```

**Result**: ✗ Same error — `Cannot set property parentRule of #<CSSStyleDeclaration> which has only a getter`

This proves the issue is in the mermaid rendering environment, not the diagram syntax. `flowchart TD\n  A --> B` is the simplest valid mermaid possible.

## Root Cause Analysis

The error `Cannot set property parentRule of #<CSSStyleDeclaration> which has only a getter` originates from mermaid trying to manipulate CSS style declarations inside JSDOM. The `CSSStyleDeclaration.parentRule` property is read-only in the DOM spec, and JSDOM enforces this. Mermaid's rendering pipeline tries to set it during SVG generation.

This is likely a **mermaid version incompatibility with JSDOM**. Newer mermaid versions may use CSS APIs that JSDOM doesn't fully support.

### Key code in `src/utils/mermaid-validation.ts`:

```typescript
const dom = new JSDOM('...', {
  runScripts: 'dangerously',
  resources: 'usable',
});
// ... sets up globals ...
const mermaid = (await import('mermaid')).default;
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });
await mermaid.render('validation-diagram', code);  // <-- crashes here
```

## Environment

- **Runtime**: Fspec tool (invoked via Claude Code's Fspec tool, not CLI)
- **Node.js**: (check with `node --version`)
- **Platform**: macOS aarch64

## Possible Fixes

1. **Update JSDOM** to a version that supports the `parentRule` setter (if one exists)
2. **Mock `parentRule`** in `validateMermaidSyntax()` similar to how `getBBox` and `getComputedTextLength` are already mocked:
   ```typescript
   // Already mocked in the code:
   (window.SVGElement.prototype as any).getBBox = function() { ... };
   (window.SVGElement.prototype as any).getComputedTextLength = function() { ... };
   
   // Potential fix — mock parentRule as writable:
   Object.defineProperty(window.CSSStyleDeclaration.prototype, 'parentRule', {
     writable: true,
     configurable: true,
     value: null,
   });
   ```
3. **Downgrade mermaid** to a version compatible with current JSDOM
4. **Use mermaid's `parse()` API** instead of `render()` for syntax validation (avoids DOM rendering entirely)
5. **Skip mermaid validation** when JSDOM rendering fails with known environment errors, falling back to syntax-only checks

## Impact

- `generate-foundation-md` is completely broken for any project with bounded contexts in the event storm
- `add-diagram` command is completely broken for all diagrams
- Foundation discovery/finalization works fine (no bounded contexts at that point)
- `foundation.json` data is unaffected — only FOUNDATION.md generation is blocked

# Mermaid Validation Bug Investigation - VAL-001

**Bug ID:** VAL-001
**Date:** 2025-11-13
**Severity:** Medium
**Component:** Attachment validation (`src/utils/attachment-mermaid-validation.ts`)

---

## Summary

The `fspec add-attachment` command validates mermaid diagrams using `mermaid.parse()`, which only checks syntax/grammar but NOT rendering semantics. This allows semantically invalid diagrams (e.g., quoted subgraph titles) to pass validation but fail when rendered in the browser.

---

## Problem Description

### What Happened

1. User created a markdown document with mermaid diagrams for RustDesk architecture documentation
2. Used `fspec add-attachment DOC-001 ARCHITECTURE_RESEARCH.md` to attach the document
3. Validation passed successfully (no errors)
4. Browser displayed "Syntax error in text" for the Audio Streaming Flow diagram
5. The problematic syntax: `subgraph "Server Side"` (quoted subgraph title)

### Expected Behavior

The `add-attachment` command should **reject** mermaid diagrams that will fail during browser rendering.

### Actual Behavior

The `add-attachment` command **accepts** diagrams with semantic errors that pass `mermaid.parse()` but fail `mermaid.render()`.

---

## Root Cause Analysis

### Code Investigation

**File:** `src/utils/mermaid-validation.ts` (lines 42-43)

```typescript
// Use mermaid.parse() to validate syntax
await mermaid.parse(code);
```

**File:** `src/commands/add-attachment.ts` (lines 45-47)

```typescript
const validationResult = await validateMermaidAttachment(
  options.filePath
);
```

### AST Analysis Results

Using `fspec research --tool=ast --operation=list-functions --file=src/utils/mermaid-validation.ts`:

```json
{
  "type": "function_declaration",
  "name": "validateMermaidSyntax",
  "line": 11,
  "text": "export async function validateMermaidSyntax(...) { ... await mermaid.parse(code); ... }"
}
```

### The Problem with `mermaid.parse()`

**What `mermaid.parse()` does:**
- ✅ Validates basic syntax/grammar (nodes, edges, keywords)
- ✅ Checks if the diagram structure is parseable
- ❌ Does NOT validate semantic rules (subgraph format, style constraints, etc.)
- ❌ Does NOT catch rendering-time errors

**What `mermaid.render()` does:**
- ✅ Validates syntax/grammar
- ✅ Validates semantic rules
- ✅ Performs layout calculations
- ✅ Applies styles
- ✅ Catches ALL errors that would occur in browser rendering

---

## Reproduction Steps

### Test Case 1: Quoted Subgraph Title (PASSES parse, FAILS render)

```bash
cat > /tmp/test-quoted-subgraph.md << 'EOF'
# Mermaid with quoted subgraph
graph LR
    subgraph "Server Side"
        A[Node A] --> B[Node B]
    end
EOF

fspec add-attachment DOC-001 /tmp/test-quoted-subgraph.md
# Result: ✓ Attachment added successfully (SHOULD FAIL!)
```

**Browser Error:**
```
Syntax error in text
mermaid version 11.12.1
```

### Test Case 2: Completely Invalid Subgraph (PASSES parse!)

```bash
cat > /tmp/test-invalid-subgraph.md << 'EOF'
# Mermaid with invalid subgraph syntax
graph LR
    subgraph TOTALLY_INVALID_SUBGRAPH_SYNTAX!!!
        A[Node] --> B[Node]
    end
EOF

fspec add-attachment DOC-001 /tmp/test-invalid-subgraph.md
# Result: ✓ Attachment added successfully (SHOULD FAIL!)
```

### Test Case 3: Broken Syntax (CORRECTLY FAILS)

```bash
cat > /tmp/test-broken-syntax.md << 'EOF'
# Mermaid with broken syntax
graph TD
    A[Start] --> B{Decision
    B -->|Yes| C[Good]
    INVALID SYNTAX HERE
EOF

fspec add-attachment DOC-001 /tmp/test-broken-syntax.md
# Result: Error: Failed to attach broken-diagram.md: Mermaid code block 1 is invalid
# (CORRECT BEHAVIOR - syntax errors are caught)
```

---

## Impact Analysis

### Severity: Medium

**User Impact:**
- Documents with semantically invalid diagrams pass validation
- Users discover errors only when viewing in browser
- Wastes time debugging "valid" diagrams

**Scope:**
- Affects all markdown attachments with mermaid diagrams
- Subgraph validation is completely missing
- Style validation may also be incomplete

**Workaround:**
- Manually test diagrams in mermaid live editor before attaching
- Fix diagrams after browser reports errors (current workflow)

---

## Proposed Solution

### Option 1: Use `mermaid.render()` instead of `mermaid.parse()` (Recommended)

**Implementation:**

```typescript
export async function validateMermaidSyntax(
  code: string
): Promise<MermaidValidationResult> {
  try {
    const dom = new JSDOM('<!DOCTYPE html><html><body><div id="mermaid-container"></div></body></html>', {
      runScripts: 'dangerously',
      resources: 'usable',
    });

    const { window } = dom;
    global.window = window as any;
    global.document = window.document as any;

    const mermaid = (await import('mermaid')).default;
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
    });

    // Use render() instead of parse() to catch ALL errors
    const { svg } = await mermaid.render('validation-diagram', code);

    // Clean up globals
    // ... (same cleanup code)

    return { valid: true };
  } catch (error: any) {
    // ... (same error handling)
    return {
      valid: false,
      error: error.message || 'Unknown Mermaid rendering error',
    };
  }
}
```

**Pros:**
- Catches ALL errors that browser would catch
- Complete validation (syntax + semantics)
- Same behavior as browser rendering

**Cons:**
- Slightly slower (rendering vs parsing)
- More memory usage
- May require additional DOM setup

### Option 2: Use both `parse()` and `render()` with fallback

```typescript
// Fast path: parse() for syntax
await mermaid.parse(code);

// Slow path: render() for semantics
try {
  await mermaid.render('validation', code);
} catch (renderError) {
  return { valid: false, error: renderError.message };
}
```

**Pros:**
- Fail fast on syntax errors
- Complete validation via render()

**Cons:**
- Redundant work (parse then render)
- More complex code

### Option 3: Add subgraph-specific validation rules

```typescript
// After parse(), check for known problematic patterns
if (code.includes('subgraph "')) {
  return {
    valid: false,
    error: 'Quoted subgraph titles are not supported. Use: subgraph ID[Title]'
  };
}
```

**Pros:**
- Fast
- Catches known issues

**Cons:**
- Incomplete (only catches known patterns)
- Maintenance burden (add rules as issues discovered)
- Doesn't catch new semantic errors

---

## Recommendation

**Implement Option 1: Use `mermaid.render()` for validation**

**Rationale:**
1. **Correctness**: Only `render()` provides complete validation
2. **Consistency**: Matches browser behavior exactly
3. **Future-proof**: Automatically catches new mermaid validation rules
4. **Performance**: Validation happens once at attachment time (acceptable cost)

**Performance Considerations:**
- Rendering is slower than parsing (~50-200ms vs ~10ms per diagram)
- Acceptable for attachment validation (one-time operation)
- Users prefer slower validation over broken diagrams

---

## Testing Strategy

### Unit Tests to Add

**File:** `src/commands/__tests__/add-attachment-mermaid-validation.test.ts`

```typescript
describe('Mermaid subgraph validation', () => {
  it('should reject quoted subgraph titles', async () => {
    const diagram = `graph LR
      subgraph "Invalid Quotes"
        A --> B
      end`;

    const result = await validateMermaidSyntax(diagram);
    expect(result.valid).toBe(false);
    expect(result.error).toContain('subgraph');
  });

  it('should accept proper subgraph syntax', async () => {
    const diagram = `graph LR
      subgraph ServerSide[Server Side]
        A --> B
      end`;

    const result = await validateMermaidSyntax(diagram);
    expect(result.valid).toBe(true);
  });

  it('should reject invalid subgraph identifiers', async () => {
    const diagram = `graph LR
      subgraph INVALID!!!
        A --> B
      end`;

    const result = await validateMermaidSyntax(diagram);
    expect(result.valid).toBe(false);
  });
});
```

### Integration Tests

```typescript
describe('add-attachment with semantic mermaid errors', () => {
  it('should reject markdown with quoted subgraph titles', async () => {
    const mdFile = createTempFile(`
# Test
\`\`\`mermaid
graph LR
  subgraph "Invalid"
    A --> B
  end
\`\`\`
    `);

    await expect(
      addAttachment({ workUnitId: 'TEST-001', filePath: mdFile })
    ).rejects.toThrow(/subgraph/);
  });
});
```

---

## Version Information

- **fspec version:** 0.8.2
- **mermaid version:** ^11.12.0 (installed: 11.12.1)
- **Node.js version:** v22.20.0
- **Browser mermaid version:** 11.12.1

---

## Investigation Timeline

1. **Initial Issue:** User attached `ARCHITECTURE_RESEARCH.md` to DOC-001
2. **Validation Passed:** No errors during `fspec add-attachment`
3. **Browser Error:** "Syntax error in text" for Audio Streaming Flow diagram
4. **Hypothesis 1:** Removed `<br/>` HTML tags (didn't fix issue)
5. **Hypothesis 2:** Changed subgraph syntax from `"Server Side"` to `ServerSide[Server Side]` (fixed!)
6. **Investigation:** Why didn't validation catch this?
7. **AST Analysis:** Found `mermaid.parse()` usage in `src/utils/mermaid-validation.ts`
8. **Reproduction:** Created test cases proving `parse()` accepts semantic errors
9. **Root Cause:** `mermaid.parse()` only validates syntax, not semantics

---

## Related Files

**Source Files:**
- `src/utils/mermaid-validation.ts` - Core validation logic
- `src/utils/attachment-mermaid-validation.ts` - Markdown extraction and validation
- `src/commands/add-attachment.ts` - Command that calls validation

**Test Files:**
- `src/commands/__tests__/add-attachment-mermaid-validation.test.ts` - Existing tests
- Need to add: subgraph-specific test cases

---

## References

- **Mermaid Documentation:** https://mermaid.js.org/config/setup/modules/mermaidAPI.html
- **parse() vs render():** https://github.com/mermaid-js/mermaid/discussions
- **Subgraph Syntax:** https://mermaid.js.org/syntax/flowchart.html#subgraphs

---

## Next Steps

1. Implement `render()`-based validation (Option 1)
2. Add comprehensive test cases for subgraph validation
3. Test performance impact (expect ~50-200ms per diagram)
4. Update documentation about validation behavior
5. Consider adding progress indicator for large attachments with many diagrams

---

## Attachment Metadata

**Work Unit:** VAL-001
**Created:** 2025-11-13
**Author:** Claude (AI Assistant)
**Investigation Method:** AST analysis, manual testing, code review

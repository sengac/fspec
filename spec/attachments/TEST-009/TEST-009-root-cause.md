# TEST-009 Root Cause Analysis

## Problem
Test `generate-event-storm-section-in-foundation-md.test.ts` is failing because it expects `generateFoundationMd()` to throw an error for invalid Mermaid syntax in Event Storm diagrams, but the function never throws.

## Root Cause
The `generate-foundation-md.ts` command validates `architectureDiagrams` Mermaid syntax (lines 61-76) but does **NOT validate the Event Storm Mermaid diagram** that gets auto-generated from `eventStorm.items` data.

**Code evidence (src/commands/generate-foundation-md.ts:58-76):**
```typescript
// Validate all Mermaid diagrams
const diagramErrors: string[] = [];
if (
  foundationData.architectureDiagrams &&
  foundationData.architectureDiagrams.length > 0
) {
  for (let i = 0; i < foundationData.architectureDiagrams.length; i++) {
    const diagram = foundationData.architectureDiagrams[i];
    const validationResult = await validateMermaidSyntax(
      diagram.mermaidCode
    );
    // ... error handling
  }
}
// Event Storm diagram validation is MISSING!
```

The Event Storm Mermaid diagram is generated dynamically from bounded context names in `eventStorm.items[]`, but this generated diagram is never validated before being written to FOUNDATION.md.

## Impact
- Invalid Event Storm data can create broken Mermaid diagrams in FOUNDATION.md
- No validation error when bounded context names contain malformed syntax
- Test cannot verify Mermaid validation behavior for Event Storm diagrams

## Fix Required
Add Event Storm Mermaid diagram validation before generating FOUNDATION.md:
1. Generate Event Storm Mermaid diagram from `eventStorm.items`
2. Validate the generated diagram using `validateMermaidSyntax()`
3. Include any validation errors in `diagramErrors[]`
4. Throw error if validation fails (same as architectureDiagrams)

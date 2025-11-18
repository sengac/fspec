# Bug Report: Missing Auto-Creation of Foundation Event Storm Work Unit

## Issue Summary

When running `fspec discover-foundation --finalize` successfully, the system does NOT automatically create a work unit for "Foundation Event Storm" as documented in the bootstrap output.

## Expected Behavior

According to the `fspec bootstrap` command output (Step 2: Bootstrap Foundation section), the expected behavior is:

```bash
$ fspec discover-foundation --finalize
✓ Generated spec/foundation.json
✓ Created work unit FOUND-XXX: Foundation Event Storm
```

The documentation explicitly states:

> "Foundation discovery complete
> fspec discover-foundation --finalize
> ✓ Generated spec/foundation.json
> ✓ Created work unit FOUND-XXX: Foundation Event Storm"

And later in Step 3 (Foundation Event Storm section):

> "Typical workflow:
> ```bash
> # 1. Foundation discovery complete
> fspec discover-foundation --finalize
> ✓ Generated spec/foundation.json
> ✓ Created work unit FOUND-XXX: Foundation Event Storm
> ```"

## Actual Behavior

When running `fspec discover-foundation --finalize`, the command only outputs:

```bash
✓ Generated spec/foundation.json
✓ Generated spec/FOUNDATION.md
✓ Foundation discovered and validated successfully
```

**No work unit is created.**

## Reproduction Steps

1. In a new project (mindstrike), run: `fspec discover-foundation`
2. Fill all foundation fields using `fspec update-foundation` commands and `fspec add-persona`, `fspec add-capability`
3. Run: `fspec discover-foundation --finalize`
4. Check work units: `fspec list-work-units`
5. **Result**: No FOUND-XXX work unit exists

## Evidence

### Command Output
```bash
$ fspec discover-foundation --finalize
✓ Generated spec/foundation.json
✓ Generated spec/FOUNDATION.md
✓ Foundation discovered and validated successfully
```

### Work Units List
After finalization, running `fspec list-work-units` shows:
- AGENT-001, AGENT-002, BUG-001, BUG-002, UI-001, UI-002, CLI-001, CORE-001, UI-003, BUG-003, UI-004
- **No FOUND-XXX work unit present**

### Attempted Manual Creation Failed
When attempting to manually create the work unit:

```bash
$ fspec create-story FOUND "Foundation Event Storm" --description "..."
Error: Prefix 'FOUND' is not registered. Run 'fspec create-prefix FOUND "Description"' first.
```

This suggests:
1. The auto-creation feature is completely missing, OR
2. The prefix 'FOUND' should be auto-registered but isn't

## Impact

- **High**: Breaks documented Foundation Event Storm workflow
- AI agents following the bootstrap documentation will be confused
- Users must manually create the work unit and prefix
- Workflow continuity is broken between foundation discovery and Event Storm

## Root Cause Analysis (Hypothesis)

The issue is likely in `src/commands/discover-foundation.ts` in the `--finalize` logic. The finalization code probably:

1. ✅ Validates the draft
2. ✅ Creates `foundation.json`
3. ✅ Generates `FOUNDATION.md`
4. ❌ **MISSING**: Creates 'FOUND' prefix if not exists
5. ❌ **MISSING**: Creates work unit with ID format FOUND-XXX

## Suggested Fix

The `--finalize` flag handler should:

```typescript
// After successful validation and file creation
if (finalize) {
  // ... existing validation and file creation code ...

  // Auto-create FOUND prefix if not exists
  const prefixExists = checkPrefixExists('FOUND');
  if (!prefixExists) {
    await createPrefix('FOUND', 'Foundation Event Storm tasks');
  }

  // Auto-create Foundation Event Storm work unit
  const workUnitId = await createStory(
    'FOUND',
    'Foundation Event Storm',
    {
      description: 'Conduct Foundation Event Storm to establish domain architecture, bounded contexts, aggregates, and domain events before creating individual work units',
      status: 'backlog'
    }
  );

  console.log(`✓ Created work unit ${workUnitId}: Foundation Event Storm`);
}
```

## Related Documentation

The bootstrap documentation (from `fspec bootstrap` output) extensively describes Foundation Event Storm workflow and assumes the work unit exists after finalization:

- Step 2 mentions the auto-creation
- Step 3 provides the complete Foundation Event Storm workflow
- Commands like `fspec add-foundation-bounded-context` are documented
- Workflow assumes: "Move work unit to specifying: `fspec update-work-unit-status FOUND-XXX specifying`"

## Testing Requirements

After fix, the following should work:

```bash
# Test 1: Auto-creation on first finalize
fspec discover-foundation
# ... fill fields ...
fspec discover-foundation --finalize
# Should output: ✓ Created work unit FOUND-001: Foundation Event Storm

# Test 2: Verify work unit exists
fspec list-work-units
# Should include: FOUND-001 [backlog] - Foundation Event Storm

# Test 3: Verify prefix registered
fspec create-story FOUND "Another Foundation Task"
# Should succeed (not error about missing prefix)

# Test 4: Idempotency - running finalize twice shouldn't create duplicate
# (If foundation.json already exists, should not create another work unit)
```

## Additional Notes

- This is a critical bug for the onboarding experience
- Affects all new projects using fspec
- The documentation is otherwise excellent and detailed
- Fix should be straightforward (add work unit creation to finalize logic)

## Environment

- Project: mindstrike
- fspec version: 0.9.0 (from --sync-version)
- Platform: macOS (Darwin 24.6.0)
- Node.js: (version not specified in context)

## Reporter

Discovered by Claude Code AI agent while following the documented Foundation Event Storm workflow in the mindstrike project.

Date: 2025-01-18

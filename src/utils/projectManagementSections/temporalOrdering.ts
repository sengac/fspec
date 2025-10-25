export function getTemporalOrderingSection(): string {
  return `## Temporal Ordering Enforcement (FEAT-011)

**CRITICAL**: fspec enforces temporal ordering to prevent AI agents from doing all work first, then retroactively walking through states as theater.

### The Problem

The system previously enforced **state sequence** (you must visit backlog → specifying → testing → implementing → validating → done) but not **work sequence** (you must do the work IN each state, not BEFORE entering it).

An AI agent could:
1. Write feature file, tests, and code all at once (violating ACDD)
2. Tag feature file with work unit ID
3. Walk through states: specifying → testing → implementing → validating → done
4. System would allow it because artifacts existed

This defeats ACDD's purpose: enforcing the SEQUENCE of work.

### The Solution

**Temporal validation** compares file modification timestamps against state entry timestamps:

- **Moving to \`testing\` state**: Feature files must be created/modified AFTER entering \`specifying\` state
- **Moving to \`implementing\` state**: Test files must be created/modified AFTER entering \`testing\` state

If files exist but were modified BEFORE entering the required state, the transition is blocked with a detailed error.

### How It Works

The system compares:
1. **State entry timestamp** (from \`workUnit.stateHistory\`)
2. **File modification timestamp** (from filesystem \`mtime\`)

**Example Error**:
\`\`\`bash
$ fspec update-work-unit-status AUTH-001 testing
✗ ACDD temporal ordering violation detected!

Feature files were created/modified BEFORE entering specifying state.
This indicates retroactive completion (doing work first, then walking through states as theater).

Violations:
  - spec/features/user-auth.feature
    File modified: 2025-01-15T09:00:00.000Z
    Entered specifying: 2025-01-15T10:00:00.000Z
    Gap: 60 minutes BEFORE state entry

ACDD requires work to be done IN each state, not BEFORE entering it:
  - Feature files must be created AFTER entering specifying state
  - Timestamps prove when work was actually done

To fix:
  1. If this is reverse ACDD or importing existing work: Use --skip-temporal-validation flag
  2. If this is a mistake: Delete AUTH-001 and restart with proper ACDD workflow
  3. If recovering from error: Move work unit back to specifying state and update files

For more info: See FEAT-011 "Prevent retroactive state walking"
\`\`\`

### Escape Hatch: --skip-temporal-validation

For legitimate cases (reverse ACDD, importing existing work):

\`\`\`bash
# Skip temporal validation when importing existing work
fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation
\`\`\`

**When to use \`--skip-temporal-validation\`**:
- Reverse ACDD scenarios (documenting existing code)
- Importing existing work into fspec
- Recovering from temporal validation errors
- Working with legacy code that pre-dates work unit creation

**When NOT to use**:
- Normal ACDD workflow (forward development)
- Writing new features from scratch
- Any time you can follow proper temporal ordering

### What This Prevents

✅ **AI agents cannot:**
- Create feature files before entering \`specifying\` state
- Create tests before entering \`testing\` state
- Write all code first, then walk through states as formality

✅ **The system now enforces:**
- ACDD temporal ordering (work done IN states, not BEFORE)
- Red-Green-Refactor discipline (tests written before implementation)
- Honest workflow progression (not retroactive completion)

**Note**: Tasks (work units with \`type='task'\`) are exempt from test file temporal validation since they don't require tests.`;
}

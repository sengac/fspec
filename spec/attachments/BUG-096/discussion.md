# BUG-096: Implementation Discussion

## Overview

The foundation event storm system has add commands but lacks corresponding remove commands, making it impossible to correct mistakes without manually editing JSON files.

## Commands to Implement

| Add Command | Remove Command (to create) |
|-------------|---------------------------|
| `add-foundation-bounded-context` | `remove-foundation-bounded-context` |
| `add-aggregate-to-foundation` | `remove-aggregate-from-foundation` |
| `add-domain-event-to-foundation` | `remove-domain-event-from-foundation` |
| `add-command-to-foundation` | `remove-command-from-foundation` |

## Design Decisions

### 1. Cascade Deletion for Bounded Contexts

When removing a bounded context, should we:

**Option A: Cascade delete (recommended)**
- Automatically remove all aggregates, domain events, and commands within that context
- Pros: Clean removal, no orphaned data
- Cons: Destructive, user might lose work unintentionally

**Option B: Require empty context**
- Refuse to delete if context contains any artifacts
- Pros: Safer, forces explicit cleanup
- Cons: More tedious for users

**Option C: Prompt/flag-based**
- Default to refusing, but allow `--cascade` flag
- Pros: Flexible, safe by default
- Cons: More complex implementation

**Recommendation:** Option C - refuse by default with `--cascade` flag for explicit cascade deletion.

### 2. Draft vs Finalized Foundation

Both `foundation.json` and `foundation.json.draft` should be supported:
- Check for draft first (same pattern as add commands)
- Fall back to foundation.json if no draft exists
- Regenerate FOUNDATION.md after changes

### 3. Implementation Pattern

Follow the existing add command patterns in:
- `src/commands/add-foundation-bounded-context.ts`
- `src/commands/add-aggregate-to-foundation.ts`
- `src/commands/add-domain-event-to-foundation.ts`
- `src/commands/add-command-to-foundation.ts`

Each remove command should:
1. Load foundation (draft or finalized)
2. Validate the item exists
3. Remove the item (with cascade logic if applicable)
4. Save the updated foundation
5. Regenerate FOUNDATION.md
6. Output success message

### 4. Error Cases

- Item doesn't exist → clear error message
- Bounded context not empty (without --cascade) → list contents, suggest --cascade
- Foundation file missing → error with guidance to run discover-foundation

## File Structure

```
src/commands/
├── remove-foundation-bounded-context.ts
├── remove-foundation-bounded-context-help.ts
├── remove-aggregate-from-foundation.ts
├── remove-aggregate-from-foundation-help.ts
├── remove-domain-event-from-foundation.ts
├── remove-domain-event-from-foundation-help.ts
├── remove-command-from-foundation.ts
├── remove-command-from-foundation-help.ts
└── __tests__/
    ├── remove-foundation-bounded-context.test.ts
    ├── remove-aggregate-from-foundation.test.ts
    ├── remove-domain-event-from-foundation.test.ts
    └── remove-command-from-foundation.test.ts
```

## Test Cases

1. Remove existing bounded context (empty)
2. Remove bounded context with cascade flag
3. Refuse to remove non-empty bounded context without cascade
4. Remove aggregate from specific context
5. Remove domain event from specific context
6. Remove command from specific context
7. Handle non-existent items gracefully
8. Work with draft file
9. Work with finalized foundation.json
10. Regenerate FOUNDATION.md after removal

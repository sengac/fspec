# AST Research: wrapInSystemReminder() Callers

## Summary

The `wrapInSystemReminder()` function is called from **24 unique locations** across the fspec codebase. All calls currently pass only content (no scope parameter).

## Call Sites in system-reminder.ts (18 internal calls)

These are all the helper functions within `src/utils/system-reminder.ts` that wrap content:

| Line | Function | Proposed Scope |
|------|----------|---------------|
| 253 | getStatusChangeReminder (specifying inline) | `workflow-guardrail` |
| 286 | getMissingEstimateReminder | `estimation` |
| 306 | getEmptyBacklogReminder | `workflow-guardrail` |
| 355 | getFileNamingReminder | `workflow-guardrail` |
| 381 | getUnregisteredTagReminder | `workflow-guardrail` |
| 417 | getMissingRequiredTagsReminder | `workflow-guardrail` |
| 445 | getUnansweredQuestionsReminder | `workflow-guardrail` |
| 473 | getEmptyExampleMappingReminder | `workflow-guardrail` |
| 500 | getPostGenerationReminder | `workflow-guardrail` |
| 604 | specifyingStateReminder | `workflow-guardrail` |
| 667 | implementingStateReminder | `workflow-guardrail` |
| 708 | doneStateReminder | `workflow-guardrail` |
| 747 | getLongDurationReminder | `workflow-guardrail` |
| 837 | getLargeEstimateReminder | `estimation` |
| 886 | getVirtualHooksReminder | `workflow-guardrail` |
| 923 | getVirtualHooksCleanupReminder | `workflow-guardrail` |
| 1030 | getBackwardTransitionCleanupReminder | `workflow-guardrail` |
| 1250 | workUnitCreatedReminder | `tool-output` |

## External Callers (6 files)

| File | Line | Proposed Scope |
|------|------|---------------|
| `commands/discover-event-storm.ts` | 68 | `tool-output` |
| `commands/discover-foundation.ts` | 176, 219, 496, 534 | `tool-output` |
| `commands/update-work-unit-status.ts` | 712, 819 | `workflow-guardrail` |
| `commands/add-architecture-note.ts` | 72 | `tool-output` |
| `commands/hooks/workUnitStatusHook.ts` | 78 | `workflow-guardrail` |
| `commands/add-example.ts` | 76 | `tool-output` |
| `commands/bootstrap.ts` | 186, 215 | `tool-output` |

## Template Generators (separate from wrapInSystemReminder)

These files emit `<system-reminder>` blocks directly in template strings:

- `src/utils/templateGenerator.ts` - Generates CLAUDE.md and agent templates
- `src/utils/projectManagementTemplate.ts` - Bootstrap output
- `src/utils/slashCommandSections/*.ts` - Various section generators
- `src/utils/agentRuntimeConfig.ts` - Agent configuration templates

These need `<!-- type:scope -->` markers added to their template strings directly.

## Migration Impact

- **24 direct callers** of `wrapInSystemReminder()` need scope parameter added
- **~10 template files** need `<!-- type:scope -->` markers in template strings
- **2 test files** need updating (`system-reminder.test.ts`, others)
- Backward compatible: scope parameter is optional, old callers continue working

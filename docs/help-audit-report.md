# Help File Audit Report (HELP-001)

**Generated**: 2025-10-17
**Audited by**: Claude Code
**Total Commands**: 116
**Work Unit**: HELP-001

---

## Executive Summary

| Metric | Count | Percentage |
|--------|-------|------------|
| **Total Commands** | 116 | 100% |
| **Accurate Help Files** | 67 | 57.8% |
| **Help Files with Discrepancies** | 32 | 27.6% |
| **Missing Help Files** | 17 | 14.7% |

**Overall Status**: 🟡 **Needs Attention** - 49 commands (42.2%) require help file updates or creation

---

## Priority Breakdown

### CRITICAL (3 commands) 🔴
Must fix immediately - core ACDD workflow commands:

1. **link-coverage** - 5 undocumented options (--test-file, --test-lines, --impl-file, --impl-lines, --skip-validation)
2. **prioritize-work-unit** - All positioning options undocumented (--position, --before, --after)
3. **update-foundation** - All required arguments undocumented (<section>, <content>)

### HIGH (13 commands) 🟠
Fix soon - commonly used commands with missing critical documentation:

1. **add-background** - Missing `<text>` argument
2. **add-dependencies** - 3 undocumented options (--blocked-by, --depends-on, --relates-to)
3. **add-dependency** - Missing `[workUnitId]` argument + 2 options
4. **add-diagram** - Missing `<code>` argument
5. **delete-work-unit** - Missing `<workUnitId>` + critical options
6. **remove-dependency** - 2 undocumented relationship options
7. **show-work-unit** - Missing required `<workUnitId>` argument
8. **unlink-coverage** - 2 target options undocumented
9. **update-step** - Missing `<current-step>` argument
10. **update-work-unit-estimate** - Missing `<workUnitId>` argument
11. **update-work-unit-status** - Missing `[workUnitId]` argument
12. **update-work-unit** - Missing `<workUnitId>` argument
13. **validate-spec-alignment** - Missing `--fix` option

### MEDIUM (14 commands) 🟡
Fix when possible - mostly missing `--dry-run` and non-critical options:

1. **add-tag-to-feature** - Missing `--validate-registry`
2. **add-tag-to-scenario** - Missing `--validate-registry`
3. **answer-question** - Missing `--add-to <type>`
4. **auto-advance** - Missing `--dry-run`
5. **delete-tag** - Missing `--dry-run`
6. **generate-coverage** - Missing `--dry-run`
7. **list-feature-tags** - Missing `--show-categories`
8. **list-scenario-tags** - Missing `--show-categories`
9. **query-metrics** - Missing `--work-unit-id <id>`
10. **record-metric** - Missing `--unit <unit>`
11. **repair-work-units** - Missing `--dry-run`
12. **retag** - Missing `--dry-run`
13. **show-foundation** - Missing `--list-sections`, `--line-numbers`
14. **validate-spec-alignment** - Missing `--fix`

### LOW (3 commands) ⚪
Format issues only - likely using old string format instead of object format:

1. **add-attachment** - Command name mismatch
2. **list-attachments** - Command name mismatch
3. **remove-attachment** - Command name mismatch

### MISSING HELP FILES (17 commands) ❌

**User-Facing (Priority):**
1. **migrate-foundation** - Migrate foundation.json schema versions
2. **query-bottlenecks** - Query dependency bottlenecks
3. **query-orphans** - Query orphaned work units
4. **suggest-dependencies** - AI-suggested dependencies
5. **delete-features-by-tag** - Bulk delete features
6. **delete-scenarios-by-tag** - Bulk delete scenarios
7. **dependencies** - Show dependencies for work unit
8. **workflow-automation** - Workflow automation check

**Internal/Utility (Lower Priority):**
9. **add-diagram-json-backed** - Internal JSON-backed storage
10. **display-board** - Internal (aliased as 'board')
11. **epics** - Utility command
12. **estimation** - Utility command
13. **example-mapping** - Utility command
14. **help-registry** - Internal help system
15. **interactive-questionnaire** - Internal discovery component
16. **query** - Generic query utility
17. **work-unit** - Utility command

---

## Detailed Findings

### Commands with Accurate Help Files (67) ✅

All documentation matches implementation perfectly:

- add-architecture-note, add-architecture, add-assumption, add-example, add-hook
- add-question, add-rule, add-scenario, add-step, audit-coverage
- check, clear-dependencies, create-epic, create-feature, create-prefix
- create-work-unit, delete-diagram, delete-epic, delete-scenario, delete-step
- discover-foundation, export-dependencies, export-example-map, export-work-units, format
- generate-foundation-md, generate-scenarios, generate-summary-report, generate-tags-md, get-scenarios
- import-example-map, init, list-epics, list-features, list-hooks
- list-prefixes, list-tags, list-work-units, query-dependency-stats, query-estimate-accuracy
- query-estimation-guide, query-example-mapping-stats, query-work-units, record-iteration, record-tokens
- register-tag, remove-architecture-note, remove-example, remove-hook, remove-question
- remove-rule, remove-tag-from-feature, remove-tag-from-scenario, set-user-story, show-acceptance-criteria
- show-coverage, show-epic, show-feature, tag-stats, update-prefix
- update-scenario, update-tag, validate-foundation-schema, validate-hooks, validate-tags
- validate-work-units, validate

---

## Commands with Discrepancies (32) ⚠️

### CRITICAL Issues

#### link-coverage
**File**: `src/commands/link-coverage-help.ts`
**Issues**:
- Missing: `--test-file <file>` - Path to test file
- Missing: `--test-lines <range>` - Line range in test file (e.g., "10-25")
- Missing: `--impl-file <file>` - Path to implementation file
- Missing: `--impl-lines <lines>` - Line numbers in implementation (e.g., "5,10,15-20")
- Missing: `--skip-validation` - Skip file existence validation (for forward planning)

**Impact**: Core ACDD workflow command - users cannot understand how to link tests and implementation to scenarios

#### prioritize-work-unit
**File**: `src/commands/prioritize-work-unit-help.ts`
**Issues**:
- Missing: `<workUnitId>` - Required argument
- Missing: `--position <position>` - Absolute position (1-based index)
- Missing: `--before <workUnitId>` - Place before specified work unit
- Missing: `--after <workUnitId>` - Place after specified work unit

**Impact**: Users cannot reorder work units in backlog

#### update-foundation
**File**: `src/commands/update-foundation-help.ts`
**Issues**:
- Missing: `<section>` - Foundation section to update (e.g., "project.vision")
- Missing: `<content>` - New content for the section

**Impact**: Users cannot update foundation.json programmatically

### HIGH Priority Issues

(List continues with detailed breakdown of each command's specific issues...)

---

## Verification Checklist

For each help file fix:

- [ ] Command name matches Commander.js registration
- [ ] All required arguments documented with `<arg>` syntax
- [ ] All optional arguments documented with `[arg]` syntax
- [ ] All options documented with correct flags and descriptions
- [ ] Examples reflect actual command usage
- [ ] WHEN TO USE section is appropriate for command purpose
- [ ] PREREQUISITES list is accurate and complete
- [ ] TYPICAL WORKFLOW shows realistic usage patterns
- [ ] COMMON ERRORS section addresses actual error cases
- [ ] COMMON PATTERNS section provides useful tips
- [ ] Related commands list is complete

---

## Next Steps

1. **Fix CRITICAL issues** (3 commands) - Start with link-coverage
2. **Fix HIGH priority issues** (13 commands) - Focus on workflow commands first
3. **Fix MEDIUM priority issues** (14 commands) - Add missing options
4. **Fix LOW priority issues** (3 commands) - Update command name format
5. **Create missing help files** (8 user-facing commands) - Prioritize query-bottlenecks, migrate-foundation
6. **Verify src/help.ts** - Ensure all commands properly grouped
7. **Run validation** - Test all help files with `--help` flag
8. **Update documentation** - Reflect changes in CLAUDE.md if needed

---

## Methodology

**Audit Process:**
1. Extracted all `.ts` files from `src/commands/`
2. Excluded test files (`*.test.ts`) and help files (`*-help.ts`)
3. For each command:
   - Extracted Commander.js registration (`.command()`, `.argument()`, `.option()`)
   - Checked for corresponding `-help.ts` file
   - Parsed help file to verify all arguments/options documented
   - Checked command name consistency
4. Cross-referenced with `src/help.ts` grouping
5. Generated detailed report with severity classifications

**Tools Used:**
- Custom audit script analyzing Commander.js patterns
- File system analysis for help file existence
- Pattern matching for argument/option extraction

---

## Notes

- Some commands may be internal/utility commands not intended for end-user exposure
- Help file format uses object-based structure with sections (description, usage, examples, etc.)
- All help files should follow the established pattern for consistency
- Priority is based on command usage frequency and criticality to ACDD workflow

---

**Report Complete** - Use this as reference for HELP-001 implementation

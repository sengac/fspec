export function getKanbanWorkflowSection(): string {
  return `## Step 3: Kanban Workflow - How to Track Work

### View the Board

\`\`\`bash
fspec board                           # See current Kanban state
fspec list-work-units --status=backlog # View backlog
fspec show-work-unit EXAMPLE-006           # See work unit details
\`\`\`

### Move Work Through the Kanban

**CRITICAL**: As you work, you MUST move work units through Kanban states AND update feature file tags:

\`\`\`bash
# 1. SELECT from backlog
fspec update-work-unit-status EXAMPLE-006 specifying

# 2. SPECIFY with Gherkin
fspec create-feature "Feature Name"
fspec add-scenario feature-name "Scenario"
fspec add-tag-to-feature spec/features/feature-name.feature @wip
fspec update-work-unit-status EXAMPLE-006 testing

# 3. TEST FIRST (write failing tests)
# Write tests in src/__tests__/*.test.ts
fspec update-work-unit-status EXAMPLE-006 implementing

# 4. IMPLEMENT (make tests pass)
# Write minimal code to pass tests
fspec update-work-unit-status EXAMPLE-006 validating

# 5. VALIDATE (quality checks)
<quality-check-commands>
example-project validate
example-project validate-tags
fspec update-work-unit-status EXAMPLE-006 done

# 6. COMPLETE (update tags)
fspec remove-tag-from-feature spec/features/feature-name.feature @wip
fspec add-tag-to-feature spec/features/feature-name.feature @done
\`\`\`

### Moving Backward Through Kanban (Fixing Mistakes)

**CRITICAL**: You CAN and SHOULD move work units backward when you discover mistakes or gaps, rather than creating new work units!

**When to Move Backward:**

✅ **Move backward to previous state when:**
- **testing → specifying**: Tests revealed incomplete/wrong acceptance criteria
- **implementing → testing**: Need to add/fix test cases
- **implementing → specifying**: Discovered missing scenarios
- **validating → implementing**: Quality checks failed, need more code
- **validating → testing**: Tests are inadequate
- **any state → specifying**: Fundamental misunderstanding of requirements

**How to Move Backward:**

\`\`\`bash
# Realized specs are incomplete while writing tests
fspec update-work-unit-status EXAMPLE-006 specifying

# Quality checks failed, need to fix implementation
fspec update-work-unit-status EXAMPLE-006 implementing

# Tests need refactoring based on implementation learnings
fspec update-work-unit-status EXAMPLE-006 testing
\`\`\`

**Why Move Backward (Not Create New Work Units):**

✅ **DO** move backward for:
- Incomplete specifications discovered during testing
- Missing test coverage discovered during implementation
- Gaps in acceptance criteria revealed by validation
- Mistakes or misunderstandings in current work

❌ **DON'T** create new work units for:
- Fixing mistakes in current work
- Refining existing specs/tests/code
- Correcting errors in the same feature

**Only Create New Work Units For:**
- Genuinely new features (out of scope)
- Bugs in already-completed work (marked \`done\`)
- Technical debt to track separately

**Remember**: Backward movement is NORMAL and ENCOURAGED. It's better to move backward and fix issues than to create unnecessary work unit fragmentation.

### Tag Management Throughout Development

**Feature file tags reflect current state:**
- \`@wip\` - Work in progress (add when starting, remove when done)
- \`@done\` - Completed and validated
- \`@blocked\` - Cannot proceed (add blocker reason to work unit)
- \`@critical\` - High priority
- \`@critical\`, \`@high\` - Release phase

**Update tags as you progress:**
\`\`\`bash
# Starting work
fspec add-tag-to-feature spec/features/example-login.feature @wip

# Completing work
fspec remove-tag-from-feature spec/features/example-login.feature @wip
fspec add-tag-to-feature spec/features/example-login.feature @done
\`\`\`

### Stable Indices and Soft-Delete Pattern

**Items within work units use stable IDs that never change:**

Work units contain items (business rules, examples, questions, architecture notes) that are assigned **stable indices** when created. These IDs:
- Auto-increment from 0 (nextRuleId, nextExampleId, nextQuestionId, nextArchitectureNoteId)
- Never shift when other items are removed
- Persist even after soft-delete

**Soft-Delete Pattern:**
- Items are marked \`deleted: true\` instead of being removed from arrays
- \`deletedAt\` timestamp records when item was deleted
- IDs remain stable across all operations
- Use \`show-deleted\` to view soft-deleted items

**Restore Commands:**
\`\`\`bash
# View deleted items with their stable IDs
fspec show-deleted AUTH-001

# Restore individual items by stable ID
fspec restore-rule AUTH-001 2
fspec restore-example AUTH-001 5
fspec restore-question AUTH-001 3
fspec restore-architecture-note AUTH-001 1

# Bulk restore with --ids flag
fspec restore-rule AUTH-001 --ids 2,5,7
fspec restore-example AUTH-001 --ids 1,3
\`\`\`

**Compaction:**
\`\`\`bash
# Permanently remove soft-deleted items (destructive!)
fspec compact-work-unit AUTH-001

# Force compact during non-done status (use with caution)
fspec compact-work-unit AUTH-001 --force
\`\`\`

**Auto-Compact Behavior:**
- When moving work unit to \`done\` status, auto-compact triggers automatically
- Permanently removes all soft-deleted items
- Renumbers remaining items sequentially (0, 1, 2, ...)
- Resets nextId counters to match remaining count
- Cannot be undone - use \`show-deleted\` before moving to done!

### If Blocked

\`\`\`bash
# Mark work unit as blocked with reason
fspec update-work-unit-status EXAMPLE-006 blocked
fspec add-tag-to-feature spec/features/feature-name.feature @blocked
# Add note to work unit about why it's blocked
\`\`\`

`;
}

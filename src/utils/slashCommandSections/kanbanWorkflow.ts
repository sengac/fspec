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
npm run check
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

### If Blocked

\`\`\`bash
# Mark work unit as blocked with reason
fspec update-work-unit-status EXAMPLE-006 blocked
fspec add-tag-to-feature spec/features/feature-name.feature @blocked
# Add note to work unit about why it's blocked
\`\`\`

`;
}

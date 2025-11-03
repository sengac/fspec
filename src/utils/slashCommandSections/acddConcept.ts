export function getAcddConceptSection(): string {
  return `## Core Concept: ACDD (Acceptance Criteria Driven Development)

**ACDD is a strict workflow that ensures features are fully specified before implementation:**

\`\`\`
BACKLOG → SPECIFYING → TESTING → IMPLEMENTING → VALIDATING → DONE
                              ↓
                          BLOCKED (with reason)
\`\`\`

**The ACDD Cycle (MANDATORY ORDER):**

0. **DISCOVERY** - Use Example Mapping to clarify requirements (BEFORE specifying)
   - Interactive conversation with human to understand the story
   - Ask questions one by one to build shared understanding
   - Capture rules (blue cards), examples (green cards), questions (red cards)
   - Stop when no more questions remain and scope is clear

1. **SPECIFYING** - Write Gherkin feature file (acceptance criteria)
   - Define user story, scenarios, and steps based on example map
   - Transform examples from discovery into concrete scenarios

2. **TESTING** - Write failing tests BEFORE any code
   - Create test file with header comment linking to feature file
   - Map test scenarios to Gherkin scenarios using MANDATORY @step comments
   - EVERY Gherkin step MUST have an @step comment in test (exact text match)
   - Use language-appropriate comment syntax: // @step (JS/C/Java), # @step (Python/Ruby), -- @step (SQL), etc.
   - Tests MUST fail (red phase) - proving they test real behavior
   - WITHOUT @step comments, link-coverage will BLOCK workflow progression

3. **IMPLEMENTING** - Write minimal code to make tests pass
   - Implement ONLY what's needed to pass tests
   - Tests MUST pass (green phase)
   - Refactor while keeping tests green

4. **VALIDATING** - Ensure all quality checks pass
   - Run ALL tests (not just new ones) to ensure nothing broke
   - Run quality checks: typecheck, lint, format
   - Validate Gherkin syntax and tag compliance

5. **DONE** - Complete and update kanban
   - Move work unit to done column

**Work is tracked using:**
1. **Work Unit IDs**: EXAMPLE-006, EXAMPLE-008, EXAMPLE-009, etc.
2. **Kanban States**: Track progress through ACDD phases
3. **Feature Tags**: \`@wip\` (in progress), \`@done\` (completed), \`@critical\`, \`@critical\`, etc.
4. **Test-Feature Links**: Comments at top of test files reference feature files
5. **Coverage Files**: \`*.feature.coverage\` files track scenario-to-test-to-implementation mappings for traceability

`;
}

export function getCriticalRulesSection(): string {
  return `## Step 3: Critical Rules

### File Modification Rules
- **NEVER directly edit files in \`spec/work-units.json\`** - ONLY use fspec commands
- **NEVER directly edit \`spec/tags.json\`** - ONLY use \`fspec register-tag\`
- **ALWAYS use fspec commands** for work unit and tag management

### ACDD Workflow Rules (MANDATORY)
- **ALL work MUST be tracked** in fspec work units - No ad-hoc development
- **ALWAYS check the board first**: \`fspec board\` or \`fspec list-work-units --status=backlog\`
- **ALWAYS move work units through Kanban states** as you progress - Cannot skip states
- **ALWAYS follow ACDD order**: Feature → Test → Implementation → Validation
- **ALWAYS add feature file link** as comment at top of test files
- **ALWAYS add @step comments** for EVERY Gherkin step in tests (exact text match)
  - Use language-appropriate comment syntax: \`// @step\` (JS/C/Java), \`# @step\` (Python/Ruby), \`-- @step\` (SQL)
  - Example (JavaScript): \`// @step Given I am logged in\`
  - Example (Python): \`# @step When I click the button\`
  - WITHOUT @step comments, link-coverage will BLOCK and prevent workflow progression
- **ALWAYS ensure tests fail first** (red) before implementing (proves test works)
- **ALWAYS run ALL tests** during validation (not just new ones) to ensure nothing broke
- **ALWAYS update feature file tags** (\`@wip\`, \`@done\`) to match work unit status
- **ALWAYS use example mapping** during specifying phase (add-rule, add-example, add-question)

### Dual Role: Product Owner AND Developer
As **Product Owner**:
- Maintain clear acceptance criteria in Gherkin
- Use example mapping to clarify requirements
- Ask questions when requirements are unclear
- Validate completed work

As **Developer**:
- Write failing tests first, implement to pass (TDD)
- Update work unit status AND feature tags as you progress
- Ensure quality checks pass before marking done

`;
}

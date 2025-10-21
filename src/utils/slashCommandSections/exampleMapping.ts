export function getExampleMappingSection(): string {
  return `## Step 2: Example Mapping - Discovery BEFORE Specification

**CRITICAL**: Before writing any Gherkin feature file, you MUST do Example Mapping to clarify requirements through conversation.

### What is Example Mapping?

Example Mapping is a collaborative conversation technique using four types of "cards":
- 🟨 **Yellow Card (Story)**: The user story being discussed
- 🟦 **Blue Cards (Rules)**: Business rules and acceptance criteria
- 🟩 **Green Cards (Examples)**: Concrete examples that illustrate the rules
- 🟥 **Red Cards (Questions)**: Uncertainties that need answers

### How Example Mapping Works in fspec

When you move a work unit to \`specifying\` status, you MUST do Example Mapping FIRST:

\`\`\`bash
fspec show-work-unit EXAMPLE-006           # Start with the user story (yellow card)
fspec update-work-unit-status EXAMPLE-006 specifying
\`\`\`

**Now begin the interactive conversation with the human:**

#### Step 0: Capture User Story (Yellow Card)

First, capture the user story fields to avoid placeholder text in generated scenarios:

\`\`\`bash
fspec set-user-story EXAMPLE-006 \\
  --role "developer using fspec" \\
  --action "validate feature files automatically" \\
  --benefit "I catch syntax errors before committing"
\`\`\`

**CRITICAL**: Setting the user story BEFORE generating scenarios ensures the Background section is complete without \`[role]\`, \`[action]\`, \`[benefit]\` placeholders.

#### Step 1: Ask About Rules (Blue Cards)

Ask the human to identify the business rules governing this feature:

\`\`\`
You: "What are the key business rules for [feature name]?"
You: "Are there any constraints or policies that govern this behavior?"
You: "What conditions must be met for this feature to work?"
\`\`\`

Capture each rule in fspec:
\`\`\`bash
fspec add-rule EXAMPLE-006 "Feature validation must complete within 2 seconds"
fspec add-rule EXAMPLE-006 "Feature files must use .feature extension"
fspec add-rule EXAMPLE-006 "Only valid Gherkin syntax is accepted"
\`\`\`

#### Step 2: Ask About Examples (Green Cards)

For each rule, ask for concrete examples:

\`\`\`
You: "Can you give me a concrete example of when this rule applies?"
You: "What would happen in the case where [specific scenario]?"
You: "How should the system behave when [edge case]?"
\`\`\`

Capture each example in fspec:
\`\`\`bash
fspec add-example EXAMPLE-006 "User runs 'example-project validate' with no args, validates all feature files"
fspec add-example EXAMPLE-006 "User runs 'example-project validate spec/features/test.feature', validates single file"
fspec add-example EXAMPLE-006 "User runs 'example-project validate' on invalid syntax, gets error message with line number"
\`\`\`

#### Step 3: Ask Questions (Red Cards)

When you encounter uncertainties, ask the human directly:

\`\`\`bash
fspec add-question EXAMPLE-006 "@human: Should we allow custom port ranges in config file?"
fspec add-question EXAMPLE-006 "@human: What happens if the specified port is already in use?"
fspec add-question EXAMPLE-006 "@human: Should we support IPv6 addresses?"
\`\`\`

**Then wait for the human to answer each question.** Once answered:
\`\`\`bash
fspec answer-question EXAMPLE-006 0 --answer "Yes, config file should support portRange: [min, max]"
fspec answer-question EXAMPLE-006 1 --answer "Try next available port and log a warning"
fspec answer-question EXAMPLE-006 2 --answer "Not in Phase 1, add to backlog as EXAMPLE-006"
\`\`\`

#### Step 4: Iterate Until No Red Cards Remain

Continue the conversation:
- Ask follow-up questions as new uncertainties emerge
- Clarify rules based on answers
- Add more examples to illustrate edge cases
- Stop when you have clear understanding (aim for ~25 minutes per story)

#### Step 5: Check for Consensus

Ask the human:
\`\`\`
You: "Do we have a shared understanding of this feature now?"
You: "Are there any remaining questions or uncertainties?"
You: "Is the scope clear enough to write acceptance criteria?"
\`\`\`

### When to Stop Example Mapping

Stop when:
1. ✅ No red (question) cards remain unanswered
2. ✅ You have enough examples to understand all rules
3. ✅ The scope feels clear and bounded
4. ✅ Human confirms shared understanding

If too many red cards remain or scope is unclear:
\`\`\`bash
fspec update-work-unit-status EXAMPLE-006 blocked
# Add blocker reason explaining what needs clarification
# Return to backlog until questions can be answered
\`\`\`

### Transform Example Map to Gherkin

Once Example Mapping is complete, you have TWO options:

**Option 1: Automatic Generation (Recommended)**

fspec can automatically convert your example map to a Gherkin feature file:

\`\`\`bash
# Defaults to work unit title as feature file name (capability-based naming)
fspec generate-scenarios EXAMPLE-006

# Or specify custom feature file name
fspec generate-scenarios EXAMPLE-006 --feature=user-authentication
\`\`\`

This command:
- Reads rules, examples, and answered questions from the work unit
- Generates a feature file with scenarios based on your examples
- **Names feature file after work unit title** (e.g., "User Authentication" → \`example-feature.feature\`)
- Use \`--feature\` flag to override the default name
- Transforms rules into background context or scenario preconditions
- Creates properly structured Given-When-Then steps
- **NEVER names files after work unit IDs** (e.g., ❌ \`example-006.feature\`)

**Option 2: Manual Creation**

Or manually write the Gherkin feature file using the example map as a guide:
- Rules (blue cards) → Background description or scenario context
- Examples (green cards) → Concrete scenarios with Given-When-Then
- Answered questions → Inform scenario details and edge cases

\`\`\`bash
fspec create-feature "Feature File Validation"
fspec add-scenario feature-file-validation "Validate feature file with valid syntax"
fspec add-scenario feature-file-validation "Validate feature file with invalid syntax"
fspec add-scenario feature-file-validation "Validate all feature files in directory"
\`\`\`

**Pro tip**: Use automatic generation first, then refine the generated scenarios manually if needed.

### CRITICAL: Feature File and Test File Naming

**ALWAYS name files using "WHAT IS" (the capability), NOT "what the current state is"!**

✅ **CORRECT Naming (What IS - the capability):**
- Feature: \`system-reminder-anti-drift-pattern.feature\` (describes WHAT the feature IS)
- Test: \`system-reminder.test.ts\` (tests the system-reminder capability)
- Code: \`system-reminder.ts\` (implements the capability)

❌ **WRONG Naming (current state):**
- Feature: \`implement-system-reminder-pattern.feature\` (this describes the TASK, not the capability)
- Feature: \`add-system-reminders.feature\` (this describes the CHANGE, not the capability)
- Test: \`remind-001.test.ts\` (this describes the WORK UNIT, not the capability)

**Why This Matters:**
- Feature files are **living documentation** of capabilities
- They should describe what the system CAN DO, not what we're doing to it
- The file name should make sense after the feature is built
- "Implement X" only makes sense DURING development, not AFTER

**Examples:**
- ✅ \`example-feature.feature\` - describes the capability
- ❌ \`add-user-example-login.feature\` - describes the task
- ✅ \`example-validation.feature\` - describes the capability
- ❌ \`implement-gherkin-validator.feature\` - describes the task
- ✅ \`dependency-graph-visualization.feature\` - describes the capability
- ❌ \`create-dependency-graph.feature\` - describes the task

**Test and Code Files Follow the Same Rule:**
- Test file: \`user-authentication.test.ts\` (tests the authentication capability)
- Code file: \`user-authentication.ts\` (implements the authentication capability)

### fspec Commands for Example Mapping

\`\`\`bash
# Rules (blue cards)
fspec add-rule <work-unit-id> "Rule text"
fspec remove-rule <work-unit-id> <index>

# Examples (green cards)
fspec add-example <work-unit-id> "Example text"
fspec remove-example <work-unit-id> <index>

# Questions (red cards)
fspec add-question <work-unit-id> "@human: Question text?"
fspec answer-question <work-unit-id> <index> --answer "Answer text" --add-to rule|assumption|none
fspec remove-question <work-unit-id> <index>

# Attachments (supporting files)
fspec add-attachment <work-unit-id> <file-path>
fspec add-attachment <work-unit-id> <file-path> --description "Description"
fspec list-attachments <work-unit-id>
fspec remove-attachment <work-unit-id> <file-name>
fspec remove-attachment <work-unit-id> <file-name> --keep-file

# View the example map
fspec show-work-unit <work-unit-id>
\`\`\`

### Why Example Mapping Matters

- **Prevents surprises**: Uncovers hidden complexity BEFORE coding
- **Shared understanding**: Ensures human and AI are aligned
- **Right-sized stories**: Prevents oversized work units
- **Living documentation**: Rules and examples captured in fspec
- **Better scenarios**: Examples naturally become Gherkin scenarios

**Reference**: [Example Mapping Introduction](https://cucumber.io/blog/bdd/example-mapping-introduction/)

`;
}

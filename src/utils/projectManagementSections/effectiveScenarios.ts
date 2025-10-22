export function getEffectiveScenariosSection(): string {
  return `## Writing Effective Scenarios

### Good Scenario Examples

✅ **GOOD - Specific and Testable**:
\`\`\`gherkin
Scenario: Create feature file with default template
  Given I am in a project with a spec/features/ directory
  When I run \`fspec create-feature "User Authentication"\`
  Then a file "spec/features/user-authentication.feature" should be created
  And the file should contain a valid Gherkin feature structure
  And the file should include a Background section placeholder
  And the file should include a Scenario placeholder
\`\`\`

✅ **GOOD - Clear Preconditions and Outcomes**:
\`\`\`gherkin
Scenario: Validate Gherkin syntax and report errors
  Given I have a feature file "spec/features/login.feature" with invalid syntax
  When I run \`fspec validate spec/features/login.feature\`
  Then the command should exit with code 1
  And the output should contain the line number of the syntax error
  And the output should contain a helpful error message
  And the output should suggest how to fix the error
\`\`\`

✅ **GOOD - Data Tables for Multiple Cases**:
\`\`\`gherkin
Scenario Outline: Validate tag format
  Given I have a feature file with tag "<tag>"
  When I run \`fspec validate-tags\`
  Then the validation should <result>

  Examples:
    | tag              | result |
    | @phase1          | pass   |
    | @Phase1          | fail   |
    | @phase-1         | fail   |
    | phase1           | fail   |
    | @my-custom-tag   | pass   |
\`\`\`

### Bad Scenario Examples

❌ **Avoid**: Vague steps ("system works"), implementation details (@cucumber/gherkin-parser), missing assertions ("file is created"). ✅ **Instead**: Specify exact commands, describe user/agent perspective, include concrete assertions (file path, content structure, validation criteria).`;
}
